use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

use crate::database::MetadataDatabase;

const STORE_FILE: &str = "libraries.json";
const STORE_KEY_SCHEMA_VERSION: &str = "schemaVersion";
const STORE_KEY_LIBRARIES: &str = "libraries";
const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub image_count: Option<u32>,
    pub cover: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSelection {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCommandError {
    pub code: &'static str,
    pub message: String,
}

impl LibraryCommandError {
    pub fn dialog(message: String) -> Self {
        Self {
            code: "dialog_failed",
            message,
        }
    }

    fn invalid_path(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_path",
            message: message.into(),
        }
    }

    fn duplicate_path() -> Self {
        Self {
            code: "duplicate_path",
            message: "This folder is already in the library list.".into(),
        }
    }

    fn persistence(message: impl Into<String>) -> Self {
        Self {
            code: "persistence_failed",
            message: message.into(),
        }
    }
}

pub fn folder_selection(path: &Path) -> Result<FolderSelection, LibraryCommandError> {
    let (name, path) = validate_folder(path)?;
    Ok(FolderSelection { name, path })
}

pub fn create_library_record(app: &AppHandle, path: &str) -> Result<Library, LibraryCommandError> {
    let selection = folder_selection(Path::new(path))?;
    let mut libraries = list_library_records(app)?;

    if libraries
        .iter()
        .any(|library| same_library_path(&library.path, &selection.path))
    {
        return Err(LibraryCommandError::duplicate_path());
    }

    let timestamp = current_timestamp()?;
    let library = Library {
        id: Uuid::new_v4().to_string(),
        name: selection.name,
        path: selection.path,
        created_at: timestamp,
        updated_at: timestamp,
        image_count: None,
        cover: None,
    };

    libraries.push(library.clone());
    save_library_records(app, &libraries)?;
    app.state::<MetadataDatabase>()
        .sync_library(&library)
        .map_err(|error| LibraryCommandError::persistence(error.message))?;
    Ok(library)
}

pub fn remove_library_record(app: &AppHandle, id: &str) -> Result<(), LibraryCommandError> {
    let mut libraries = list_library_records(app)?;
    let original_length = libraries.len();
    libraries.retain(|library| library.id != id);

    if libraries.len() != original_length {
        save_library_records(app, &libraries)?;
        app.state::<MetadataDatabase>()
            .remove_library(id)
            .map_err(|error| LibraryCommandError::persistence(error.message))?;
    }

    Ok(())
}

pub fn find_library_record(
    app: &AppHandle,
    id: &str,
) -> Result<Option<Library>, LibraryCommandError> {
    Ok(list_library_records(app)?
        .into_iter()
        .find(|library| library.id == id))
}

pub fn update_library_image_count(
    app: &AppHandle,
    id: &str,
    image_count: Option<u32>,
) -> Result<Option<Library>, LibraryCommandError> {
    let mut libraries = list_library_records(app)?;
    let Some(library) = libraries.iter_mut().find(|library| library.id == id) else {
        return Ok(None);
    };

    library.image_count = image_count;
    library.updated_at = current_timestamp()?;
    let updated = library.clone();
    save_library_records(app, &libraries)?;
    app.state::<MetadataDatabase>()
        .sync_library(&updated)
        .map_err(|error| LibraryCommandError::persistence(error.message))?;
    Ok(Some(updated))
}

pub fn list_library_records(app: &AppHandle) -> Result<Vec<Library>, LibraryCommandError> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| LibraryCommandError::persistence(error.to_string()))?;
    let mut libraries: Vec<Library> = store
        .get(STORE_KEY_LIBRARIES)
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|error| LibraryCommandError::persistence(error.to_string()))
        })
        .transpose()?
        .unwrap_or_default();

    libraries.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(libraries)
}

fn save_library_records(app: &AppHandle, libraries: &[Library]) -> Result<(), LibraryCommandError> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| LibraryCommandError::persistence(error.to_string()))?;
    store.set(STORE_KEY_SCHEMA_VERSION, json!(SCHEMA_VERSION));
    store.set(
        STORE_KEY_LIBRARIES,
        serde_json::to_value(libraries)
            .map_err(|error| LibraryCommandError::persistence(error.to_string()))?,
    );
    store
        .save()
        .map_err(|error| LibraryCommandError::persistence(error.to_string()))
}

fn validate_folder(path: &Path) -> Result<(String, String), LibraryCommandError> {
    let metadata = fs::metadata(path)
        .map_err(|_| LibraryCommandError::invalid_path("The selected folder does not exist."))?;

    if !metadata.is_dir() {
        return Err(LibraryCommandError::invalid_path(
            "The selected path is not a folder.",
        ));
    }

    let canonical_path = fs::canonicalize(path)
        .map_err(|_| LibraryCommandError::invalid_path("The selected folder is unavailable."))?;
    let name = canonical_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| LibraryCommandError::invalid_path("A drive root cannot be a library."))?;

    Ok((name, canonical_path.to_string_lossy().into_owned()))
}

#[cfg(target_os = "windows")]
fn same_library_path(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

#[cfg(not(target_os = "windows"))]
fn same_library_path(left: &str, right: &str) -> bool {
    left == right
}

fn current_timestamp() -> Result<u64, LibraryCommandError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|_| LibraryCommandError::persistence("The system clock is invalid."))
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path};

    use super::folder_selection;
    use uuid::Uuid;

    #[test]
    fn accepts_folders_and_rejects_files() {
        let root = env::temp_dir().join(format!("illustrate-viewer-library-{}", Uuid::new_v4()));
        let file = root.join("not-a-folder.txt");
        fs::create_dir(&root).expect("temporary folder should be created");
        fs::write(&file, []).expect("temporary file should be created");

        let selected = folder_selection(&root).expect("folder should be valid");
        assert_eq!(selected.name, root.file_name().unwrap().to_string_lossy());
        assert!(folder_selection(&file).is_err());
        assert!(folder_selection(&root.join("missing")).is_err());

        #[cfg(target_os = "windows")]
        assert!(folder_selection(Path::new(r"C:\")).is_err());

        fs::remove_file(file).expect("temporary file should be removed");
        fs::remove_dir(root).expect("temporary folder should be removed");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn compares_windows_paths_without_case_sensitivity() {
        assert!(super::same_library_path(
            r"C:\Illustrations",
            r"c:\illustrations"
        ));
    }
}
