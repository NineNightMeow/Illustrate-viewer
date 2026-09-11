use tauri::{AppHandle, State};

use crate::scanner::{scan_library_record, ScanRegistry, ScanResult, ScannerCommandError};

#[tauri::command]
pub async fn scan_library(
    app: AppHandle,
    scanner: State<'_, ScanRegistry>,
    library_id: String,
) -> Result<ScanResult, ScannerCommandError> {
    scanner.begin(&library_id)?;
    let scan_id = library_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || scan_library_record(&app, &scan_id))
        .await
        .map_err(|error| ScannerCommandError::scan_failed(error.to_string()));
    scanner.finish(&library_id);
    result.and_then(|scan| scan)
}
