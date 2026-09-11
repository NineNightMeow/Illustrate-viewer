use tauri::State;

use crate::{
    database::{FavoriteCommandError, MetadataDatabase},
    scanner::ImageAsset,
};

#[tauri::command]
pub async fn add_favorite(
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<(), FavoriteCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.add_favorite(&image_id))
        .await
        .map_err(|error| {
            FavoriteCommandError::operation(format!("Favorite task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn remove_favorite(
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<(), FavoriteCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.remove_favorite(&image_id))
        .await
        .map_err(|error| {
            FavoriteCommandError::operation(format!("Favorite task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn is_favorite(
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<bool, FavoriteCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.is_favorite(&image_id))
        .await
        .map_err(|error| {
            FavoriteCommandError::operation(format!("Favorite task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn list_favorites(
    database: State<'_, MetadataDatabase>,
) -> Result<Vec<ImageAsset>, FavoriteCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_favorites())
        .await
        .map_err(|error| {
            FavoriteCommandError::operation(format!("Favorite task could not complete: {error}"))
        })?
}
