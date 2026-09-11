use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use image::{ImageFormat, ImageReader};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "custom-background.json";
const STORE_KEY_APPROVED_SOURCE: &str = "approvedSource";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomBackgroundSelection {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomBackgroundCommandError {
    pub code: &'static str,
    pub message: String,
}

impl CustomBackgroundCommandError {
    fn dialog(message: impl Into<String>) -> Self {
        Self {
            code: "dialog_failed",
            message: message.into(),
        }
    }

    fn source_missing() -> Self {
        Self {
            code: "source_missing",
            message: "The selected background image is no longer available.".into(),
        }
    }

    fn source_unreadable(message: impl Into<String>) -> Self {
        Self {
            code: "source_unreadable",
            message: message.into(),
        }
    }

    fn invalid_source(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_source",
            message: message.into(),
        }
    }

    fn unsupported_format() -> Self {
        Self {
            code: "unsupported_format",
            message: "Choose a PNG, JPEG, or WebP image.".into(),
        }
    }

    fn not_approved() -> Self {
        Self {
            code: "source_not_approved",
            message: "Choose the background image again before using it.".into(),
        }
    }

    fn persistence(message: impl Into<String>) -> Self {
        Self {
            code: "persistence_failed",
            message: message.into(),
        }
    }
}

#[tauri::command]
pub async fn choose_custom_background(
    app: AppHandle,
) -> Result<Option<CustomBackgroundSelection>, CustomBackgroundCommandError> {
    let selected = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || {
            app.dialog()
                .file()
                .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
                .blocking_pick_file()
        }
    })
    .await
    .map_err(|error| CustomBackgroundCommandError::dialog(error.to_string()))?;

    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| CustomBackgroundCommandError::dialog(error.to_string()))?;
    let path = validate_custom_background_path(&path)?;
    approve_source(&app, &path)?;

    Ok(Some(CustomBackgroundSelection {
        name: filename(&path)?,
        path: path.to_string_lossy().into_owned(),
    }))
}

#[tauri::command]
pub async fn prepare_custom_background(
    app: AppHandle,
    source: String,
) -> Result<String, CustomBackgroundCommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = validate_custom_background_path(Path::new(&source))?;
        if !is_approved_source(&app, &source)? {
            return Err(CustomBackgroundCommandError::not_approved());
        }

        app.asset_protocol_scope()
            .allow_file(&source)
            .map_err(|error| CustomBackgroundCommandError::source_unreadable(error.to_string()))?;
        Ok(source.to_string_lossy().into_owned())
    })
    .await
    .map_err(|error| CustomBackgroundCommandError::source_unreadable(error.to_string()))?
}

#[tauri::command]
pub async fn clear_custom_background(app: AppHandle) -> Result<(), CustomBackgroundCommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = app
            .store(STORE_FILE)
            .map_err(|error| CustomBackgroundCommandError::persistence(error.to_string()))?;
        store.set(STORE_KEY_APPROVED_SOURCE, json!(null));
        store
            .save()
            .map_err(|error| CustomBackgroundCommandError::persistence(error.to_string()))
    })
    .await
    .map_err(|error| CustomBackgroundCommandError::persistence(error.to_string()))?
}

fn approve_source(app: &AppHandle, source: &Path) -> Result<(), CustomBackgroundCommandError> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| CustomBackgroundCommandError::persistence(error.to_string()))?;
    store.set(
        STORE_KEY_APPROVED_SOURCE,
        json!(source.to_string_lossy().into_owned()),
    );
    store
        .save()
        .map_err(|error| CustomBackgroundCommandError::persistence(error.to_string()))
}

fn is_approved_source(
    app: &AppHandle,
    source: &Path,
) -> Result<bool, CustomBackgroundCommandError> {
    let store = app
        .store(STORE_FILE)
        .map_err(|error| CustomBackgroundCommandError::persistence(error.to_string()))?;
    let approved = store
        .get(STORE_KEY_APPROVED_SOURCE)
        .and_then(|value| value.as_str().map(str::to_owned));

    Ok(approved
        .as_deref()
        .is_some_and(|approved| same_path(approved, source)))
}

fn validate_custom_background_path(path: &Path) -> Result<PathBuf, CustomBackgroundCommandError> {
    let file_type = fs::symlink_metadata(path).map_err(source_error)?;
    if file_type.file_type().is_symlink() {
        return Err(CustomBackgroundCommandError::invalid_source(
            "The selected background image cannot be a symbolic link.",
        ));
    }

    let metadata = fs::metadata(path).map_err(source_error)?;
    if !metadata.is_file() {
        return Err(CustomBackgroundCommandError::invalid_source(
            "The selected background path is not a file.",
        ));
    }

    let source = fs::canonicalize(path).map_err(source_error)?;
    let format = ImageReader::open(&source)
        .map_err(source_error)?
        .with_guessed_format()
        .map_err(|error| CustomBackgroundCommandError::invalid_source(error.to_string()))?
        .format();
    if !matches!(
        format,
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
    ) {
        return Err(CustomBackgroundCommandError::unsupported_format());
    }

    Ok(source)
}

fn filename(path: &Path) -> Result<String, CustomBackgroundCommandError> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            CustomBackgroundCommandError::invalid_source("The selected image has no filename.")
        })
}

#[cfg(target_os = "windows")]
fn same_path(left: &str, right: &Path) -> bool {
    left.eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(target_os = "windows"))]
fn same_path(left: &str, right: &Path) -> bool {
    left == right.to_string_lossy()
}

fn source_error(error: std::io::Error) -> CustomBackgroundCommandError {
    match error.kind() {
        ErrorKind::NotFound => CustomBackgroundCommandError::source_missing(),
        ErrorKind::PermissionDenied => CustomBackgroundCommandError::source_unreadable(
            "The selected background image cannot be read.",
        ),
        _ => CustomBackgroundCommandError::source_unreadable(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path};

    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
    use uuid::Uuid;

    use super::validate_custom_background_path;

    #[test]
    fn accepts_static_background_formats_and_rejects_other_files() {
        let root = env::temp_dir().join(format!("illustrate-viewer-background-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("temporary folder should be created");
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(16, 12, Rgb([20, 40, 60])));

        for (name, format) in [
            ("background.png", ImageFormat::Png),
            ("background.jpg", ImageFormat::Jpeg),
            ("background.webp", ImageFormat::WebP),
        ] {
            let path = root.join(name);
            image
                .save_with_format(&path, format)
                .expect("test image should be written");
            assert!(validate_custom_background_path(&path).is_ok());
        }

        let text = root.join("not-an-image.txt");
        fs::write(&text, b"not an image").expect("test file should be written");
        assert!(validate_custom_background_path(&text).is_err());
        assert!(validate_custom_background_path(Path::new("missing-background.png")).is_err());

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }
}
