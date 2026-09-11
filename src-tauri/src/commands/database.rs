use tauri::State;

use crate::database::{MetadataDatabase, MetadataDatabaseError, MetadataDatabaseStatus};

#[tauri::command]
pub async fn get_metadata_database_status(
    database: State<'_, MetadataDatabase>,
) -> Result<MetadataDatabaseStatus, MetadataDatabaseError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || database.status())
        .await
        .map_err(|error| {
            MetadataDatabaseError::operation(format!(
                "Metadata database task could not complete: {error}"
            ))
        })?
}
