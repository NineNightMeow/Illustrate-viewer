use std::{
    collections::HashSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Mutex,
    time::UNIX_EPOCH,
};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::database::MetadataDatabase;
use crate::library::{
    find_library_record, update_library_image_count, Library, LibraryCommandError,
};

#[derive(Default)]
pub struct ScanRegistry(Mutex<HashSet<String>>);

impl ScanRegistry {
    pub fn begin(&self, library_id: &str) -> Result<(), ScannerCommandError> {
        let mut active = self
            .0
            .lock()
            .map_err(|_| ScannerCommandError::scan_failed("The scanner state is unavailable."))?;

        if !active.insert(library_id.to_string()) {
            return Err(ScannerCommandError::scan_in_progress());
        }

        Ok(())
    }

    pub fn finish(&self, library_id: &str) {
        if let Ok(mut active) = self.0.lock() {
            active.remove(library_id);
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub id: String,
    pub library_id: String,
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub size: u64,
    pub created_at: Option<u64>,
    pub modified_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub library: Library,
    pub assets: Vec<ImageAsset>,
    pub skipped_entries: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannerCommandError {
    pub code: &'static str,
    pub message: String,
}

impl ScannerCommandError {
    fn library_not_found() -> Self {
        Self {
            code: "library_not_found",
            message: "The library record no longer exists.".into(),
        }
    }

    fn library_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "library_unavailable",
            message: message.into(),
        }
    }

    fn permission_denied(message: impl Into<String>) -> Self {
        Self {
            code: "permission_denied",
            message: message.into(),
        }
    }

    pub fn scan_failed(message: impl Into<String>) -> Self {
        Self {
            code: "scan_failed",
            message: message.into(),
        }
    }

    fn persistence_failed(message: impl Into<String>) -> Self {
        Self {
            code: "persistence_failed",
            message: message.into(),
        }
    }

    fn scan_in_progress() -> Self {
        Self {
            code: "scan_in_progress",
            message: "This library is already being scanned.".into(),
        }
    }

    fn from_library(error: LibraryCommandError) -> Self {
        match error.code {
            "persistence_failed" => Self::persistence_failed(error.message),
            _ => Self::scan_failed(error.message),
        }
    }
}

pub fn scan_library_record(
    app: &AppHandle,
    library_id: &str,
) -> Result<ScanResult, ScannerCommandError> {
    let library = find_library_record(app, library_id)
        .map_err(ScannerCommandError::from_library)?
        .ok_or_else(ScannerCommandError::library_not_found)?;
    let (assets, skipped_entries) = scan_folder(Path::new(&library.path), library_id)?;
    let image_count = if skipped_entries == 0 {
        Some(
            assets
                .len()
                .try_into()
                .map_err(|_| ScannerCommandError::scan_failed("Too many image files to count."))?,
        )
    } else {
        None
    };
    app.state::<MetadataDatabase>()
        .sync_images_for_library(library_id, &assets)
        .map_err(|error| ScannerCommandError::persistence_failed(error.message))?;
    let library = update_library_image_count(app, library_id, image_count)
        .map_err(ScannerCommandError::from_library)?
        .ok_or_else(ScannerCommandError::library_not_found)?;

    Ok(ScanResult {
        library,
        assets,
        skipped_entries,
    })
}

fn scan_folder(
    root: &Path,
    library_id: &str,
) -> Result<(Vec<ImageAsset>, u32), ScannerCommandError> {
    let root = canonical_root(root)?;
    let mut directories = vec![root.clone()];
    let mut assets = Vec::new();
    let mut skipped_entries = 0_u32;

    while let Some(directory) = directories.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if directory == root => return Err(root_error(&error)),
            Err(_) => {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    skipped_entries = skipped_entries.saturating_add(1);
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => {
                    skipped_entries = skipped_entries.saturating_add(1);
                    continue;
                }
            };

            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                directories.push(entry.path());
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let Some(extension) = image_extension(&path) else {
                continue;
            };
            let metadata = match entry.metadata() {
                Ok(metadata) if metadata.len() > 0 => metadata,
                _ => {
                    skipped_entries = skipped_entries.saturating_add(1);
                    continue;
                }
            };
            let Some(modified_at) = modified_at(&metadata) else {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            };
            let relative_path = path.strip_prefix(&root).unwrap_or(&path);
            let relative_path = relative_path.to_string_lossy().replace('\\', "/");

            assets.push(ImageAsset {
                id: format!("{library_id}:{relative_path}"),
                library_id: library_id.to_string(),
                path: path.to_string_lossy().into_owned(),
                filename: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                extension: extension.to_string(),
                size: metadata.len(),
                created_at: created_at(&metadata),
                modified_at,
            });
        }
    }

    assets.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((assets, skipped_entries))
}

