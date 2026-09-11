use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use super::{generate_thumbnail_record_with_cancellation, ThumbnailCommandError, ThumbnailRequest};

const THUMBNAIL_WORKER_COUNT: usize = 2;
const PRIORITY_COUNT: usize = 6;
const MAX_BACKGROUND_QUEUED: usize = 128;

#[derive(Clone)]
pub struct ThumbnailTaskQueue(Arc<QueueInner>);

struct QueueInner {
    state: Mutex<QueueState>,
    ready: Condvar,
}

struct QueueState {
    tasks: HashMap<String, ThumbnailTask>,
    active_task_by_image_id: HashMap<String, String>,
    pending: [VecDeque<String>; PRIORITY_COUNT],
    total: usize,
    completed: usize,
    failed: usize,
    cancelled: usize,
    workers_started: bool,
    shutting_down: bool,
}

struct ThumbnailTask {
    id: String,
    image_id: String,
    library_id: String,
    source_path: String,
    window_scope: Option<String>,
    priority: ThumbnailTaskPriority,
    status: ThumbnailTaskStatus,
    cancellation: Arc<AtomicBool>,
}

struct ProcessingTask {
    id: String,
    image_id: String,
    library_id: String,
    source_path: String,
    cancellation: Arc<AtomicBool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThumbnailTaskPriority {
    ViewerCurrent,
    QuickPreviewCurrent,
    ViewerAdjacent,
    GalleryViewport,
    GalleryPrefetch,
    Background,
}

impl ThumbnailTaskPriority {
    const ORDER: [Self; PRIORITY_COUNT] = [
        Self::ViewerCurrent,
        Self::QuickPreviewCurrent,
        Self::ViewerAdjacent,
        Self::GalleryViewport,
        Self::GalleryPrefetch,
        Self::Background,
    ];

