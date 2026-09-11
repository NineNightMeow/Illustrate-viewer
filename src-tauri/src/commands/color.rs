use tauri::{AppHandle, State};

use crate::color::ColorAnalysisError;
use crate::database::{Color, ImageColorMetadata, MetadataDatabase};

#[tauri::command]
pub async fn get_image_color_metadata(
    app: AppHandle,
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<Option<ImageColorMetadata>, ColorAnalysisError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::color::get_cached_image_color_metadata(&app, &database, &image_id)
    })
    .await
    .map_err(|error| {
        ColorAnalysisError::operation(format!("Color metadata task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn analyze_image_colors(
    app: AppHandle,
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<ImageColorMetadata, ColorAnalysisError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::color::analyze_image_colors(&app, &database, &image_id)
    })
    .await
    .map_err(|error| {
        ColorAnalysisError::operation(format!("Color analysis task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn sample_image_color(
    app: AppHandle,
    database: State<'_, MetadataDatabase>,
    image_id: String,
    x: u32,
    y: u32,
) -> Result<Color, ColorAnalysisError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::color::sample_image_color(&app, &database, &image_id, x, y)
    })
    .await
    .map_err(|error| {
        ColorAnalysisError::operation(format!("Color picker task could not complete: {error}"))
    })?
}
