use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use image::{GenericImageView, ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::library::{find_library_record, LibraryCommandError};

mod queue;

pub use queue::{
    GalleryThumbnailWindow, ThumbnailQueueStatus, ThumbnailQueueSubmission, ThumbnailTaskQueue,
};

const THUMBNAIL_LONGEST_EDGE: u32 = 512;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailRequest {
    pub image_id: String,
    pub library_id: String,
    pub source_path: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum ThumbnailStatus {
    Pending,
    Generating,
    Completed,
    Failed,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailRecord {
    pub image_id: String,
    pub source_path: String,
    pub source_modified_at: u64,
    pub source_size: u64,
    pub thumbnail_path: String,
    pub width: u32,
    pub height: u32,
    pub generated_at: u64,
    pub status: ThumbnailStatus,
    pub error_code: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThumbnailCacheRecord {
    image_id: String,
    thumbnail_path: String,
    width: u32,
    height: u32,
    generated_at: u64,
    source_modified_at: u64,
    source_size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailCommandError {
    pub code: &'static str,
    pub message: String,
}

impl ThumbnailCommandError {
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

    fn source_missing() -> Self {
        Self {
            code: "source_missing",
            message: "The source image no longer exists.".into(),
        }
    }

    fn source_unreadable(message: impl Into<String>) -> Self {
        Self {
            code: "source_unreadable",
            message: message.into(),
        }
    }

    fn invalid_source_path(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_source_path",
            message: message.into(),
        }
    }

    fn unsupported_format() -> Self {
        Self {
            code: "unsupported_format",
            message: "The source image format is not supported.".into(),
        }
    }

    fn decode_failed(message: impl Into<String>) -> Self {
        Self {
            code: "decode_failed",
            message: message.into(),
        }
    }

    fn cache_read_failed(message: impl Into<String>) -> Self {
        Self {
            code: "cache_read_failed",
            message: message.into(),
        }
    }

    fn cache_write_failed(message: impl Into<String>) -> Self {
        Self {
            code: "cache_write_failed",
            message: message.into(),
        }
    }

    fn cancelled() -> Self {
        Self {
            code: "cancelled",
            message: "The thumbnail task was cancelled.".into(),
        }
    }

    pub fn thumbnail_failed(message: impl Into<String>) -> Self {
        Self {
            code: "thumbnail_failed",
            message: message.into(),
        }
    }

    fn from_library(error: LibraryCommandError) -> Self {
        match error.code {
            "persistence_failed" => Self::thumbnail_failed(error.message),
            _ => Self::library_unavailable(error.message),
        }
    }
}

pub(crate) fn generate_thumbnail_record_with_cancellation(
    app: &AppHandle,
    request: ThumbnailRequest,
    cancellation: &AtomicBool,
) -> Result<ThumbnailRecord, ThumbnailCommandError> {
    ensure_not_cancelled(cancellation)?;
    let library = find_library_record(app, &request.library_id)
        .map_err(ThumbnailCommandError::from_library)?
        .ok_or_else(ThumbnailCommandError::library_not_found)?;
    let cache_directory = thumbnail_cache_directory(app, &library.id)?;
    let source_path = match validate_source_path(
        Path::new(&library.path),
        &library.id,
        &request.image_id,
        Path::new(&request.source_path),
    ) {
        Ok(source_path) => source_path,
        Err(error) if error.code == "source_missing" => {
            remove_cache_entry(&thumbnail_cache_path(&cache_directory, &request.image_id))?;
            return Err(error);
        }
        Err(error) => return Err(error),
    };

    let record = generate_or_reuse_thumbnail_with_cancellation(
        &source_path,
        &cache_directory,
        &request.image_id,
        cancellation,
    )?;
    app.asset_protocol_scope()
        .allow_file(&record.thumbnail_path)
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?;

    Ok(record)
}

pub(crate) fn find_cached_thumbnail_record(
    app: &AppHandle,
    request: &ThumbnailRequest,
) -> Result<Option<ThumbnailRecord>, ThumbnailCommandError> {
    let library = find_library_record(app, &request.library_id)
        .map_err(ThumbnailCommandError::from_library)?
        .ok_or_else(ThumbnailCommandError::library_not_found)?;
    let cache_directory = thumbnail_cache_directory(app, &library.id)?;
    let source_path = match validate_source_path(
        Path::new(&library.path),
        &library.id,
        &request.image_id,
        Path::new(&request.source_path),
    ) {
        Ok(source_path) => source_path,
        Err(error) if error.code == "source_missing" => {
            remove_cache_entry(&thumbnail_cache_path(&cache_directory, &request.image_id))?;
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    let source_metadata = fs::metadata(&source_path).map_err(source_error)?;
    let thumbnail_path = thumbnail_cache_path(&cache_directory, &request.image_id);
    let Some(record) = load_cached_thumbnail(
        &thumbnail_path,
        &request.image_id,
        &source_path,
        &source_metadata,
    )?
    else {
        return Ok(None);
    };

    app.asset_protocol_scope()
        .allow_file(&record.thumbnail_path)
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?;
    Ok(Some(record))
}

pub(crate) fn prepare_image_asset(
    app: &AppHandle,
    request: &ThumbnailRequest,
) -> Result<String, ThumbnailCommandError> {
    let library = find_library_record(app, &request.library_id)
        .map_err(ThumbnailCommandError::from_library)?
        .ok_or_else(ThumbnailCommandError::library_not_found)?;
    let source_path = validate_source_path(
        Path::new(&library.path),
        &library.id,
        &request.image_id,
        Path::new(&request.source_path),
    )?;

    app.asset_protocol_scope()
        .allow_file(&source_path)
        .map_err(|error| ThumbnailCommandError::thumbnail_failed(error.to_string()))?;
    Ok(source_path.to_string_lossy().into_owned())
}

pub fn clear_thumbnail_cache(app: &AppHandle) -> Result<(), ThumbnailCommandError> {
    clear_thumbnail_cache_directory(&thumbnail_cache_root(app)?)
}

pub(crate) fn remove_stale_thumbnail_temporary_files(app: &AppHandle) {
    if let Ok(cache_root) = thumbnail_cache_root(app) {
        remove_temporary_cache_files(&cache_root);
    }
}

fn validate_source_path(
    library_path: &Path,
    library_id: &str,
    image_id: &str,
    source_path: &Path,
) -> Result<PathBuf, ThumbnailCommandError> {
    let library_root = fs::canonicalize(library_path).map_err(library_error)?;
    let source_file_type = fs::symlink_metadata(source_path).map_err(source_error)?;
    if source_file_type.file_type().is_symlink() {
        return Err(ThumbnailCommandError::invalid_source_path(
            "The source image cannot be a symbolic link.",
        ));
    }

    let source_metadata = fs::metadata(source_path).map_err(source_error)?;
    if !source_metadata.is_file() {
        return Err(ThumbnailCommandError::invalid_source_path(
            "The source path is not a file.",
        ));
    }

    let source_path = fs::canonicalize(source_path).map_err(source_error)?;
    let relative_path = source_path.strip_prefix(&library_root).map_err(|_| {
        ThumbnailCommandError::invalid_source_path("The source image is outside this library.")
    })?;
    let expected_image_id = format!(
        "{library_id}:{}",
        relative_path.to_string_lossy().replace('\\', "/")
    );
    if image_id != expected_image_id {
        return Err(ThumbnailCommandError::invalid_source_path(
            "The source image does not match the scanned asset.",
        ));
    }
    if !is_supported_image(&source_path) {
        return Err(ThumbnailCommandError::unsupported_format());
    }

    Ok(source_path)
}

#[cfg(test)]
fn generate_or_reuse_thumbnail(
    source_path: &Path,
    cache_directory: &Path,
    image_id: &str,
) -> Result<ThumbnailRecord, ThumbnailCommandError> {
    let cancellation = AtomicBool::new(false);
    generate_or_reuse_thumbnail_with_cancellation(
        source_path,
        cache_directory,
        image_id,
        &cancellation,
    )
}

fn generate_or_reuse_thumbnail_with_cancellation(
    source_path: &Path,
    cache_directory: &Path,
    image_id: &str,
    cancellation: &AtomicBool,
) -> Result<ThumbnailRecord, ThumbnailCommandError> {
    ensure_not_cancelled(cancellation)?;
    let thumbnail_path = thumbnail_cache_path(cache_directory, image_id);
    let source_metadata = match fs::metadata(source_path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            remove_cache_entry(&thumbnail_path)?;
            return Err(ThumbnailCommandError::invalid_source_path(
                "The source path is not a file.",
            ));
        }
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                remove_cache_entry(&thumbnail_path)?;
            }
            return Err(source_error(error));
        }
    };

    if let Some(record) =
        load_cached_thumbnail(&thumbnail_path, image_id, source_path, &source_metadata)?
    {
        ensure_not_cancelled(cancellation)?;
        return Ok(record);
    }

    let (width, height) =
        generate_thumbnail_file_with_cancellation(source_path, &thumbnail_path, cancellation)?;
    if let Err(error) = ensure_not_cancelled(cancellation) {
        remove_cache_entry(&thumbnail_path)?;
        return Err(error);
    }
    let cache_record = ThumbnailCacheRecord {
        image_id: image_id.to_string(),
        thumbnail_path: thumbnail_path.to_string_lossy().into_owned(),
        width,
        height,
        generated_at: current_timestamp(),
        source_modified_at: modified_at(&source_metadata),
        source_size: source_metadata.len(),
    };
    save_cache_record_with_cancellation(&thumbnail_path, &cache_record, cancellation)?;
    if let Err(error) = ensure_not_cancelled(cancellation) {
        remove_cache_entry(&thumbnail_path)?;
        return Err(error);
    }

    Ok(thumbnail_record(
        &cache_record,
        source_path,
        &source_metadata,
    ))
}

fn load_cached_thumbnail(
    thumbnail_path: &Path,
    image_id: &str,
    source_path: &Path,
    source_metadata: &fs::Metadata,
) -> Result<Option<ThumbnailRecord>, ThumbnailCommandError> {
    let Some(cache_record) = read_cache_record(&cache_record_path(thumbnail_path))? else {
        remove_cache_entry(thumbnail_path)?;
        return Ok(None);
    };

    let expected_path = thumbnail_path.to_string_lossy();
    let cache_matches_source = cache_record.image_id == image_id
        && cache_record.thumbnail_path == expected_path.as_ref()
        && cache_record.source_modified_at == modified_at(source_metadata)
        && cache_record.source_size == source_metadata.len();
    let cache_file_is_valid = cached_thumbnail_dimensions(thumbnail_path)
        .is_some_and(|dimensions| dimensions == (cache_record.width, cache_record.height));

    if !cache_matches_source || !cache_file_is_valid {
        remove_cache_entry(thumbnail_path)?;
        return Ok(None);
    }

    Ok(Some(thumbnail_record(
        &cache_record,
        source_path,
        source_metadata,
    )))
}

fn thumbnail_record(
    cache_record: &ThumbnailCacheRecord,
    source_path: &Path,
    source_metadata: &fs::Metadata,
) -> ThumbnailRecord {
    ThumbnailRecord {
        image_id: cache_record.image_id.clone(),
        source_path: source_path.to_string_lossy().into_owned(),
        source_modified_at: modified_at(source_metadata),
        source_size: source_metadata.len(),
        thumbnail_path: cache_record.thumbnail_path.clone(),
        width: cache_record.width,
        height: cache_record.height,
        generated_at: cache_record.generated_at,
        status: ThumbnailStatus::Completed,
        error_code: None,
    }
}

fn thumbnail_cache_root(app: &AppHandle) -> Result<PathBuf, ThumbnailCommandError> {
    app.path()
        .app_cache_dir()
        .map(|path| path.join("thumbnails"))
        .map_err(|error| ThumbnailCommandError::cache_write_failed(error.to_string()))
}

fn thumbnail_cache_directory(
    app: &AppHandle,
    library_id: &str,
) -> Result<PathBuf, ThumbnailCommandError> {
    Ok(thumbnail_cache_root(app)?.join(library_id))
}

fn thumbnail_cache_path(cache_directory: &Path, image_id: &str) -> PathBuf {
    let cache_key = Uuid::new_v5(&Uuid::NAMESPACE_URL, image_id.as_bytes());
    cache_directory.join(format!("{cache_key}.webp"))
}

fn cache_record_path(thumbnail_path: &Path) -> PathBuf {
    thumbnail_path.with_extension("json")
}

fn read_cache_record(
    cache_record_path: &Path,
) -> Result<Option<ThumbnailCacheRecord>, ThumbnailCommandError> {
    let bytes = match fs::read(cache_record_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ThumbnailCommandError::cache_read_failed(error.to_string())),
    };

    Ok(serde_json::from_slice(&bytes).ok())
}

#[cfg(test)]
fn save_cache_record(
    thumbnail_path: &Path,
    cache_record: &ThumbnailCacheRecord,
) -> Result<(), ThumbnailCommandError> {
    let cancellation = AtomicBool::new(false);
    save_cache_record_with_cancellation(thumbnail_path, cache_record, &cancellation)
}

fn save_cache_record_with_cancellation(
    thumbnail_path: &Path,
    cache_record: &ThumbnailCacheRecord,
    cancellation: &AtomicBool,
) -> Result<(), ThumbnailCommandError> {
    let cache_record_path = cache_record_path(thumbnail_path);
    let cache_directory = cache_record_path.parent().ok_or_else(|| {
        ThumbnailCommandError::cache_write_failed("The thumbnail cache path is invalid.")
    })?;
    fs::create_dir_all(cache_directory)
        .map_err(|error| ThumbnailCommandError::cache_write_failed(error.to_string()))?;
    let data = serde_json::to_vec(cache_record)
        .map_err(|error| ThumbnailCommandError::cache_write_failed(error.to_string()))?;
    let temporary_path = cache_record_path.with_extension(format!("json.{}.tmp", Uuid::new_v4()));
    fs::write(&temporary_path, data)
        .map_err(|error| ThumbnailCommandError::cache_write_failed(error.to_string()))?;
    if let Err(error) = ensure_not_cancelled(cancellation) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    replace_cache_file(&temporary_path, &cache_record_path)
}

#[cfg(test)]
fn generate_thumbnail_file(
    source_path: &Path,
    thumbnail_path: &Path,
) -> Result<(u32, u32), ThumbnailCommandError> {
    let cancellation = AtomicBool::new(false);
    generate_thumbnail_file_with_cancellation(source_path, thumbnail_path, &cancellation)
}

fn generate_thumbnail_file_with_cancellation(
    source_path: &Path,
    thumbnail_path: &Path,
    cancellation: &AtomicBool,
) -> Result<(u32, u32), ThumbnailCommandError> {
    ensure_not_cancelled(cancellation)?;
    let image = ImageReader::open(source_path)
        .map_err(source_error)?
        .with_guessed_format()
        .map_err(|error| ThumbnailCommandError::decode_failed(error.to_string()))?
        .decode()
        .map_err(|error| ThumbnailCommandError::decode_failed(error.to_string()))?;
    ensure_not_cancelled(cancellation)?;
    let thumbnail = image.thumbnail(THUMBNAIL_LONGEST_EDGE, THUMBNAIL_LONGEST_EDGE);
    let (width, height) = thumbnail.dimensions();
    let cache_directory = thumbnail_path.parent().ok_or_else(|| {
        ThumbnailCommandError::cache_write_failed("The thumbnail cache path is invalid.")
    })?;
    fs::create_dir_all(cache_directory)
        .map_err(|error| ThumbnailCommandError::cache_write_failed(error.to_string()))?;

    let temporary_path = thumbnail_path.with_extension(format!("webp.{}.tmp", Uuid::new_v4()));
    if let Err(error) = thumbnail.save_with_format(&temporary_path, ImageFormat::WebP) {
        let _ = fs::remove_file(&temporary_path);
        return Err(ThumbnailCommandError::cache_write_failed(error.to_string()));
    }
    if let Err(error) = ensure_not_cancelled(cancellation) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    replace_cache_file(&temporary_path, thumbnail_path)?;

    Ok((width, height))
}

fn ensure_not_cancelled(cancellation: &AtomicBool) -> Result<(), ThumbnailCommandError> {
    if cancellation.load(Ordering::Acquire) {
        return Err(ThumbnailCommandError::cancelled());
    }

    Ok(())
}

fn replace_cache_file(
    temporary_path: &Path,
    destination_path: &Path,
) -> Result<(), ThumbnailCommandError> {
    if destination_path.exists() {
        fs::remove_file(destination_path)
            .map_err(|error| ThumbnailCommandError::cache_write_failed(error.to_string()))?;
    }
    if let Err(error) = fs::rename(temporary_path, destination_path) {
        let _ = fs::remove_file(temporary_path);
        return Err(ThumbnailCommandError::cache_write_failed(error.to_string()));
    }

    Ok(())
}

fn remove_cache_entry(thumbnail_path: &Path) -> Result<(), ThumbnailCommandError> {
    for path in [
        thumbnail_path.to_path_buf(),
        cache_record_path(thumbnail_path),
    ] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(ThumbnailCommandError::cache_write_failed(error.to_string())),
        }
    }

    Ok(())
}

