use tauri::State;

use crate::{
    database::{
        MetadataDatabase, SmartCollection, SmartCollectionCommandError, SmartCollectionRule,
    },
    scanner::ImageAsset,
};

#[tauri::command]
pub async fn create_smart_collection(
    database: State<'_, MetadataDatabase>,
    name: String,
    rule: SmartCollectionRule,
) -> Result<SmartCollection, SmartCollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.create_smart_collection(&name, rule))
        .await
        .map_err(|error| {
            SmartCollectionCommandError::operation(format!(
                "Smart collection task could not complete: {error}"
            ))
        })?
}

#[tauri::command]
pub async fn delete_smart_collection(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
) -> Result<(), SmartCollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.delete_smart_collection(&collection_id))
        .await
        .map_err(|error| {
            SmartCollectionCommandError::operation(format!(
                "Smart collection task could not complete: {error}"
            ))
        })?
}

#[tauri::command]
pub async fn rename_smart_collection(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
    name: String,
) -> Result<SmartCollection, SmartCollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.rename_smart_collection(&collection_id, &name)
    })
    .await
    .map_err(|error| {
        SmartCollectionCommandError::operation(format!(
            "Smart collection task could not complete: {error}"
        ))
    })?
}

#[tauri::command]
pub async fn list_smart_collections(
    database: State<'_, MetadataDatabase>,
) -> Result<Vec<SmartCollection>, SmartCollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.list_smart_collections())
        .await
        .map_err(|error| {
            SmartCollectionCommandError::operation(format!(
                "Smart collection task could not complete: {error}"
            ))
        })?
}

#[tauri::command]
pub async fn list_smart_collection_images(
    database: State<'_, MetadataDatabase>,
    collection_id: String,
) -> Result<Vec<ImageAsset>, SmartCollectionCommandError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.list_smart_collection_images(&collection_id)
    })
    .await
    .map_err(|error| {
        SmartCollectionCommandError::operation(format!(
            "Smart collection task could not complete: {error}"
        ))
    })?
}
