use std::collections::HashSet;

use tauri::{AppHandle, State};

use crate::thumbnail::{
    clear_thumbnail_cache as clear_thumbnail_cache_files, find_cached_thumbnail_record,
    prepare_image_asset as prepare_image_asset_file, GalleryThumbnailWindow, ThumbnailCommandError,
    ThumbnailQueueStatus, ThumbnailQueueSubmission, ThumbnailRequest, ThumbnailTaskQueue,
};

#[tauri::command]
pub async fn clear_thumbnail_cache(app: AppHandle) -> Result<(), ThumbnailCommandError> {
    tauri::async_runtime::spawn_blocking(move || clear_thumbnail_cache_files(&app))
        .await
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?
}

#[tauri::command]
pub async fn prepare_image_asset(
    app: AppHandle,
    asset: ThumbnailRequest,
) -> Result<String, ThumbnailCommandError> {
    tauri::async_runtime::spawn_blocking(move || prepare_image_asset_file(&app, &asset))
        .await
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?
}

#[tauri::command]
pub async fn enqueue_thumbnail_tasks(
    app: AppHandle,
    queue: State<'_, ThumbnailTaskQueue>,
    tasks: Vec<ThumbnailRequest>,
) -> Result<ThumbnailQueueSubmission, ThumbnailCommandError> {
    let queue = (*queue).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (pending, cache_hits) = split_cached_requests(&app, tasks);
        let mut submission = queue.enqueue(pending)?;
        submission.cache_hits = cache_hits;
        Ok(submission)
    })
    .await
    .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?
}

#[tauri::command]
pub async fn sync_gallery_thumbnail_window(
    app: AppHandle,
    queue: State<'_, ThumbnailTaskQueue>,
    window: GalleryThumbnailWindow,
) -> Result<ThumbnailQueueSubmission, ThumbnailCommandError> {
    let queue = (*queue).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (window, cache_hits) = split_cached_gallery_window(&app, window);
        let mut submission = queue.sync_gallery_window(window)?;
        submission.cache_hits = cache_hits;
        Ok(submission)
    })
    .await
    .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?
}

#[tauri::command]
pub async fn release_gallery_thumbnail_window(
    queue: State<'_, ThumbnailTaskQueue>,
    scope_id: String,
) -> Result<ThumbnailQueueStatus, ThumbnailCommandError> {
    let queue = (*queue).clone();
    tauri::async_runtime::spawn_blocking(move || queue.release_gallery_window(&scope_id))
        .await
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?
}

#[tauri::command]
pub async fn get_thumbnail_queue_status(
    queue: State<'_, ThumbnailTaskQueue>,
) -> Result<ThumbnailQueueStatus, ThumbnailCommandError> {
    let queue = (*queue).clone();
    tauri::async_runtime::spawn_blocking(move || queue.status())
        .await
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?
}

#[tauri::command]
pub async fn cancel_thumbnail_tasks(
    queue: State<'_, ThumbnailTaskQueue>,
    library_id: Option<String>,
) -> Result<ThumbnailQueueStatus, ThumbnailCommandError> {
    let queue = (*queue).clone();
    tauri::async_runtime::spawn_blocking(move || queue.cancel(library_id.as_deref()))
        .await
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?
}

fn split_cached_requests(
    app: &AppHandle,
    tasks: Vec<ThumbnailRequest>,
) -> (
    Vec<ThumbnailRequest>,
    Vec<crate::thumbnail::ThumbnailRecord>,
) {
    let mut pending = Vec::new();
    let mut cache_hits = Vec::new();
    let mut seen = HashSet::new();

    for task in tasks {
        if !seen.insert(task.image_id.clone()) {
            continue;
        }
        match find_cached_thumbnail_record(app, &task) {
            Ok(Some(record)) => cache_hits.push(record),
            Ok(None) | Err(_) => pending.push(task),
        }
    }

    (pending, cache_hits)
}

fn split_cached_gallery_window(
    app: &AppHandle,
    window: GalleryThumbnailWindow,
) -> (
    GalleryThumbnailWindow,
    Vec<crate::thumbnail::ThumbnailRecord>,
) {
    let mut seen = HashSet::new();
    let (viewport, mut cache_hits) = split_cached_window_requests(app, window.viewport, &mut seen);
    let (prefetch, prefetch_cache_hits) =
        split_cached_window_requests(app, window.prefetch, &mut seen);
    cache_hits.extend(prefetch_cache_hits);

    (
        GalleryThumbnailWindow {
            scope_id: window.scope_id,
            viewport,
            prefetch,
        },
        cache_hits,
    )
}

fn split_cached_window_requests(
    app: &AppHandle,
    tasks: Vec<ThumbnailRequest>,
    seen: &mut HashSet<String>,
) -> (
    Vec<ThumbnailRequest>,
    Vec<crate::thumbnail::ThumbnailRecord>,
) {
    let mut pending = Vec::new();
    let mut cache_hits = Vec::new();

    for task in tasks {
        if !seen.insert(task.image_id.clone()) {
            continue;
        }
        match find_cached_thumbnail_record(app, &task) {
            Ok(Some(record)) => cache_hits.push(record),
            Ok(None) | Err(_) => pending.push(task),
        }
    }

    (pending, cache_hits)
}