fn clear_thumbnail_cache_directory(cache_root: &Path) -> Result<(), ThumbnailCommandError> {
    match fs::remove_dir_all(cache_root) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ThumbnailCommandError::cache_write_failed(error.to_string())),
    }
}

fn remove_temporary_cache_files(cache_root: &Path) {
    let mut directories = vec![cache_root.to_path_buf()];

    while let Some(directory) = directories.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if file_type.is_file()
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.ends_with(".tmp"))
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn cached_thumbnail_dimensions(path: &Path) -> Option<(u32, u32)> {
    ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()
        .map(|image| image.dimensions())
}

fn is_supported_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase()),
        Some(extension) if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif")
    )
}

fn library_error(error: std::io::Error) -> ThumbnailCommandError {
    match error.kind() {
        ErrorKind::NotFound => {
            ThumbnailCommandError::library_unavailable("The library folder is no longer available.")
        }
        ErrorKind::PermissionDenied => {
            ThumbnailCommandError::library_unavailable("The library folder cannot be read.")
        }
        _ => ThumbnailCommandError::library_unavailable(error.to_string()),
    }
}

fn source_error(error: std::io::Error) -> ThumbnailCommandError {
    match error.kind() {
        ErrorKind::NotFound => ThumbnailCommandError::source_missing(),
        ErrorKind::PermissionDenied => {
            ThumbnailCommandError::source_unreadable("The source image cannot be read.")
        }
        _ => ThumbnailCommandError::source_unreadable(error.to_string()),
    }
}