fn canonical_root(path: &Path) -> Result<PathBuf, ScannerCommandError> {
    let metadata = fs::metadata(path).map_err(|error| root_error(&error))?;
    if !metadata.is_dir() {
        return Err(ScannerCommandError::library_unavailable(
            "The library path is not a folder.",
        ));
    }

    fs::canonicalize(path).map_err(|error| root_error(&error))
}

fn root_error(error: &std::io::Error) -> ScannerCommandError {
    match error.kind() {
        ErrorKind::NotFound => {
            ScannerCommandError::library_unavailable("The library folder is no longer available.")
        }
        ErrorKind::PermissionDenied => {
            ScannerCommandError::permission_denied("The library folder cannot be read.")
        }
        _ => ScannerCommandError::scan_failed(error.to_string()),
    }
}

fn image_extension(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("png"),
        "jpg" => Some("jpg"),
        "jpeg" => Some("jpeg"),
        "webp" => Some("webp"),
        "gif" => Some("gif"),
        _ => None,
    }
}

fn modified_at(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}

fn created_at(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .created()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}

#[cfg(test)]
mod tests {
    use std::{env, fs, io};

    use uuid::Uuid;

    use super::scan_folder;

    fn temporary_folder(name: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!("illustrate-viewer-{name}-{}", Uuid::new_v4()))
    }

    #[test]
    fn scans_supported_extensions_and_skips_other_entries() {
        let root = temporary_folder("scanner");
        fs::create_dir_all(root.join("nested")).expect("temporary folders should be created");
        for filename in [
            "image.PNG",
            "nested/photo.jpg",
            "art.jpeg",
            "frame.webp",
            "loop.gif",
        ] {
            let path = root.join(filename);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("temporary parent should be created");
            }
            fs::write(path, b"scanner only reads file metadata")
                .expect("temporary image should be created");
        }
        fs::write(root.join("notes.txt"), b"not an image")
            .expect("temporary text file should be created");
        fs::write(root.join("broken.jpg"), []).expect("empty image file should be created");

        let (assets, skipped_entries) =
            scan_folder(&root, "library-1").expect("folder should scan");

        assert_eq!(assets.len(), 5);
        assert_eq!(skipped_entries, 1);
        assert!(assets
            .iter()
            .all(|asset| asset.id.starts_with("library-1:")));
        assert!(assets.iter().all(|asset| asset.size > 0));

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn scans_one_thousand_files_without_reading_contents() {
        let root = temporary_folder("scanner-large");
        fs::create_dir_all(&root).expect("temporary folder should be created");
        for index in 0..1_000 {
            fs::write(root.join(format!("image-{index}.png")), b"x")
                .expect("temporary image should be created");
        }

        let (assets, skipped_entries) =
            scan_folder(&root, "library-1").expect("folder should scan");

        assert_eq!(assets.len(), 1_000);
        assert_eq!(skipped_entries, 0);
        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn reports_missing_and_unreadable_roots_without_panicking() {
        let missing = temporary_folder("scanner-missing");
        let error = match scan_folder(&missing, "library-1") {
            Err(error) => error,
            Ok(_) => panic!("missing folder should fail"),
        };
        assert_eq!(error.code, "library_unavailable");

        let error = super::root_error(&io::Error::from(io::ErrorKind::PermissionDenied));
        assert_eq!(error.code, "permission_denied");
    }

    #[test]
    #[ignore = "performance smoke test"]
    fn scans_ten_thousand_files_without_reading_contents() {
        let root = temporary_folder("scanner-ten-thousand");
        fs::create_dir_all(&root).expect("temporary folder should be created");
        for index in 0..10_000 {
            fs::write(root.join(format!("image-{index}.jpg")), b"x")
                .expect("temporary image should be created");
        }

        let (assets, skipped_entries) =
            scan_folder(&root, "library-1").expect("folder should scan");

        assert_eq!(assets.len(), 10_000);
        assert_eq!(skipped_entries, 0);
        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }
}
