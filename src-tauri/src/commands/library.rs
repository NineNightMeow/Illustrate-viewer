use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::library::{
    create_library_record, folder_selection, list_library_records, remove_library_record,
    FolderSelection, Library, LibraryCommandError,
};

#[tauri::command]
pub async fn select_folder(app: AppHandle) -> Result<Option<FolderSelection>, LibraryCommandError> {
    let selected =
        tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
            .await
            .map_err(|error| LibraryCommandError::dialog(error.to_string()))?;

    selected
        .map(|path| {
            path.into_path()
                .map_err(|error| LibraryCommandError::dialog(error.to_string()))
                .and_then(|path| folder_selection(&path))
        })
        .transpose()
}

#[tauri::command]
pub async fn create_library(app: AppHandle, path: String) -> Result<Library, LibraryCommandError> {
    tauri::async_runtime::spawn_blocking(move || create_library_record(&app, &path))
        .await
        .map_err(|error| LibraryCommandError::dialog(error.to_string()))?
}

#[tauri::command]
pub async fn remove_library(app: AppHandle, id: String) -> Result<(), LibraryCommandError> {
    tauri::async_runtime::spawn_blocking(move || remove_library_record(&app, &id))
        .await
        .map_err(|error| LibraryCommandError::dialog(error.to_string()))?
}

#[tauri::command]
pub async fn list_libraries(app: AppHandle) -> Result<Vec<Library>, LibraryCommandError> {
    tauri::async_runtime::spawn_blocking(move || list_library_records(&app))
        .await
        .map_err(|error| LibraryCommandError::dialog(error.to_string()))?
}
