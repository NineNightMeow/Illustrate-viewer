use tauri::{AppHandle, State};

use crate::{
    database::{ImageFingerprint, MetadataDatabase},
    fingerprint::{DuplicateCandidatePage, DuplicateIndicator, FingerprintError},
};

#[tauri::command]
pub async fn get_image_fingerprint(
    app: AppHandle,
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<Option<ImageFingerprint>, FingerprintError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::fingerprint::get_cached_image_fingerprint(&app, &database, &image_id)
    })
    .await
    .map_err(|error| {
        FingerprintError::operation(format!("Fingerprint task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn analyze_image_fingerprint(
    app: AppHandle,
    database: State<'_, MetadataDatabase>,
    image_id: String,
) -> Result<ImageFingerprint, FingerprintError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::fingerprint::analyze_image_fingerprint(&app, &database, &image_id)
    })
    .await
    .map_err(|error| {
        FingerprintError::operation(format!("Fingerprint task could not complete: {error}"))
    })?
}

#[tauri::command]
pub async fn list_duplicate_candidates(
    app: AppHandle,
    database: State<'_, MetadataDatabase>,
    image_id: String,
    offset: u32,
    limit: u16,
) -> Result<DuplicateCandidatePage, FingerprintError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::fingerprint::list_duplicate_candidates(&app, &database, &image_id, offset, limit)
    })
    .await
    .map_err(|error| {
        FingerprintError::operation(format!(
            "Duplicate candidate task could not complete: {error}"
        ))
    })?
}

#[tauri::command]
pub async fn get_duplicate_indicators(
    database: State<'_, MetadataDatabase>,
    image_ids: Vec<String>,
) -> Result<Vec<DuplicateIndicator>, FingerprintError> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::fingerprint::get_cached_duplicate_indicators(&database, &image_ids)
    })
    .await
    .map_err(|error| {
        FingerprintError::operation(format!(
            "Duplicate indicator task could not complete: {error}"
        ))
    })?
}
