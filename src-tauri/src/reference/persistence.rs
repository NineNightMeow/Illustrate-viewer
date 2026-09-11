use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const REFERENCES_FILE: &str = "references.json";
const SCHEMA_VERSION: u8 = 2;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedReference {
    pub reference_id: String,
    pub image_id: String,
    pub library_id: String,
    pub relative_path: String,
    pub geometry: Option<ReferenceGeometry>,
    pub state: ReferenceWindowState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceGeometry {
    /// Logical coordinates are relative to `monitor`'s work area, not the virtual desktop.
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale_factor: f64,
    #[serde(default)]
    pub monitor: Option<MonitorIdentity>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorIdentity {
    // Tauri does not expose a stable hardware ID here. Name plus the last physical
    // position and resolution is an explainable signature, with primary-monitor fallback.
    pub name: Option<String>,
    pub position_x: i32,
    pub position_y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceWindowState {
    pub mirrored: bool,
    pub opacity: u8,
    pub always_on_top: bool,
    pub locked: bool,
}

impl Default for ReferenceWindowState {
    fn default() -> Self {
        Self {
            mirrored: false,
            opacity: 100,
            always_on_top: false,
            locked: false,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReferenceFile {
    schema_version: u8,
    references: Vec<PersistedReference>,
}

pub fn load(app: &AppHandle) -> Result<Vec<PersistedReference>, String> {
    let path = data_path(app)?;
    load_from_path(&path)
}

pub fn save(app: &AppHandle, records: &[PersistedReference]) -> Result<(), String> {
    let path = data_path(app)?;
    save_to_path(&path, records)
}

pub fn geometry_is_valid(geometry: &ReferenceGeometry) -> bool {
    geometry.x.is_finite()
        && geometry.y.is_finite()
        && geometry.width.is_finite()
        && geometry.height.is_finite()
        && geometry.scale_factor.is_finite()
        && geometry.width >= 160.0
        && geometry.height >= 120.0
        && geometry.width <= 8_192.0
        && geometry.height <= 8_192.0
        && geometry.x.abs() <= 100_000.0
        && geometry.y.abs() <= 100_000.0
        && geometry.scale_factor > 0.0
        && geometry.scale_factor <= 8.0
        && geometry
            .monitor
            .as_ref()
            .is_none_or(monitor_identity_is_valid)
}

fn monitor_identity_is_valid(monitor: &MonitorIdentity) -> bool {
    monitor.width > 0
        && monitor.height > 0
        && monitor.scale_factor.is_finite()
        && monitor.scale_factor > 0.0
        && monitor.scale_factor <= 8.0
}

fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join(REFERENCES_FILE))
}

fn load_from_path(path: &Path) -> Result<Vec<PersistedReference>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read(path).map_err(|error| error.to_string())?;
    let file: ReferenceFile =
        serde_json::from_slice(&content).map_err(|error| error.to_string())?;
    if !(1..=SCHEMA_VERSION).contains(&file.schema_version) {
        return Err("The reference window data uses an unsupported schema.".into());
    }

    Ok(file.references)
}

fn save_to_path(path: &Path, records: &[PersistedReference]) -> Result<(), String> {
    let file = ReferenceFile {
        schema_version: SCHEMA_VERSION,
        references: records.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&file).map_err(|error| error.to_string())?;
    serde_json::from_slice::<ReferenceFile>(&bytes).map_err(|error| error.to_string())?;

    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    let mut output = File::create(&temporary).map_err(|error| error.to_string())?;
    output
        .write_all(&bytes)
        .map_err(|error| error.to_string())?;
    output.sync_all().map_err(|error| error.to_string())?;

    if path.exists() {
        fs::copy(path, &backup).map_err(|error| error.to_string())?;
    }
    replace_file(&temporary, path)
}

fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    if fs::rename(temporary, destination).is_ok() {
        return Ok(());
    }

    let replacement = destination.with_extension("json.replace");
    if replacement.exists() {
        fs::remove_file(&replacement).map_err(|error| error.to_string())?;
    }
    if destination.exists() {
        fs::rename(destination, &replacement).map_err(|error| error.to_string())?;
    }

    if let Err(error) = fs::rename(temporary, destination) {
        if replacement.exists() {
            let _ = fs::rename(&replacement, destination);
        }
        return Err(error.to_string());
    }

    if replacement.exists() {
        fs::remove_file(replacement).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::{
        load_from_path, save_to_path, MonitorIdentity, PersistedReference, ReferenceGeometry,
        ReferenceWindowState,
    };
    use uuid::Uuid;

    #[test]
    fn safely_replaces_a_reference_file() {
        let root = env::temp_dir().join(format!("illustrate-viewer-references-{}", Uuid::new_v4()));
        fs::create_dir(&root).expect("temporary directory should be created");
        let path = root.join("references.json");
        let record = PersistedReference {
            reference_id: Uuid::new_v4().to_string(),
            image_id: "library-1:study.png".into(),
            library_id: "library-1".into(),
            relative_path: "study.png".into(),
            geometry: Some(ReferenceGeometry {
                x: 20.0,
                y: 30.0,
                width: 720.0,
                height: 540.0,
                scale_factor: 1.0,
                monitor: Some(MonitorIdentity {
                    name: Some("Display A".into()),
                    position_x: 0,
                    position_y: 0,
                    width: 1_920,
                    height: 1_080,
                    scale_factor: 1.0,
                }),
            }),
            state: ReferenceWindowState::default(),
        };

        save_to_path(&path, &[record.clone()]).expect("reference data should save");
        assert_eq!(
            load_from_path(&path)
                .expect("reference data should load")
                .len(),
            1
        );

        save_to_path(&path, &[]).expect("existing reference data should replace");
        assert!(load_from_path(&path)
            .expect("replacement reference data should load")
            .is_empty());
        assert!(path.with_extension("json.bak").exists());

        fs::remove_dir_all(root).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_invalid_geometry() {
        assert!(!super::geometry_is_valid(&ReferenceGeometry {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 540.0,
            scale_factor: 1.0,
            monitor: None,
        }));
    }

    #[test]
    fn loads_m4_5_geometry_without_a_monitor_identity() {
        let root = env::temp_dir().join(format!("illustrate-viewer-references-{}", Uuid::new_v4()));
        fs::create_dir(&root).expect("temporary directory should be created");
        let path = root.join("references.json");
        fs::write(
            &path,
            r#"{
                "schemaVersion": 1,
                "references": [{
                    "referenceId": "f9dcf4d8-8703-4f3f-962b-f0e2589263db",
                    "imageId": "library-1:study.png",
                    "libraryId": "library-1",
                    "relativePath": "study.png",
                    "geometry": { "x": 20.0, "y": 30.0, "width": 720.0, "height": 540.0, "scaleFactor": 1.0 },
                    "state": { "mirrored": false, "opacity": 100, "alwaysOnTop": false, "locked": false }
                }]
            }"#,
        )
        .expect("legacy data should write");

        let records = load_from_path(&path).expect("legacy data should load");

        assert_eq!(records.len(), 1);
        assert!(records[0]
            .geometry
            .as_ref()
            .is_some_and(|geometry| geometry.monitor.is_none()));
        fs::remove_dir_all(root).expect("temporary directory should be removed");
    }
}
