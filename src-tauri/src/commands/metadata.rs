use tauri::State;

use crate::{
    database::{ImageMetadata, ImageMetadataError, MetadataDatabase},
    metadata::MetadataExtractionError,
    metadata_queue::{MetadataQueueStatus, MetadataQueueSubmission, MetadataTaskQueue},
};

#[tauri::command]
pub fn get_image_metadata(
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<Option<ImageMetadata>, ImageMetadataError> {
    database.get_image_metadata(&image_id)
}

#[tauri::command]
pub fn enqueue_metadata_tasks(
    queue: State<'_, MetadataTaskQueue>,
    image_ids: Vec<String>,
) -> Result<MetadataQueueSubmission, MetadataExtractionError> {
    queue.enqueue(image_ids)
}

#[tauri::command]
pub fn retry_image_metadata(
    queue: State<'_, MetadataTaskQueue>,
    image_id: String,
) -> Result<MetadataQueueSubmission, MetadataExtractionError> {
    queue.retry(image_id)
}

#[tauri::command]
pub fn cancel_metadata_tasks(
    queue: State<'_, MetadataTaskQueue>,
    image_id: Option<String>,
) -> Result<MetadataQueueStatus, MetadataExtractionError> {
    queue.cancel(image_id.as_deref())
}

#[tauri::command]
pub fn get_metadata_queue_status(
    queue: State<'_, MetadataTaskQueue>,
) -> Result<MetadataQueueStatus, MetadataExtractionError> {
    queue.status()
}
