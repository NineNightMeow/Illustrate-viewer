use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    database::{ImageMetadata, MetadataDatabase},
    metadata::{extract_image_metadata, MetadataExtractionError},
};

const METADATA_WORKER_COUNT: usize = 1;

#[derive(Clone)]
pub struct MetadataTaskQueue(Arc<QueueInner>);

struct QueueInner {
    state: Mutex<QueueState>,
    ready: Condvar,
}

struct QueueState {
    tasks: HashMap<String, MetadataTask>,
    pending: VecDeque<String>,
    workers_started: bool,
    shutting_down: bool,
}

struct MetadataTask {
    status: MetadataTaskStatus,
    cancellation: Arc<AtomicBool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataTaskStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataQueueStatus {
    pub total: usize,
    pub idle: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataQueueSubmission {
    pub queue_status: MetadataQueueStatus,
    pub accepted_image_ids: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetadataTaskUpdate {
    image_id: String,
    status: MetadataTaskStatus,
    metadata: Option<ImageMetadata>,
    error_code: Option<&'static str>,
}

impl Default for MetadataTaskQueue {
    fn default() -> Self {
        Self(Arc::new(QueueInner {
            state: Mutex::new(QueueState {
                tasks: HashMap::new(),
                pending: VecDeque::new(),
                workers_started: false,
                shutting_down: false,
            }),
            ready: Condvar::new(),
        }))
    }
}

impl MetadataTaskQueue {
    pub fn start_workers(
        &self,
        app: AppHandle,
        database: MetadataDatabase,
    ) -> Result<(), MetadataExtractionError> {
        let mut state = self.lock_state()?;
        if state.workers_started {
            return Ok(());
        }
        state.workers_started = true;
        drop(state);
        for worker_index in 0..METADATA_WORKER_COUNT {
            let queue = self.clone();
            let app = app.clone();
            let database = database.clone();
            thread::Builder::new()
                .name(format!("metadata-worker-{}", worker_index + 1))
                .spawn(move || queue.run_worker(app, database))
                .map_err(|error| MetadataExtractionError::queue(error.to_string()))?;
        }
        Ok(())
    }

    pub fn enqueue(
        &self,
        image_ids: Vec<String>,
    ) -> Result<MetadataQueueSubmission, MetadataExtractionError> {
        let mut state = self.lock_state()?;
        if state.shutting_down {
            return Err(MetadataExtractionError::queue(
                "The metadata queue is shutting down.",
            ));
        }
        let mut accepted = Vec::new();
        let mut seen = HashSet::new();
        for image_id in image_ids {
            if image_id.trim().is_empty() || !seen.insert(image_id.clone()) {
                continue;
            }
            match state.tasks.get(&image_id).map(|task| task.status) {
                Some(MetadataTaskStatus::Idle | MetadataTaskStatus::Running) => {}
                _ => {
                    state.tasks.insert(
                        image_id.clone(),
                        MetadataTask {
                            status: MetadataTaskStatus::Idle,
                            cancellation: Arc::new(AtomicBool::new(false)),
                        },
                    );
                    state.pending.push_back(image_id.clone());
                }
            }
            accepted.push(image_id);
        }
        let submission = MetadataQueueSubmission {
            queue_status: status(&state),
            accepted_image_ids: accepted,
        };
        self.0.ready.notify_all();
        Ok(submission)
    }

    pub fn retry(
        &self,
        image_id: String,
    ) -> Result<MetadataQueueSubmission, MetadataExtractionError> {
        self.enqueue(vec![image_id])
    }

    pub fn cancel(
        &self,
        image_id: Option<&str>,
    ) -> Result<MetadataQueueStatus, MetadataExtractionError> {
        let mut state = self.lock_state()?;
        let ids = state
            .tasks
            .iter()
            .filter(|(id, task)| {
                image_id.map(|target| target == id.as_str()).unwrap_or(true)
                    && matches!(
                        task.status,
                        MetadataTaskStatus::Idle | MetadataTaskStatus::Running
                    )
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            if let Some(task) = state.tasks.get_mut(&id) {
                task.cancellation.store(true, Ordering::Release);
                task.status = MetadataTaskStatus::Cancelled;
            }
        }
        let result = status(&state);
        self.0.ready.notify_all();
        Ok(result)
    }

    pub fn status(&self) -> Result<MetadataQueueStatus, MetadataExtractionError> {
        let state = self.lock_state()?;
        Ok(status(&state))
    }

    pub fn shutdown(&self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.shutting_down = true;
            for task in state.tasks.values_mut() {
                if matches!(
                    task.status,
                    MetadataTaskStatus::Idle | MetadataTaskStatus::Running
                ) {
                    task.cancellation.store(true, Ordering::Release);
                    task.status = MetadataTaskStatus::Cancelled;
                }
            }
            self.0.ready.notify_all();
        }
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, QueueState>, MetadataExtractionError> {
        self.0
            .state
            .lock()
            .map_err(|_| MetadataExtractionError::queue("The metadata queue is unavailable."))
    }

    fn run_worker(&self, app: AppHandle, database: MetadataDatabase) {
        while let Some((image_id, cancellation)) = self.next_task() {
            emit_update(&app, &image_id, MetadataTaskStatus::Running, None, None);
            let result = extract_image_metadata(&app, &database, &image_id, &cancellation);
            let (task_status, metadata, error_code) = match result {
                Ok(metadata) => (MetadataTaskStatus::Completed, Some(metadata), None),
                Err(error) if error.code == "cancelled" => {
                    (MetadataTaskStatus::Cancelled, None, Some(error.code))
                }
                Err(error) => (MetadataTaskStatus::Failed, None, Some(error.code)),
            };
            self.finish_task(&image_id, task_status);
            emit_update(&app, &image_id, task_status, metadata, error_code);
        }
    }

    fn next_task(&self) -> Option<(String, Arc<AtomicBool>)> {
        let mut state = self.0.state.lock().ok()?;
        loop {
            if state.shutting_down {
                return None;
            }
            while let Some(image_id) = state.pending.pop_front() {
                let Some(task) = state.tasks.get_mut(&image_id) else {
                    continue;
                };
                if task.status != MetadataTaskStatus::Idle
                    || task.cancellation.load(Ordering::Acquire)
                {
                    continue;
                }
                task.status = MetadataTaskStatus::Running;
                return Some((image_id, task.cancellation.clone()));
            }
            state = self.0.ready.wait(state).ok()?;
        }
    }

    fn finish_task(&self, image_id: &str, status: MetadataTaskStatus) {
        if let Ok(mut state) = self.0.state.lock() {
            if let Some(task) = state.tasks.get_mut(image_id) {
                task.status = if task.cancellation.load(Ordering::Acquire) {
                    MetadataTaskStatus::Cancelled
                } else {
                    status
                };
            }
        }
    }
}

fn status(state: &QueueState) -> MetadataQueueStatus {
    let mut result = MetadataQueueStatus {
        total: state.tasks.len(),
        ..Default::default()
    };
    for task in state.tasks.values() {
        match task.status {
            MetadataTaskStatus::Idle => result.idle += 1,
            MetadataTaskStatus::Running => result.running += 1,
            MetadataTaskStatus::Completed => result.completed += 1,
            MetadataTaskStatus::Failed => result.failed += 1,
            MetadataTaskStatus::Cancelled => result.cancelled += 1,
        }
    }
    result
}

fn emit_update(
    app: &AppHandle,
    image_id: &str,
    status: MetadataTaskStatus,
    metadata: Option<ImageMetadata>,
    error_code: Option<&'static str>,
) {
    let _ = app.emit(
        "metadata-task-updated",
        MetadataTaskUpdate {
            image_id: image_id.to_owned(),
            status,
            metadata,
            error_code,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::MetadataTaskQueue;

    fn ids(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("image-{index}")).collect()
    }

    #[test]
    fn queues_and_cancels_metadata_tasks_at_required_library_sizes() {
        for count in [100, 1_000, 10_000] {
            let queue = MetadataTaskQueue::default();
            let submitted = queue.enqueue(ids(count)).expect("queue metadata tasks");
            assert_eq!(submitted.queue_status.idle, count);
            assert_eq!(submitted.accepted_image_ids.len(), count);
            let cancelled = queue.cancel(None).expect("cancel metadata tasks");
            assert_eq!(cancelled.idle, 0);
            assert_eq!(cancelled.cancelled, count);
        }
    }

    #[test]
    fn retry_replaces_a_cancelled_task_without_duplicates() {
        let queue = MetadataTaskQueue::default();
        queue.enqueue(vec!["image-1".into()]).expect("queue task");
        queue.cancel(Some("image-1")).expect("cancel task");
        let retry = queue.retry("image-1".into()).expect("retry task");
        assert_eq!(retry.queue_status.total, 1);
        assert_eq!(retry.queue_status.idle, 1);
    }
}