fn modified_at(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| duration.as_millis().try_into().ok())
        .unwrap_or_default()
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path, sync::atomic::AtomicBool};

    use image::{DynamicImage, GenericImageView, ImageFormat, Rgb, RgbImage};
    use uuid::Uuid;

    use super::{
        cache_record_path, clear_thumbnail_cache_directory, generate_or_reuse_thumbnail,
        generate_thumbnail_file, generate_thumbnail_file_with_cancellation, read_cache_record,
        remove_temporary_cache_files, save_cache_record, thumbnail_cache_path,
    };

    fn temporary_folder(name: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!("illustrate-viewer-{name}-{}", Uuid::new_v4()))
    }

    fn write_image(path: &Path, format: ImageFormat, width: u32, height: u32) {
        let image =
            DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb([50, 100, 150])));
        image
            .save_with_format(path, format)
            .expect("test image should be written");
    }

    #[test]
    fn creates_webp_thumbnails_for_png_jpeg_and_webp() {
        let root = temporary_folder("thumbnail-formats");
        fs::create_dir_all(&root).expect("temporary folder should be created");

        for (filename, format) in [
            ("source.png", ImageFormat::Png),
            ("source.jpg", ImageFormat::Jpeg),
            ("source.webp", ImageFormat::WebP),
        ] {
            let source = root.join(filename);
            let thumbnail = root.join(format!("{filename}.webp"));
            write_image(&source, format, 1_200, 800);

            let (width, height) = generate_thumbnail_file(&source, &thumbnail)
                .expect("thumbnail generation should succeed");

            assert!(thumbnail.is_file());
            assert!(width <= 512 && height <= 512);
            assert_eq!(
                image::open(&thumbnail).unwrap().dimensions(),
                (width, height)
            );
        }

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn reuses_valid_cache_and_regenerates_after_source_changes() {
        let root = temporary_folder("thumbnail-cache-lifecycle");
        let cache_directory = root.join("cache").join("thumbnails").join("library-1");
        let source = root.join("source.png");
        fs::create_dir_all(&root).expect("temporary folder should be created");
        write_image(&source, ImageFormat::Png, 1_200, 800);

        let image_id = "library-1:source.png";
        let first = generate_or_reuse_thumbnail(&source, &cache_directory, image_id)
            .expect("first request should create cache");
        let thumbnail_path = std::path::PathBuf::from(&first.thumbnail_path);
        let cache_record_path = cache_record_path(&thumbnail_path);
        assert!(thumbnail_path.is_file());
        assert!(cache_record_path.is_file());

        let mut cache_record = read_cache_record(&cache_record_path)
            .unwrap()
            .expect("cache metadata should exist");
        cache_record.generated_at = 1;
        save_cache_record(&thumbnail_path, &cache_record)
            .expect("cache metadata should be writable");
        let reused = generate_or_reuse_thumbnail(&source, &cache_directory, image_id)
            .expect("second request should reuse cache");
        assert_eq!(reused.generated_at, 1);

        write_image(&source, ImageFormat::Png, 2_000, 1_000);
        let regenerated = generate_or_reuse_thumbnail(&source, &cache_directory, image_id)
            .expect("changed source should regenerate cache");
        assert_ne!(regenerated.generated_at, 1);

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn regenerates_corrupt_cache_and_discards_cache_for_deleted_source() {
        let root = temporary_folder("thumbnail-cache-errors");
        let cache_directory = root.join("cache").join("thumbnails").join("library-1");
        let source = root.join("source.png");
        fs::create_dir_all(&root).expect("temporary folder should be created");
        write_image(&source, ImageFormat::Png, 1_200, 800);

        let image_id = "library-1:source.png";
        let first = generate_or_reuse_thumbnail(&source, &cache_directory, image_id)
            .expect("first request should create cache");
        let thumbnail_path = std::path::PathBuf::from(&first.thumbnail_path);
        fs::write(&thumbnail_path, b"not a thumbnail").expect("cache file should be corruptible");
        let regenerated = generate_or_reuse_thumbnail(&source, &cache_directory, image_id)
            .expect("corrupt cache should regenerate");
        assert!(image::open(&regenerated.thumbnail_path).is_ok());

        fs::remove_file(&source).expect("source should be removable");
        let error = match generate_or_reuse_thumbnail(&source, &cache_directory, image_id) {
            Err(error) => error,
            Ok(_) => panic!("deleted source should fail"),
        };
        assert_eq!(error.code, "source_missing");
        assert!(!thumbnail_path.exists());
        assert!(!cache_record_path(&thumbnail_path).exists());

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn clears_the_thumbnail_cache_directory() {
        let root = temporary_folder("thumbnail-clear-cache");
        let cache_root = root.join("cache").join("thumbnails");
        fs::create_dir_all(cache_root.join("library-1"))
            .expect("cache directory should be created");
        fs::write(cache_root.join("library-1").join("image.webp"), b"cache")
            .expect("cache file should be written");

        clear_thumbnail_cache_directory(&cache_root).expect("cache directory should clear");
        assert!(!cache_root.exists());

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn removes_stale_temporary_cache_files_without_removing_completed_thumbnails() {
        let root = temporary_folder("thumbnail-temporary-cache");
        fs::create_dir_all(root.join("library-1")).expect("cache directory should be created");
        let temporary = root.join("library-1").join("image.webp.pending.tmp");
        let completed = root.join("library-1").join("image.webp");
        fs::write(&temporary, b"temporary").expect("temporary cache file should be written");
        fs::write(&completed, b"completed").expect("completed cache file should be written");

        remove_temporary_cache_files(&root);
        assert!(!temporary.exists());
        assert!(completed.exists());

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn reports_missing_and_corrupt_sources() {
        let root = temporary_folder("thumbnail-errors");
        fs::create_dir_all(&root).expect("temporary folder should be created");
        let thumbnail = root.join("thumbnail.webp");

        let error = generate_thumbnail_file(&root.join("missing.png"), &thumbnail).unwrap_err();
        assert_eq!(error.code, "source_missing");

        let corrupt = root.join("corrupt.jpg");
        fs::write(&corrupt, b"not an image").expect("corrupt test file should be written");
        let error = generate_thumbnail_file(&corrupt, &thumbnail).unwrap_err();
        assert_eq!(error.code, "decode_failed");

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn cancellation_does_not_write_a_final_thumbnail_file() {
        let root = temporary_folder("thumbnail-cancellation");
        fs::create_dir_all(&root).expect("temporary folder should be created");
        let source = root.join("source.png");
        let thumbnail = root.join("thumbnail.webp");
        write_image(&source, ImageFormat::Png, 1_200, 800);
        let cancellation = AtomicBool::new(true);

        let error = generate_thumbnail_file_with_cancellation(&source, &thumbnail, &cancellation)
            .expect_err("cancelled generation should fail");
        assert_eq!(error.code, "cancelled");
        assert!(!thumbnail.exists());

        fs::remove_dir_all(root).expect("temporary folder should be removed");
    }

    #[test]
    fn puts_each_library_thumbnail_in_a_safe_cache_file() {
        let root = temporary_folder("thumbnail-cache");
        let cache_directory = root.join("library-1");
        let path = thumbnail_cache_path(&cache_directory, "library-1:nested/image.png");

        assert_eq!(path.parent(), Some(cache_directory.as_path()));
        assert_eq!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("webp")
        );
        assert!(!path.file_name().unwrap().to_string_lossy().contains(':'));
    }
}
