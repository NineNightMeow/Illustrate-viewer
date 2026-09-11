mod geometry;
mod persistence;

use std::{
    collections::HashMap,
    path::{Component, Path},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use image::ImageReader;
use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use uuid::Uuid;

use crate::{
    library::find_library_record,
    thumbnail::{prepare_image_asset, ThumbnailRequest},
};
use geometry::{capture_geometry, initial_window_size, restore_geometry, MonitorWorkArea};
use persistence::{geometry_is_valid, PersistedReference, ReferenceGeometry, ReferenceWindowState};

const REFERENCE_WINDOW_WIDTH: f64 = 720.0;
const REFERENCE_WINDOW_HEIGHT: f64 = 540.0;
const MAX_ACTIVE_REFERENCE_WINDOWS: usize = 10;
const GEOMETRY_SAVE_DEBOUNCE: Duration = Duration::from_millis(350);

#[derive(Default)]
pub struct ReferenceWindowRegistry {
    // ponytail: one process-wide lock keeps create/focus atomic; partition only if opening windows ever profiles as a bottleneck.
    state: Mutex<RegistryState>,
    save_state: Mutex<SaveState>,
    is_shutting_down: AtomicBool,
}

#[derive(Default)]
struct RegistryState {
    by_image_id: HashMap<String, ReferenceWindowEntry>,
    image_id_by_window_label: HashMap<String, String>,
    records_by_reference_id: HashMap<String, PersistedReference>,
}

#[derive(Default)]
struct SaveState {
    last_request: Option<Instant>,
    worker_running: bool,
}

#[derive(Clone, Debug)]
struct ReferenceWindowEntry {
    reference_id: String,
    image_id: String,
    library_id: String,
    relative_path: String,
    window_label: String,
    filename: String,
    source_path: String,
    geometry: Option<ReferenceGeometry>,
    state: ReferenceWindowState,
    runtime: ReferenceRuntimeState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReferenceRuntimeState {
    is_creating: bool,
    is_visible: bool,
    is_focused: bool,
}

impl Default for ReferenceRuntimeState {
    fn default() -> Self {
        Self {
            is_creating: false,
            is_visible: true,
            is_focused: false,
        }
    }
}

impl ReferenceRuntimeState {
    fn creating() -> Self {
        Self {
            is_creating: true,
            ..Self::default()
        }
    }
}

impl ReferenceWindowEntry {
    fn persisted(&self) -> PersistedReference {
        PersistedReference {
            reference_id: self.reference_id.clone(),
            image_id: self.image_id.clone(),
            library_id: self.library_id.clone(),
            relative_path: self.relative_path.clone(),
            geometry: self.geometry.clone(),
            state: self.state.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenReferenceWindowResult {
    pub reference_id: String,
    pub window_label: String,
    pub was_existing: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceAsset {
    pub image_id: String,
    pub filename: String,
    pub source_path: String,
    pub state: ReferenceWindowState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReferenceWindowBatchResult {
    pub affected: usize,
    pub stale_removed: usize,
}

enum ReferenceWindowReservation {
    Existing(ReferenceWindowEntry),
    Reserved(ReferenceWindowEntry),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCommandError {
    pub code: &'static str,
    pub message: String,
}

impl ReferenceCommandError {
    fn source_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "source_unavailable",
            message: message.into(),
        }
    }

    fn window_creation_failed(message: impl Into<String>) -> Self {
        Self {
            code: "window_creation_failed",
            message: message.into(),
        }
    }

    fn reference_unavailable() -> Self {
        Self {
            code: "reference_unavailable",
            message: "This reference window is no longer active.".into(),
        }
    }

    fn invalid_state() -> Self {
        Self {
            code: "invalid_state",
            message: "The reference window state is invalid.".into(),
        }
    }

    fn reference_limit_reached() -> Self {
        Self {
            code: "reference_limit_reached",
            message: format!(
                "A maximum of {MAX_ACTIVE_REFERENCE_WINDOWS} reference windows can be open."
            ),
        }
    }

    fn persistence_failed(message: impl Into<String>) -> Self {
        Self {
            code: "persistence_failed",
            message: message.into(),
        }
    }
}

#[tauri::command]
pub async fn open_reference_window(
    app: AppHandle,
    registry: State<'_, ReferenceWindowRegistry>,
    asset: ThumbnailRequest,
) -> Result<OpenReferenceWindowResult, ReferenceCommandError> {
    cleanup_stale_references(&app, &registry);
    if let Some(result) = focus_existing_window(&app, &registry, &asset.image_id) {
        return result;
    }
    if registry.is_at_capacity() {
        return Err(ReferenceCommandError::reference_limit_reached());
    }

    let relative_path = relative_path_from_image_id(&asset.image_id, &asset.library_id)?;
    let source_path = prepare_source_path(&app, asset.clone()).await?;

    loop {
        let reservation = {
            let mut state = registry
                .state
                .lock()
                .expect("reference registry lock poisoned");
            match state.entry_by_image_id(&asset.image_id).cloned() {
                Some(entry) => ReferenceWindowReservation::Existing(entry),
                None => ReferenceWindowReservation::Reserved(
                    state
                        .reserve_new(&asset, &relative_path, &source_path)
                        .ok_or_else(ReferenceCommandError::reference_limit_reached)?,
                ),
            }
        };

        match reservation {
            ReferenceWindowReservation::Existing(entry) => {
                if let Some(result) = focus_reference_entry(&app, &registry, entry) {
                    return result;
                }
            }
            ReferenceWindowReservation::Reserved(entry) => {
                let window = match build_reference_window(&app, &entry) {
                    Ok(window) => window,
                    Err(error) => {
                        registry.remove_window_label(&entry.window_label);
                        return Err(ReferenceCommandError::window_creation_failed(
                            error.to_string(),
                        ));
                    }
                };
                registry.complete_window_creation(
                    &entry.window_label,
                    capture_webview_geometry(&window),
                );
                let _ = flush_references(&app, &registry);
                return Ok(OpenReferenceWindowResult {
                    reference_id: entry.reference_id,
                    window_label: entry.window_label,
                    was_existing: false,
                });
            }
        }
    }
}

#[tauri::command]
pub fn get_reference_asset(
    window: WebviewWindow,
    registry: State<'_, ReferenceWindowRegistry>,
) -> Result<ReferenceAsset, ReferenceCommandError> {
    let state = registry
        .state
        .lock()
        .expect("reference registry lock poisoned");
    let entry = state
        .entry_by_window_label(window.label())
        .ok_or_else(ReferenceCommandError::reference_unavailable)?;

    Ok(ReferenceAsset {
        image_id: entry.image_id.clone(),
        filename: entry.filename.clone(),
        source_path: entry.source_path.clone(),
        state: entry.state.clone(),
    })
}

#[tauri::command]
pub fn update_reference_state(
    app: AppHandle,
    window: WebviewWindow,
    registry: State<'_, ReferenceWindowRegistry>,
    reference_state: ReferenceWindowState,
) -> Result<(), ReferenceCommandError> {
    if !(20..=100).contains(&reference_state.opacity) {
        return Err(ReferenceCommandError::invalid_state());
    }

    let updated = registry
        .state
        .lock()
        .expect("reference registry lock poisoned")
        .update_window_state(window.label(), reference_state);
    if !updated {
        return Err(ReferenceCommandError::reference_unavailable());
    }

    flush_references(&app, &registry)
}

pub fn restore_references(app: &AppHandle, registry: &ReferenceWindowRegistry) {
    let Ok(records) = persistence::load(app) else {
        return;
    };
    registry.install_restored_records(records.clone());

    for record in records {
        if registry.is_at_capacity() {
            break;
        }
        let _ = restore_reference(app, registry, &record);
    }
}

pub fn update_reference_focus(
    window: &tauri::Window,
    registry: &ReferenceWindowRegistry,
    is_focused: bool,
) {
    registry
        .state
        .lock()
        .expect("reference registry lock poisoned")
        .update_window_focus(window.label(), is_focused);
}

pub fn cleanup_stale_references(app: &AppHandle, registry: &ReferenceWindowRegistry) -> usize {
    if registry.is_shutting_down.load(Ordering::Acquire) {
        return 0;
    }

    let stale_labels: Vec<_> = registry
        .active_window_labels()
        .into_iter()
        .filter(|window_label| app.get_webview_window(window_label).is_none())
        .collect();
    let removed = registry.remove_stale_window_labels(&stale_labels);
    if removed > 0 {
        let _ = flush_references(app, registry);
    }
    removed
}

pub fn hide_all_references(
    app: &AppHandle,
    registry: &ReferenceWindowRegistry,
) -> ReferenceWindowBatchResult {
    set_all_reference_visibility(app, registry, false)
}

pub fn show_all_references(
    app: &AppHandle,
    registry: &ReferenceWindowRegistry,
) -> ReferenceWindowBatchResult {
    set_all_reference_visibility(app, registry, true)
}

pub fn update_reference_geometry(window: &tauri::Window, registry: &ReferenceWindowRegistry) {
    if registry.is_shutting_down.load(Ordering::Acquire) {
        return;
    }

    let updated = registry
        .state
        .lock()
        .expect("reference registry lock poisoned")
        .update_geometry(window.label(), capture_window_geometry(window));
    if updated {
        request_geometry_save(window.app_handle().clone(), registry);
    }
}

pub fn remove_destroyed_reference(window: &tauri::Window, registry: &ReferenceWindowRegistry) {
    if registry.is_shutting_down.load(Ordering::Acquire) {
        return;
    }

    let removed = registry
        .state
        .lock()
        .expect("reference registry lock poisoned")
        .remove_by_window_label(window.label());
    if removed.is_some() {
        let _ = flush_references(window.app_handle(), registry);
    }
}

pub fn begin_shutdown(registry: &ReferenceWindowRegistry) {
    registry.is_shutting_down.store(true, Ordering::Release);
}

pub fn flush_references(
    app: &AppHandle,
    registry: &ReferenceWindowRegistry,
) -> Result<(), ReferenceCommandError> {
    let records = registry.persisted_records();
    persistence::save(app, &records).map_err(ReferenceCommandError::persistence_failed)
}

fn request_geometry_save(app: AppHandle, registry: &ReferenceWindowRegistry) {
    let should_start_worker = {
        let mut save_state = registry
            .save_state
            .lock()
            .expect("reference save state lock poisoned");
        save_state.last_request = Some(Instant::now());
        if save_state.worker_running {
            false
        } else {
            save_state.worker_running = true;
            true
        }
    };
    if !should_start_worker {
        return;
    }

    thread::spawn(move || loop {
        thread::sleep(GEOMETRY_SAVE_DEBOUNCE);
        let registry = app.state::<ReferenceWindowRegistry>();
        if registry.is_shutting_down.load(Ordering::Acquire) {
            return;
        }

        let should_flush = {
            let mut save_state = registry
                .save_state
                .lock()
                .expect("reference save state lock poisoned");
            if save_state
                .last_request
                .is_some_and(|last_request| last_request.elapsed() >= GEOMETRY_SAVE_DEBOUNCE)
            {
                save_state.worker_running = false;
                true
            } else {
                false
            }
        };
        if should_flush {
            let _ = flush_references(&app, &registry);
            return;
        }
    });
}

async fn prepare_source_path(
    app: &AppHandle,
    asset: ThumbnailRequest,
) -> Result<String, ReferenceCommandError> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || prepare_image_asset(&app, &asset))
        .await
        .map_err(|error| ReferenceCommandError::source_unavailable(error.to_string()))?
        .map_err(|error| ReferenceCommandError::source_unavailable(error.message))
}

fn restore_reference(
    app: &AppHandle,
    registry: &ReferenceWindowRegistry,
    record: &PersistedReference,
) -> Result<(), ReferenceCommandError> {
    if !persisted_record_is_valid(record) {
        return Err(ReferenceCommandError::invalid_state());
    }
    if registry.has_active_image(&record.image_id) {
        return Ok(());
    }
    if registry.is_at_capacity() {
        return Err(ReferenceCommandError::reference_limit_reached());
    }

    let library = find_library_record(app, &record.library_id)
        .map_err(|error| ReferenceCommandError::source_unavailable(error.message))?
        .ok_or_else(|| {
            ReferenceCommandError::source_unavailable("The reference library is unavailable.")
        })?;
    let source_path = Path::new(&library.path).join(&record.relative_path);
    let request = ThumbnailRequest {
        image_id: record.image_id.clone(),
        library_id: record.library_id.clone(),
        source_path: source_path.to_string_lossy().into_owned(),
    };
    let source_path = prepare_image_asset(app, &request)
        .map_err(|error| ReferenceCommandError::source_unavailable(error.message))?;

    let entry = {
        let mut state = registry
            .state
            .lock()
            .expect("reference registry lock poisoned");
        if state.entry_by_image_id(&record.image_id).is_some() {
            return Ok(());
        }
        state
            .reserve_restored(record, &source_path)
            .ok_or_else(ReferenceCommandError::reference_limit_reached)?
    };
    let window = match build_reference_window(app, &entry) {
        Ok(window) => window,
        Err(error) => {
            registry.remove_window_label(&entry.window_label);
            return Err(ReferenceCommandError::window_creation_failed(
                error.to_string(),
            ));
        }
    };
    registry.complete_window_creation(&entry.window_label, capture_webview_geometry(&window));
    Ok(())
}

fn build_reference_window(
    app: &AppHandle,
    entry: &ReferenceWindowEntry,
) -> tauri::Result<WebviewWindow> {
    let (initial_width, initial_height) = entry
        .geometry
        .as_ref()
        .filter(|geometry| geometry_is_valid(geometry))
        .map(|_| (REFERENCE_WINDOW_WIDTH, REFERENCE_WINDOW_HEIGHT))
        .unwrap_or_else(|| initial_reference_window_size(app, entry));
    let window = WebviewWindowBuilder::new(
        app,
        &entry.window_label,
        WebviewUrl::App("reference.html".into()),
    )
    .title(&entry.filename)
    .decorations(false)
    .resizable(!entry.state.locked)
    .always_on_top(entry.state.always_on_top)
    .inner_size(initial_width, initial_height)
    .min_inner_size(160.0, 120.0)
    .center()
    .build()?;

    if let Some(geometry) = entry
        .geometry
        .as_ref()
        .filter(|geometry| geometry_is_valid(geometry))
    {
        if let Some(restored) = restore_window_geometry(app, geometry) {
            // Tauri's builder accepts logical desktop coordinates. Restore with physical
            // values after creation so a mixed-DPI virtual desktop never mixes units.
            let _ = window.set_size(tauri::PhysicalSize::new(restored.width, restored.height));
            let _ = window.set_position(tauri::PhysicalPosition::new(restored.x, restored.y));
        }
    }

    Ok(window)
}

fn initial_reference_window_size(app: &AppHandle, entry: &ReferenceWindowEntry) -> (f64, f64) {
    let monitor = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_work_area(&monitor))
        .or_else(|| {
            app.available_monitors()
                .ok()
                .and_then(|monitors| monitors.first().map(monitor_work_area))
        });
    let Some(monitor) = monitor.as_ref() else {
        return (REFERENCE_WINDOW_WIDTH, REFERENCE_WINDOW_HEIGHT);
    };
    let (image_width, image_height) = image_dimensions(app, entry).unwrap_or((
        REFERENCE_WINDOW_WIDTH as u32,
        REFERENCE_WINDOW_HEIGHT as u32,
    ));
    initial_window_size(image_width, image_height, monitor)
}

fn image_dimensions(app: &AppHandle, entry: &ReferenceWindowEntry) -> Option<(u32, u32)> {
    if let Some(metadata) = app
        .try_state::<crate::database::MetadataDatabase>()
        .and_then(|database| database.get_image_metadata(&entry.image_id).ok().flatten())
        .filter(|metadata| metadata.status == "ready")
    {
        if let (Some(width), Some(height)) = (metadata.width, metadata.height) {
            if width > 0 && height > 0 {
                return Some((width, height));
            }
        }
    }

    ImageReader::open(&entry.source_path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
        .filter(|(width, height)| *width > 0 && *height > 0)
}

fn focus_existing_window(
    app: &AppHandle,
    registry: &ReferenceWindowRegistry,
    image_id: &str,
) -> Option<Result<OpenReferenceWindowResult, ReferenceCommandError>> {
    let entry = registry.entry_by_image_id(image_id)?;
    focus_reference_entry(app, registry, entry)
}

fn focus_reference_entry(
    app: &AppHandle,
    registry: &ReferenceWindowRegistry,
    entry: ReferenceWindowEntry,
) -> Option<Result<OpenReferenceWindowResult, ReferenceCommandError>> {
    if entry.runtime.is_creating {
        return Some(Ok(OpenReferenceWindowResult {
            reference_id: entry.reference_id,
            window_label: entry.window_label,
            was_existing: true,
        }));
    }

    let Some(window) = app.get_webview_window(&entry.window_label) else {
        registry.remove_window_label(&entry.window_label);
        return None;
    };

    if !entry.runtime.is_visible || !window.is_visible().unwrap_or(true) {
        if window.show().is_err() {
            registry.remove_window_label(&entry.window_label);
            return None;
        }
        registry.update_window_visibility(&entry.window_label, true);
    }

    match window.set_focus() {
        Ok(()) => {
            registry.update_window_focus(&entry.window_label, true);
            Some(Ok(OpenReferenceWindowResult {
                reference_id: entry.reference_id,
                window_label: entry.window_label,
                was_existing: true,
            }))
        }
        Err(_) => {
            registry.remove_window_label(&entry.window_label);
            None
        }
    }
}

fn set_all_reference_visibility(
    app: &AppHandle,
    registry: &ReferenceWindowRegistry,
    is_visible: bool,
) -> ReferenceWindowBatchResult {
    let stale_removed = cleanup_stale_references(app, registry);
    let labels = registry.window_labels_with_visibility(is_visible);
    let mut affected = 0;
    let mut stale_labels = Vec::new();

    for window_label in labels {
        let Some(window) = app.get_webview_window(&window_label) else {
            stale_labels.push(window_label);
            continue;
        };
        let operation = if is_visible {
            window.show()
        } else {
            window.hide()
        };
        if operation.is_ok() {
            registry.update_window_visibility(&window_label, is_visible);
            affected += 1;
        } else {
            stale_labels.push(window_label);
        }
    }

    let newly_removed = registry.remove_stale_window_labels(&stale_labels);
    if newly_removed > 0 {
        let _ = flush_references(app, registry);
    }
    ReferenceWindowBatchResult {
        affected,
        stale_removed: stale_removed + newly_removed,
    }
}

fn capture_window_geometry(window: &tauri::Window) -> Option<ReferenceGeometry> {
    let monitor = window.current_monitor().ok()??;
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    capture_geometry(
        &monitor_work_area(&monitor),
        position.x,
        position.y,
        size.width,
        size.height,
    )
}

fn capture_webview_geometry(window: &WebviewWindow) -> Option<ReferenceGeometry> {
    let monitor = window.current_monitor().ok()??;
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    capture_geometry(
        &monitor_work_area(&monitor),
        position.x,
        position.y,
        size.width,
        size.height,
    )
}

fn restore_window_geometry(
    app: &AppHandle,
    geometry: &ReferenceGeometry,
) -> Option<geometry::PhysicalWindowGeometry> {
    let monitors: Vec<_> = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(monitor_work_area)
        .collect();
    let primary_monitor = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_work_area(&monitor));
    restore_geometry(geometry, &monitors, primary_monitor.as_ref())
}

fn monitor_work_area(monitor: &tauri::Monitor) -> MonitorWorkArea {
    let position = monitor.position();
    let size = monitor.size();
    let work_area = monitor.work_area();
    MonitorWorkArea {
        name: monitor.name().cloned(),
        position_x: position.x,
        position_y: position.y,
        width: size.width,
        height: size.height,
        work_x: work_area.position.x,
        work_y: work_area.position.y,
        work_width: work_area.size.width,
        work_height: work_area.size.height,
        scale_factor: monitor.scale_factor(),
    }
}

fn relative_path_from_image_id(
    image_id: &str,
    library_id: &str,
) -> Result<String, ReferenceCommandError> {
    let prefix = format!("{library_id}:");
    let relative_path = image_id
        .strip_prefix(&prefix)
        .filter(|path| relative_path_is_safe(path))
        .ok_or_else(ReferenceCommandError::invalid_state)?;
    Ok(relative_path.to_string())
}

fn relative_path_is_safe(relative_path: &str) -> bool {
    !relative_path.is_empty()
        && Path::new(relative_path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn persisted_record_is_valid(record: &PersistedReference) -> bool {
    Uuid::parse_str(&record.reference_id).is_ok()
        && !record.library_id.is_empty()
        && relative_path_from_image_id(&record.image_id, &record.library_id)
            .map(|relative_path| {
                relative_path == record.relative_path
                    && relative_path_is_safe(&record.relative_path)
            })
            .unwrap_or(false)
}

impl ReferenceWindowRegistry {
    fn install_restored_records(&self, records: Vec<PersistedReference>) {
        let mut state = self.state.lock().expect("reference registry lock poisoned");
        for mut record in records {
            if !persisted_record_is_valid(&record)
                || state.by_image_id.contains_key(&record.image_id)
            {
                continue;
            }
            record.geometry = record.geometry.filter(geometry_is_valid);
            record.state.opacity = record.state.opacity.clamp(20, 100);
            state
                .records_by_reference_id
                .entry(record.reference_id.clone())
                .or_insert(record);
        }
    }

    fn has_active_image(&self, image_id: &str) -> bool {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .by_image_id
            .contains_key(image_id)
    }

    fn entry_by_image_id(&self, image_id: &str) -> Option<ReferenceWindowEntry> {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .entry_by_image_id(image_id)
            .cloned()
    }

    fn is_at_capacity(&self) -> bool {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .is_at_capacity()
    }

    fn active_window_labels(&self) -> Vec<String> {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .active_window_labels()
    }

    fn window_labels_with_visibility(&self, is_visible: bool) -> Vec<String> {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .window_labels_with_visibility(is_visible)
    }

    fn update_window_visibility(&self, window_label: &str, is_visible: bool) {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .update_window_visibility(window_label, is_visible);
    }

    fn update_window_focus(&self, window_label: &str, is_focused: bool) {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .update_window_focus(window_label, is_focused);
    }

    fn complete_window_creation(&self, window_label: &str, geometry: Option<ReferenceGeometry>) {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .complete_window_creation(window_label, geometry);
    }

    fn remove_window_label(&self, window_label: &str) -> bool {
        self.state
            .lock()
            .expect("reference registry lock poisoned")
            .remove_by_window_label(window_label)
            .is_some()
    }

    fn remove_stale_window_labels(&self, window_labels: &[String]) -> usize {
        let mut state = self.state.lock().expect("reference registry lock poisoned");
        window_labels
            .iter()
            .filter(|window_label| state.remove_by_window_label(window_label).is_some())
            .count()
    }

    fn persisted_records(&self) -> Vec<PersistedReference> {
        let state = self.state.lock().expect("reference registry lock poisoned");
        let mut records: Vec<_> = state.records_by_reference_id.values().cloned().collect();
        records.sort_by(|left, right| left.reference_id.cmp(&right.reference_id));
        records
    }
}

impl RegistryState {
    fn reserve_new(
        &mut self,
        asset: &ThumbnailRequest,
        relative_path: &str,
        source_path: &str,
    ) -> Option<ReferenceWindowEntry> {
        if self.is_at_capacity() {
            return None;
        }
        let existing_record = self
            .records_by_reference_id
            .values()
            .find(|record| record.image_id == asset.image_id)
            .cloned();
        let reference_id = existing_record
            .as_ref()
            .map(|record| record.reference_id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let entry = ReferenceWindowEntry {
            window_label: format!("reference-{reference_id}"),
            reference_id,
            image_id: asset.image_id.clone(),
            library_id: asset.library_id.clone(),
            relative_path: relative_path.to_string(),
            filename: filename_from_path(source_path),
            source_path: source_path.to_string(),
            geometry: existing_record
                .as_ref()
                .and_then(|record| record.geometry.clone())
                .filter(geometry_is_valid),
            state: existing_record
                .map(|record| record.state)
                .unwrap_or_default(),
            runtime: ReferenceRuntimeState::creating(),
        };
        self.insert_entry(entry.clone());
        Some(entry)
    }

    fn reserve_restored(
        &mut self,
        record: &PersistedReference,
        source_path: &str,
    ) -> Option<ReferenceWindowEntry> {
        if self.is_at_capacity() {
            return None;
        }
        let entry = ReferenceWindowEntry {
            window_label: format!("reference-{}", record.reference_id),
            reference_id: record.reference_id.clone(),
            image_id: record.image_id.clone(),
            library_id: record.library_id.clone(),
            relative_path: record.relative_path.clone(),
            filename: filename_from_path(source_path),
            source_path: source_path.to_string(),
            geometry: record.geometry.clone().filter(geometry_is_valid),
            state: record.state.clone(),
            runtime: ReferenceRuntimeState::creating(),
        };
        self.insert_entry(entry.clone());
        Some(entry)
    }

    fn insert_entry(&mut self, entry: ReferenceWindowEntry) {
        self.image_id_by_window_label
            .insert(entry.window_label.clone(), entry.image_id.clone());
        self.by_image_id
            .insert(entry.image_id.clone(), entry.clone());
        self.records_by_reference_id
            .insert(entry.reference_id.clone(), entry.persisted());
    }

    fn entry_by_image_id(&self, image_id: &str) -> Option<&ReferenceWindowEntry> {
        self.by_image_id.get(image_id)
    }

    fn entry_by_window_label(&self, window_label: &str) -> Option<&ReferenceWindowEntry> {
        let image_id = self.image_id_by_window_label.get(window_label)?;
        self.by_image_id.get(image_id)
    }

    fn is_at_capacity(&self) -> bool {
        self.by_image_id.len() >= MAX_ACTIVE_REFERENCE_WINDOWS
    }

    fn active_window_labels(&self) -> Vec<String> {
        self.by_image_id
            .values()
            .filter(|entry| !entry.runtime.is_creating)
            .map(|entry| entry.window_label.clone())
            .collect()
    }

    fn window_labels_with_visibility(&self, is_visible: bool) -> Vec<String> {
        self.by_image_id
            .values()
            .filter(|entry| !entry.runtime.is_creating && entry.runtime.is_visible != is_visible)
            .map(|entry| entry.window_label.clone())
            .collect()
    }

    fn update_window_visibility(&mut self, window_label: &str, is_visible: bool) -> bool {
        let Some(entry) = self
            .image_id_by_window_label
            .get(window_label)
            .and_then(|image_id| self.by_image_id.get_mut(image_id))
        else {
            return false;
        };
        entry.runtime.is_visible = is_visible;
        if !is_visible {
            entry.runtime.is_focused = false;
        }
        true
    }

    fn update_window_focus(&mut self, window_label: &str, is_focused: bool) -> bool {
        let Some(entry) = self
            .image_id_by_window_label
            .get(window_label)
            .and_then(|image_id| self.by_image_id.get_mut(image_id))
        else {
            return false;
        };
        if is_focused {
            entry.runtime.is_visible = true;
        }
        entry.runtime.is_focused = is_focused;
        true
    }

    fn complete_window_creation(
        &mut self,
        window_label: &str,
        geometry: Option<ReferenceGeometry>,
    ) -> bool {
        let Some(entry) = self
            .image_id_by_window_label
            .get(window_label)
            .and_then(|image_id| self.by_image_id.get_mut(image_id))
        else {
            return false;
        };
        entry.runtime.is_creating = false;
        entry.runtime.is_visible = true;
        entry.geometry = geometry;
        let record = entry.persisted();
        self.records_by_reference_id
            .insert(record.reference_id.clone(), record);
        true
    }

    fn update_geometry(&mut self, window_label: &str, geometry: Option<ReferenceGeometry>) -> bool {
        let Some(entry) = self
            .image_id_by_window_label
            .get(window_label)
            .and_then(|image_id| self.by_image_id.get_mut(image_id))
        else {
            return false;
        };
        entry.geometry = geometry;
        let record = entry.persisted();
        self.records_by_reference_id
            .insert(record.reference_id.clone(), record);
        true
    }

    fn update_window_state(&mut self, window_label: &str, state: ReferenceWindowState) -> bool {
        let Some(entry) = self
            .image_id_by_window_label
            .get(window_label)
            .and_then(|image_id| self.by_image_id.get_mut(image_id))
        else {
            return false;
        };
        entry.state = state;
        let record = entry.persisted();
        self.records_by_reference_id
            .insert(record.reference_id.clone(), record);
        true
    }

    fn remove_by_window_label(&mut self, window_label: &str) -> Option<ReferenceWindowEntry> {
        let image_id = self.image_id_by_window_label.remove(window_label)?;
        let entry = self.by_image_id.remove(&image_id)?;
        self.records_by_reference_id.remove(&entry.reference_id);
        Some(entry)
    }
}

fn filename_from_path(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("Reference")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        relative_path_from_image_id, relative_path_is_safe, ReferenceWindowState, RegistryState,
    };
    use crate::thumbnail::ThumbnailRequest;

    fn request(image_id: &str) -> ThumbnailRequest {
        ThumbnailRequest {
            image_id: image_id.into(),
            library_id: "library-1".into(),
            source_path: r"C:\library\study.png".into(),
        }
    }

    #[test]
    fn deduplicates_reservations_for_the_same_image() {
        let mut registry = RegistryState::default();
        let first = registry
            .reserve_new(
                &request("library-1:study.png"),
                "study.png",
                r"C:\library\study.png",
            )
            .expect("first reference should reserve");
        let duplicate = registry.entry_by_image_id("library-1:study.png").unwrap();

        assert_eq!(duplicate.reference_id, first.reference_id);
        assert_eq!(duplicate.window_label, first.window_label);
    }

    #[test]
    fn removes_destroyed_entries_so_the_image_can_open_again() {
        let mut registry = RegistryState::default();
        let first = registry
            .reserve_new(
                &request("library-1:study.png"),
                "study.png",
                r"C:\library\study.png",
            )
            .expect("first reference should reserve");

        assert!(registry
            .remove_by_window_label(&first.window_label)
            .is_some());
        assert!(registry.entry_by_image_id("library-1:study.png").is_none());

        let reopened = registry
            .reserve_new(
                &request("library-1:study.png"),
                "study.png",
                r"C:\library\study.png",
            )
            .expect("reopened reference should reserve");
        assert_ne!(reopened.reference_id, first.reference_id);
    }

    #[test]
    fn only_updates_the_calling_reference_state() {
        let mut registry = RegistryState::default();
        let left = registry
            .reserve_new(
                &request("library-1:left.png"),
                "left.png",
                r"C:\library\left.png",
            )
            .expect("left reference should reserve");
        let right = registry
            .reserve_new(
                &request("library-1:right.png"),
                "right.png",
                r"C:\library\right.png",
            )
            .expect("right reference should reserve");

        assert!(registry.update_window_state(
            &left.window_label,
            ReferenceWindowState {
                mirrored: true,
                opacity: 75,
                always_on_top: false,
                locked: true,
            },
        ));
        assert!(registry
            .entry_by_window_label(&right.window_label)
            .is_some_and(|entry| !entry.state.mirrored && !entry.state.locked));
    }

    #[test]
    fn only_accepts_safe_library_relative_paths() {
        assert_eq!(
            relative_path_from_image_id("library-1:folder/study.png", "library-1").unwrap(),
            "folder/study.png",
        );
        assert!(!relative_path_is_safe("../study.png"));
        assert!(relative_path_from_image_id("library-1:../study.png", "library-1").is_err());
    }

    #[test]
    fn bounds_active_references_at_the_stress_target() {
        let mut registry = RegistryState::default();

        for index in 0..super::MAX_ACTIVE_REFERENCE_WINDOWS {
            let image_id = format!("library-1:reference-{index}.png");
            assert!(registry
                .reserve_new(
                    &request(&image_id),
                    &format!("reference-{index}.png"),
                    r"C:\library\study.png"
                )
                .is_some());
        }

        assert!(registry.is_at_capacity());
        assert!(registry
            .reserve_new(
                &request("library-1:overflow.png"),
                "overflow.png",
                r"C:\library\study.png"
            )
            .is_none());
    }

    #[test]
    fn keeps_visibility_and_focus_out_of_persisted_reference_state() {
        let mut registry = RegistryState::default();
        let reference = registry
            .reserve_new(
                &request("library-1:study.png"),
                "study.png",
                r"C:\library\study.png",
            )
            .expect("reference should reserve");

        assert!(registry.update_window_focus(&reference.window_label, true));
        assert!(registry.update_window_visibility(&reference.window_label, false));
        let runtime = &registry
            .entry_by_window_label(&reference.window_label)
            .expect("reference should remain registered")
            .runtime;

        assert!(!runtime.is_visible && !runtime.is_focused);
        assert_eq!(registry.records_by_reference_id.len(), 1);
    }
}
