use tauri::State;

use crate::{
    database::{ImageFilter, ImageSearchQuery, MetadataDatabase, SearchCommandError},
    scanner::ImageAsset,
};

#[tauri::command]
pub async fn search_images(
    database: State<'_, MetadataDatabase>,
    query: String,
) -> Result<Vec<ImageAsset>, SearchCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.search_images(&query))
        .await
        .map_err(|error| {
            SearchCommandError::operation(format!("Search task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn filter_images(
    database: State<'_, MetadataDatabase>,
    filters: ImageFilter,
) -> Result<Vec<ImageAsset>, SearchCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.filter_images(&filters))
        .await
        .map_err(|error| {
            SearchCommandError::operation(format!("Filter task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn query_image_assets(
    database: State<'_, MetadataDatabase>,
    query: ImageSearchQuery,
) -> Result<Vec<ImageAsset>, SearchCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.query_image_assets(&query))
        .await
        .map_err(|error| {
            SearchCommandError::operation(format!("Image query task could not complete: {error}"))
        })?
}
