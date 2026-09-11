use tauri::State;

use crate::{
    database::{ImageTagsByImageId, MetadataDatabase, Tag, TagCommandError},
    scanner::ImageAsset,
};

#[tauri::command]
pub async fn create_tag(
    database: State<'_, MetadataDatabase>,
    name: String,
) -> Result<Tag, TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.create_tag(&name))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn delete_tag(
    database: State<'_, MetadataDatabase>,
    tag_id: String,
) -> Result<(), TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.delete_tag(&tag_id))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn rename_tag(
    database: State<'_, MetadataDatabase>,
    tag_id: String,
    name: String,
) -> Result<Tag, TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.rename_tag(&tag_id, &name))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn list_tags(database: State<'_, MetadataDatabase>) -> Result<Vec<Tag>, TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_tags())
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn add_tag_to_image(
    database: State<'_, MetadataDatabase>,
    tag_id: String,
    image_id: String,
) -> Result<(), TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.add_tag_to_image(&tag_id, &image_id))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn remove_tag_from_image(
    database: State<'_, MetadataDatabase>,
    tag_id: String,
    image_id: String,
) -> Result<(), TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.remove_tag_from_image(&tag_id, &image_id))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn list_image_tags(
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<Vec<Tag>, TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_image_tags(&image_id))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn list_image_tags_for_images(
    database: State<'_, MetadataDatabase>,
    image_ids: Vec<String>,
) -> Result<ImageTagsByImageId, TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_image_tags_for_images(&image_ids))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}

#[tauri::command]
pub async fn list_tag_images(
    database: State<'_, MetadataDatabase>,
    tag_id: String,
) -> Result<Vec<ImageAsset>, TagCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_tag_images(&tag_id))
        .await
        .map_err(|error| {
            TagCommandError::operation(format!("Tag task could not complete: {error}"))
        })?
}
