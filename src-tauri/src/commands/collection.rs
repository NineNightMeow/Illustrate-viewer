use tauri::State;

use crate::{
    database::{Collection, CollectionCommandError, CollectionMembership, MetadataDatabase},
    scanner::ImageAsset,
};

#[tauri::command]
pub async fn create_collection(
    database: State<'_, MetadataDatabase>,
    name: String,
) -> Result<Collection, CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.create_collection(&name))
        .await
        .map_err(|error| {
            CollectionCommandError::operation(format!(
                "Collection task could not complete: {error}"
            ))
        })?
}

#[tauri::command]
pub async fn list_collection_memberships(
    database: State<'_, MetadataDatabase>,
    collection_ids: Vec<String>,
    image_ids: Vec<String>,
) -> Result<Vec<CollectionMembership>, CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.list_collection_memberships(&collection_ids, &image_ids)
    })
    .await
    .map_err(|error| {
        CollectionCommandError::operation(format!("Collection task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn add_images_to_collections(
    database: State<'_, MetadataDatabase>,
    collection_ids: Vec<String>,
    image_ids: Vec<String>,
) -> Result<(), CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.add_images_to_collections(&collection_ids, &image_ids)
    })
    .await
    .map_err(|error| {
        CollectionCommandError::operation(format!("Collection task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn create_collection_with_images(
    database: State<'_, MetadataDatabase>,
    name: String,
    image_ids: Vec<String>,
) -> Result<Collection, CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.create_collection_with_images(&name, &image_ids)
    })
    .await
    .map_err(|error| {
        CollectionCommandError::operation(format!("Collection task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn delete_collection(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
) -> Result<(), CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.delete_collection(&collection_id))
        .await
        .map_err(|error| {
            CollectionCommandError::operation(format!(
                "Collection task could not complete: {error}"
            ))
        })?
}

#[tauri::command]
pub async fn rename_collection(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
    name: String,
) -> Result<Collection, CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.rename_collection(&collection_id, &name))
        .await
        .map_err(|error| {
            CollectionCommandError::operation(format!(
                "Collection task could not complete: {error}"
            ))
        })?
}

#[tauri::command]
pub async fn list_collections(
    database: State<'_, MetadataDatabase>,
) -> Result<Vec<Collection>, CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_collections())
        .await
        .map_err(|error| {
            CollectionCommandError::operation(format!(
                "Collection task could not complete: {error}"
            ))
        })?
}

#[tauri::command]
pub async fn add_image_to_collection(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
    image_id: String,
) -> Result<(), CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.add_image_to_collection(&collection_id, &image_id)
    })
    .await
    .map_err(|error| {
        CollectionCommandError::operation(format!("Collection task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn remove_image_from_collection(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
    image_id: String,
) -> Result<(), CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.remove_image_from_collection(&collection_id, &image_id)
    })
    .await
    .map_err(|error| {
        CollectionCommandError::operation(format!("Collection task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn list_collection_images(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
) -> Result<Vec<ImageAsset>, CollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_collection_images(&collection_id))
        .await
        .map_err(|error| {
            CollectionCommandError::operation(format!(
                "Collection task could not complete: {error}"
            ))
        })?
}