    const fn index(self) -> usize {
        match self {
            Self::ViewerCurrent => 0,
            Self::QuickPreviewCurrent => 1,
            Self::ViewerAdjacent => 2,
            Self::GalleryViewport => 3,
            Self::GalleryPrefetch => 4,
            Self::Background => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ThumbnailTaskStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryThumbnailWindow {
    pub scope_id: String,
    pub viewport: Vec<ThumbnailRequest>,
    pub prefetch: Vec<ThumbnailRequest>,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailQueueStatus {
    pub total: usize,
    pub queued: usize,
    pub queued_high: usize,
    pub queued_normal: usize,
    pub queued_low: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailQueueSubmission {
    pub queue_status: ThumbnailQueueStatus,
    pub accepted_image_ids: Vec<String>,
    pub cache_hits: Vec<super::ThumbnailRecord>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThumbnailTaskUpdate {
    image_id: String,
    status: super::ThumbnailStatus,
    record: Option<super::ThumbnailRecord>,
    error_code: Option<String>,
}

impl Default for ThumbnailTaskQueue {
    fn default() -> Self {
        Self(Arc::new(QueueInner {
            state: Mutex::new(QueueState {
                tasks: HashMap::new(),
                active_task_by_image_id: HashMap::new(),
                pending: std::array::from_fn(|_| VecDeque::new()),
                total: 0,
                completed: 0,
                failed: 0,
                cancelled: 0,
                workers_started: false,
                shutting_down: false,
            }),
            ready: Condvar::new(),
        }))
    }
}

impl ThumbnailTaskQueue {
    pub fn start_workers(&self, app: AppHandle) -> Result<(), ThumbnailCommandError> {
        let mut state = self.lock_state()?;
        if state.workers_started {
            return Ok(());
        }
        state.workers_started = true;
        drop(state);

        for worker_index in 0..THUMBNAIL_WORKER_COUNT {
            let queue = self.clone();
            let app = app.clone();
            thread::Builder::new()
                .name(format!("thumbnail-worker-{}", worker_index + 1))
                .spawn(move || {
                    if worker_index == 0 {
                        super::remove_stale_thumbnail_temporary_files(&app);
                    }
                    queue.run_worker(app);
                })
                .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?;
        }

        Ok(())
    }

    pub fn enqueue(
        &self,
        requests: Vec<ThumbnailRequest>,
    ) -> Result<ThumbnailQueueSubmission, ThumbnailCommandError> {
        let mut state = self.lock_state()?;
        ensure_running(&state)?;

        let mut accepted = Vec::new();
        let mut accepted_set = HashSet::new();
        for request in requests {
            upsert_task(
                &mut state,
                request,
                ThumbnailTaskPriority::Background,
                None,
                &mut accepted,
                &mut accepted_set,
            );
        }

        let submission = submission(&state, accepted);
        self.0.ready.notify_all();
        Ok(submission)
    }

    pub fn sync_gallery_window(
        &self,
        window: GalleryThumbnailWindow,
    ) -> Result<ThumbnailQueueSubmission, ThumbnailCommandError> {
        let mut state = self.lock_state()?;
        ensure_running(&state)?;

        cancel_queued_tasks_outside_window_scope(&mut state, &window.scope_id);

        let mut desired = HashMap::new();
        for request in window.prefetch {
            desired.insert(
                request.image_id.clone(),
                (request, ThumbnailTaskPriority::GalleryPrefetch),
            );
        }
        for request in window.viewport {
            desired.insert(
                request.image_id.clone(),
                (request, ThumbnailTaskPriority::GalleryViewport),
            );
        }

        let mut accepted = Vec::new();
        let mut accepted_set = HashSet::new();
        for (_, (request, priority)) in desired.iter() {
            upsert_task(
                &mut state,
                request.clone(),
                *priority,
                Some(&window.scope_id),
                &mut accepted,
                &mut accepted_set,
            );
        }

        let stale_task_ids: Vec<String> = state
            .tasks
            .values()
            .filter(|task| {
                task.status == ThumbnailTaskStatus::Queued
                    && task.window_scope.as_deref() == Some(window.scope_id.as_str())
                    && !desired.contains_key(&task.image_id)
            })
            .map(|task| task.id.clone())
            .collect();
        for task_id in stale_task_ids {
            set_queued_priority(&mut state, &task_id, ThumbnailTaskPriority::Background);
        }
        trim_background_queue(&mut state);

        let submission = submission(&state, accepted);
        self.0.ready.notify_all();
        Ok(submission)
    }

    pub fn release_gallery_window(
        &self,
        scope_id: &str,
    ) -> Result<ThumbnailQueueStatus, ThumbnailCommandError> {
        let mut state = self.lock_state()?;
        let queued_task_ids: Vec<String> = state
            .tasks
            .values()
            .filter(|task| {
                task.status == ThumbnailTaskStatus::Queued
                    && task.window_scope.as_deref() == Some(scope_id)
            })
            .map(|task| task.id.clone())
            .collect();
        for task_id in queued_task_ids {
            set_queued_priority(&mut state, &task_id, ThumbnailTaskPriority::Background);
        }
        trim_background_queue(&mut state);

        let status = queue_status(&state);
        self.0.ready.notify_all();
        Ok(status)
    }

    pub fn status(&self) -> Result<ThumbnailQueueStatus, ThumbnailCommandError> {
        let state = self.lock_state()?;
        Ok(queue_status(&state))
    }

    pub fn cancel(
        &self,
        library_id: Option<&str>,
    ) -> Result<ThumbnailQueueStatus, ThumbnailCommandError> {
        let mut state = self.lock_state()?;
        cancel_tasks(&mut state, library_id);
        let status = queue_status(&state);
        self.0.ready.notify_all();
        Ok(status)
    }

    pub fn shutdown(&self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.shutting_down = true;
            cancel_tasks(&mut state, None);
            for pending in &mut state.pending {
                pending.clear();
            }
            self.0.ready.notify_all();
        }
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, QueueState>, ThumbnailCommandError> {
        self.0.state.lock().map_err(|_| {
            ThumbnailCommandError::thumbnail_failed("The thumbnail queue is unavailable.")
        })
    }

    fn run_worker(&self, app: AppHandle) {
        while let Some(task) = self.next_task() {
            let request = ThumbnailRequest {
                image_id: task.image_id.clone(),
                library_id: task.library_id.clone(),
                source_path: task.source_path.clone(),
            };
            let result =
                generate_thumbnail_record_with_cancellation(&app, request, &task.cancellation);
            emit_task_update(&app, &task.image_id, &result, &task.cancellation);
            self.finish_task(&task.id, result, &task.cancellation);
        }
    }

    fn next_task(&self) -> Option<ProcessingTask> {
        let mut state = self.0.state.lock().ok()?;

        loop {
            if state.shutting_down {
                return None;
            }

            for priority in ThumbnailTaskPriority::ORDER {
                while let Some(task_id) = state.pending[priority.index()].pop_front() {
                    let Some(task) = state.tasks.get_mut(&task_id) else {
                        continue;
                    };
                    if task.status != ThumbnailTaskStatus::Queued
                        || task.cancellation.load(Ordering::Acquire)
                    {
                        continue;
                    }

                    task.status = ThumbnailTaskStatus::Processing;
                    return Some(ProcessingTask {
                        id: task.id.clone(),
                        image_id: task.image_id.clone(),
                        library_id: task.library_id.clone(),
                        source_path: task.source_path.clone(),
                        cancellation: task.cancellation.clone(),
                    });
                }
            }

            state = self.0.ready.wait(state).ok()?;
        }
    }

    fn finish_task(
        &self,
        task_id: &str,
        result: Result<super::ThumbnailRecord, ThumbnailCommandError>,
        cancellation: &AtomicBool,
    ) {
        let Ok(mut state) = self.0.state.lock() else {
            return;
        };
        let Some(mut task) = state.tasks.remove(task_id) else {
            return;
        };
        state.active_task_by_image_id.remove(&task.image_id);

        if task.status == ThumbnailTaskStatus::Cancelled || cancellation.load(Ordering::Acquire) {
            task.status = ThumbnailTaskStatus::Cancelled;
            return;
        }

        match result {
            Ok(_) => {
                task.status = ThumbnailTaskStatus::Completed;
                state.completed += 1;
            }
            Err(ThumbnailCommandError {
                code: "cancelled", ..
            }) => {
                task.status = ThumbnailTaskStatus::Cancelled;
                state.cancelled += 1;
            }
            Err(_) => {
                task.status = ThumbnailTaskStatus::Failed;
                state.failed += 1;
            }
        }
    }
}

fn ensure_running(state: &QueueState) -> Result<(), ThumbnailCommandError> {
    if state.shutting_down {
        return Err(ThumbnailCommandError::thumbnail_failed(
            "The thumbnail queue is shutting down.",
        ));
    }
    Ok(())
}

fn upsert_task(
    state: &mut QueueState,
    request: ThumbnailRequest,
    priority: ThumbnailTaskPriority,
    window_scope: Option<&str>,
    accepted: &mut Vec<String>,
    accepted_set: &mut HashSet<String>,
) {
    if let Some(task_id) = state
        .active_task_by_image_id
        .get(&request.image_id)
        .cloned()
    {
        match state.tasks.get(&task_id).map(|task| task.status) {
            Some(ThumbnailTaskStatus::Queued) => {
                set_queued_priority(state, &task_id, priority);
                if let Some(task) = state.tasks.get_mut(&task_id) {
                    task.source_path = request.source_path;
                    task.window_scope = window_scope.map(str::to_owned);
                }
            }
            Some(ThumbnailTaskStatus::Processing) => {}
            _ => {
                state.active_task_by_image_id.remove(&request.image_id);
                enqueue_new_task(state, request.clone(), priority, window_scope);
            }
        }
        push_accepted(accepted, accepted_set, request.image_id);
        return;
    }

    let image_id = request.image_id.clone();
    enqueue_new_task(state, request, priority, window_scope);
    push_accepted(accepted, accepted_set, image_id);
}

fn enqueue_new_task(
    state: &mut QueueState,
    request: ThumbnailRequest,
    priority: ThumbnailTaskPriority,
    window_scope: Option<&str>,
) {
    let task_id = Uuid::new_v4().to_string();
    state.pending[priority.index()].push_back(task_id.clone());
    state
        .active_task_by_image_id
        .insert(request.image_id.clone(), task_id.clone());
    state.tasks.insert(
        task_id.clone(),
        ThumbnailTask {
            id: task_id,
            image_id: request.image_id,
            library_id: request.library_id,
            source_path: request.source_path,
            window_scope: window_scope.map(str::to_owned),
            priority,
            status: ThumbnailTaskStatus::Queued,
            cancellation: Arc::new(AtomicBool::new(false)),
        },
    );
    state.total += 1;
}

fn push_accepted(accepted: &mut Vec<String>, accepted_set: &mut HashSet<String>, image_id: String) {
    if accepted_set.insert(image_id.clone()) {
        accepted.push(image_id);
    }
}

fn set_queued_priority(state: &mut QueueState, task_id: &str, priority: ThumbnailTaskPriority) {
    let Some(task) = state.tasks.get(task_id) else {
        return;
    };
    if task.status != ThumbnailTaskStatus::Queued || task.priority == priority {
        return;
    }
    let previous_priority = task.priority;
    state.pending[previous_priority.index()].retain(|pending_id| pending_id != task_id);
    if let Some(task) = state.tasks.get_mut(task_id) {
        task.priority = priority;
    }
    state.pending[priority.index()].push_back(task_id.to_string());
}

fn cancel_queued_tasks_outside_window_scope(state: &mut QueueState, scope_id: &str) {
    let task_ids: Vec<String> = state
        .tasks
        .values()
        .filter(|task| {
            task.status == ThumbnailTaskStatus::Queued
                && task
                    .window_scope
                    .as_deref()
                    .is_some_and(|scope| scope != scope_id)
        })
        .map(|task| task.id.clone())
        .collect();
    for task_id in task_ids {
        discard_queued_task(state, &task_id);
    }
}

fn trim_background_queue(state: &mut QueueState) {
    while state.pending[ThumbnailTaskPriority::Background.index()].len() > MAX_BACKGROUND_QUEUED {
        let Some(task_id) = state.pending[ThumbnailTaskPriority::Background.index()].pop_front()
        else {
            break;
        };
        discard_queued_task(state, &task_id);
    }
}

fn discard_queued_task(state: &mut QueueState, task_id: &str) {
    let Some(task) = state.tasks.get(task_id) else {
        return;
    };
    if task.status != ThumbnailTaskStatus::Queued {
        return;
    }
    let priority = task.priority;
    state.pending[priority.index()].retain(|pending_id| pending_id != task_id);
    let Some(task) = state.tasks.remove(task_id) else {
        return;
    };
    if state
        .active_task_by_image_id
        .get(&task.image_id)
        .is_some_and(|active_id| active_id == task_id)
    {
        state.active_task_by_image_id.remove(&task.image_id);
    }
    state.cancelled += 1;
}

fn emit_task_update(
    app: &AppHandle,
    image_id: &str,
    result: &Result<super::ThumbnailRecord, ThumbnailCommandError>,
    cancellation: &AtomicBool,
) {
    let update = if cancellation.load(Ordering::Acquire) {
        ThumbnailTaskUpdate {
            image_id: image_id.to_string(),
            status: super::ThumbnailStatus::Failed,
            record: None,
            error_code: Some("cancelled".into()),
        }
    } else {
        match result {
            Ok(record) => ThumbnailTaskUpdate {
                image_id: image_id.to_string(),
                status: super::ThumbnailStatus::Completed,
                record: Some(record.clone()),
                error_code: None,
            },
            Err(error) => ThumbnailTaskUpdate {
                image_id: image_id.to_string(),
                status: super::ThumbnailStatus::Failed,
                record: None,
                error_code: Some(error.code.into()),
            },
        }
    };

    let _ = app.emit("thumbnail-task-updated", update);
}

fn cancel_tasks(state: &mut QueueState, library_id: Option<&str>) {
    let queued_task_ids: Vec<String> = state
        .tasks
        .values()
        .filter(|task| {
            task.status == ThumbnailTaskStatus::Queued
                && library_id
                    .map(|library_id| task.library_id == library_id)
                    .unwrap_or(true)
        })
        .map(|task| task.id.clone())
        .collect();
    for task_id in queued_task_ids {
        discard_queued_task(state, &task_id);
    }

    for task in state.tasks.values_mut() {
        let matches_library = library_id
            .map(|library_id| task.library_id == library_id)
            .unwrap_or(true);
        if task.status != ThumbnailTaskStatus::Processing || !matches_library {
            continue;
        }
        task.cancellation.store(true, Ordering::Release);
        task.status = ThumbnailTaskStatus::Cancelled;
        state.cancelled += 1;
    }
}

fn submission(state: &QueueState, accepted_image_ids: Vec<String>) -> ThumbnailQueueSubmission {
    ThumbnailQueueSubmission {
        queue_status: queue_status(state),
        accepted_image_ids,
        cache_hits: Vec::new(),
    }
}

fn queue_status(state: &QueueState) -> ThumbnailQueueStatus {
    let mut status = ThumbnailQueueStatus {
        total: state.total,
        completed: state.completed,
        failed: state.failed,
        cancelled: state.cancelled,
        ..Default::default()
    };

    for task in state.tasks.values() {
        match task.status {
            ThumbnailTaskStatus::Queued => {
                status.queued += 1;
                match task.priority {
                    ThumbnailTaskPriority::GalleryViewport => status.queued_high += 1,
                    ThumbnailTaskPriority::GalleryPrefetch => status.queued_normal += 1,
                    ThumbnailTaskPriority::Background => status.queued_low += 1,
                    ThumbnailTaskPriority::ViewerCurrent
                    | ThumbnailTaskPriority::QuickPreviewCurrent
                    | ThumbnailTaskPriority::ViewerAdjacent => status.queued_high += 1,
                }
            }
            ThumbnailTaskStatus::Processing => status.processing += 1,
            ThumbnailTaskStatus::Completed
            | ThumbnailTaskStatus::Failed
            | ThumbnailTaskStatus::Cancelled => {}
        }
    }

    status
}

#[cfg(test)]
mod tests {
    use super::{GalleryThumbnailWindow, ThumbnailTaskQueue, ThumbnailTaskStatus};
    use crate::thumbnail::ThumbnailRequest;

    fn request(library_id: &str, index: usize) -> ThumbnailRequest {
        ThumbnailRequest {
            image_id: format!("{library_id}:image-{index}.png"),
            library_id: library_id.into(),
            source_path: format!("C:/images/{library_id}/image-{index}.png"),
        }
    }

    fn requests(count: usize) -> Vec<ThumbnailRequest> {
        (0..count)
            .map(|index| request("library-1", index))
            .collect()
    }

    fn window(
        scope_id: &str,
        viewport: Vec<ThumbnailRequest>,
        prefetch: Vec<ThumbnailRequest>,
    ) -> GalleryThumbnailWindow {
        GalleryThumbnailWindow {
            scope_id: scope_id.into(),
            viewport,
            prefetch,
        }
    }

    #[test]
    fn queues_and_cancels_one_thousand_tasks() {
        let queue = ThumbnailTaskQueue::default();
        let status = queue.enqueue(requests(1_000)).unwrap().queue_status;
        assert_eq!(status.total, 1_000);
        assert_eq!(status.queued_low, 1_000);

        let status = queue.cancel(None).expect("tasks should cancel");
        assert_eq!(status.queued, 0);
        assert_eq!(status.cancelled, 1_000);
    }

    #[test]
    fn queues_and_cancels_ten_thousand_tasks() {
        let queue = ThumbnailTaskQueue::default();
        let status = queue.enqueue(requests(10_000)).unwrap().queue_status;
        assert_eq!(status.total, 10_000);
        assert_eq!(status.queued, 10_000);

        let status = queue
            .cancel(Some("library-1"))
            .expect("tasks should cancel");
        assert_eq!(status.queued, 0);
        assert_eq!(status.cancelled, 10_000);
    }

    #[test]
    fn ten_thousand_search_results_queue_only_the_visible_window() {
        let queue = ThumbnailTaskQueue::default();
        let search_results: Vec<_> = (0..10_000)
            .map(|index| {
                request(
                    if index % 2 == 0 {
                        "library-1"
                    } else {
                        "library-2"
                    },
                    index,
                )
            })
            .collect();

        let status = queue
            .sync_gallery_window(window(
                "search",
                search_results[..8].to_vec(),
                search_results[8..16].to_vec(),
            ))
            .unwrap()
            .queue_status;

        assert_eq!(status.total, 16);
        assert_eq!(status.queued_high, 8);
        assert_eq!(status.queued_normal, 8);
        assert_eq!(status.queued_low, 0);
    }

    #[test]
    fn search_window_replacement_demotes_stale_work_without_duplicate_tasks() {
        let queue = ThumbnailTaskQueue::default();
        let first_window: Vec<_> = (0..129).map(|index| request("library-1", index)).collect();
        queue
            .sync_gallery_window(window("search", first_window, Vec::new()))
            .unwrap();

        let status = queue
            .sync_gallery_window(window("search", vec![request("library-2", 0)], Vec::new()))
            .unwrap()
            .queue_status;

        assert_eq!(status.queued_high, 1);
        assert_eq!(status.queued_low, 128);
        assert_eq!(status.cancelled, 1);
        assert_eq!(status.total, 130);
    }

    #[test]
    fn gallery_tasks_are_promoted_without_duplicate_active_work() {
        let queue = ThumbnailTaskQueue::default();
        let first = queue
            .sync_gallery_window(window(
                "library-1",
                Vec::new(),
                vec![request("library-1", 1)],
            ))
            .unwrap();
        assert_eq!(first.queue_status.total, 1);
        assert_eq!(first.queue_status.queued_normal, 1);

        let promoted = queue
            .sync_gallery_window(window(
                "library-1",
                vec![request("library-1", 1)],
                Vec::new(),
            ))
            .unwrap();
        assert_eq!(promoted.queue_status.total, 1);
        assert_eq!(promoted.queue_status.queued_high, 1);
        assert_eq!(promoted.accepted_image_ids, vec!["library-1:image-1.png"]);
    }

    #[test]
    fn workers_take_viewport_work_before_prefetch_and_background_work() {
        let queue = ThumbnailTaskQueue::default();
        queue.enqueue(vec![request("library-1", 1)]).unwrap();
        queue
            .sync_gallery_window(window(
                "library-1",
                vec![request("library-1", 3)],
                vec![request("library-1", 2)],
            ))
            .unwrap();

        assert_eq!(queue.next_task().unwrap().image_id, "library-1:image-3.png");
        assert_eq!(queue.next_task().unwrap().image_id, "library-1:image-2.png");
        assert_eq!(queue.next_task().unwrap().image_id, "library-1:image-1.png");
    }

    #[test]
    fn stale_gallery_tasks_are_capped_at_one_hundred_twenty_eight_low_priority_items() {
        let queue = ThumbnailTaskQueue::default();
        let viewport = (0..129).map(|index| request("library-1", index)).collect();
        queue
            .sync_gallery_window(window("library-1", viewport, Vec::new()))
            .unwrap();

        let status = queue
            .sync_gallery_window(window("library-1", Vec::new(), Vec::new()))
            .unwrap()
            .queue_status;
        assert_eq!(status.queued_high, 0);
        assert_eq!(status.queued_low, 128);
        assert_eq!(status.cancelled, 1);
    }

    #[test]
    fn switching_libraries_cancels_queued_work_but_allows_processing_work_to_finish() {
        let queue = ThumbnailTaskQueue::default();
        queue
            .sync_gallery_window(window(
                "library-1",
                vec![request("library-1", 1)],
                Vec::new(),
            ))
            .unwrap();
        let processing = queue.next_task().expect("first task should start");
        queue
            .sync_gallery_window(window(
                "library-1",
                vec![request("library-1", 2)],
                Vec::new(),
            ))
            .unwrap();

        let status = queue
            .sync_gallery_window(window(
                "library-2",
                vec![request("library-2", 1)],
                Vec::new(),
            ))
            .unwrap()
            .queue_status;
        assert_eq!(status.processing, 1);
        assert_eq!(status.queued_high, 1);
        assert_eq!(status.cancelled, 1);
        assert!(!processing
            .cancellation
            .load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(
            queue
                .0
                .state
                .lock()
                .unwrap()
                .tasks
                .get(&processing.id)
                .unwrap()
                .status,
            ThumbnailTaskStatus::Processing
        );
    }

    #[test]
    fn cancellation_marks_processing_tasks_without_waiting_for_a_worker() {
        let queue = ThumbnailTaskQueue::default();
        queue.enqueue(requests(1)).expect("task should queue");
        let processing = queue.next_task().expect("task should start");
        assert_eq!(queue.status().unwrap().processing, 1);

        let status = queue.cancel(Some("library-1")).expect("task should cancel");
        assert_eq!(status.cancelled, 1);
        assert!(processing
            .cancellation
            .load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(
            queue
                .0
                .state
                .lock()
                .unwrap()
                .tasks
                .get(&processing.id)
                .unwrap()
                .status,
            ThumbnailTaskStatus::Cancelled
        );
    }
}
