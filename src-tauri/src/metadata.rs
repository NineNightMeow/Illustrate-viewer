use std::{
    fs,
    io::ErrorKind,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use image::{DynamicImage, ImageFormat, ImageReader};
use serde::Serialize;
use tauri::AppHandle;

use crate::{
    database::{ImageMetadata, ImageMetadataError, MetadataDatabase, MetadataDatabaseError},
    thumbnail::{prepare_image_asset, ThumbnailCommandError, ThumbnailRequest},
};

pub const METADATA_EXTRACTOR_VERSION: u32 = 2;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExtractionError {
    pub code: &'static str,
    pub message: String,
}

impl MetadataExtractionError {
    fn image_not_found() -> Self {
        Self {
            code: "image_metadata_image_not_found",
            message: "The image is no longer available in the metadata database.".into(),
        }
    }

    fn source(error: ThumbnailCommandError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }

    fn decode(error: impl ToString) -> Self {
        Self {
            code: "metadata_decode_failed",
            message: error.to_string(),
        }
    }

    fn metadata(error: MetadataDatabaseError) -> Self {
        Self {
            code: "image_metadata_persistence_failed",
            message: error.message,
        }
    }

    fn persistence(error: ImageMetadataError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }

    pub(crate) fn cancelled() -> Self {
        Self {
            code: "cancelled",
            message: "The metadata task was cancelled.".into(),
        }
    }

    pub(crate) fn queue(message: impl Into<String>) -> Self {
        Self {
            code: "metadata_queue_failed",
            message: message.into(),
        }
    }
}

pub fn extract_image_metadata(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
    cancellation: &AtomicBool,
) -> Result<ImageMetadata, MetadataExtractionError> {
    ensure_not_cancelled(cancellation)?;
    let asset = database
        .find_image_asset(image_id)
        .map_err(MetadataExtractionError::metadata)?
        .ok_or_else(MetadataExtractionError::image_not_found)?;
    let fallback_modified_at = i64::try_from(asset.modified_at).unwrap_or(0);
    let source_path = match prepare_image_asset(
        app,
        &ThumbnailRequest {
            image_id: asset.id.clone(),
            library_id: asset.library_id,
            source_path: asset.path,
        },
    ) {
        Ok(path) => path,
        Err(error) => {
            let error = MetadataExtractionError::source(error);
            persist_failure(database, image_id, fallback_modified_at, None, &error)?;
            return Err(error);
        }
    };
    let source = Path::new(&source_path);
    let file = match fs::metadata(source) {
        Ok(file) => file,
        Err(error) => {
            let error = source_file_error(error);
            persist_failure(database, image_id, fallback_modified_at, None, &error)?;
            return Err(error);
        }
    };
    let source_modified_at = modified_at(&file)?;
    let source_created_at = created_at(&file);
    if let Some(metadata) = database
        .get_image_metadata(image_id)
        .map_err(MetadataExtractionError::persistence)?
        .filter(|metadata| metadata_is_current(metadata, source_modified_at))
    {
        return Ok(metadata);
    }

    let result = extract_metadata_from_source(
        source,
        image_id,
        file.len(),
        source_created_at,
        source_modified_at,
        cancellation,
    );
    match result {
        Ok(metadata) => {
            database
                .replace_image_metadata(&metadata)
                .map_err(MetadataExtractionError::persistence)?;
            Ok(metadata)
        }
        Err(error) if error.code == "cancelled" => Err(error),
        Err(error) => {
            persist_failure(
                database,
                image_id,
                source_modified_at,
                source_created_at,
                &error,
            )?;
            Err(error)
        }
    }
}

pub(crate) fn metadata_is_current(metadata: &ImageMetadata, source_modified_at: i64) -> bool {
    metadata.status == "ready"
        && metadata.extraction_version == METADATA_EXTRACTOR_VERSION
        && metadata.source_modified_at == source_modified_at
}

fn extract_metadata_from_source(
    path: &Path,
    image_id: &str,
    file_size: u64,
    source_created_at: Option<i64>,
    source_modified_at: i64,
    cancellation: &AtomicBool,
) -> Result<ImageMetadata, MetadataExtractionError> {
    ensure_not_cancelled(cancellation)?;
    let reader = ImageReader::open(path)
        .map_err(source_file_error)?
        .with_guessed_format()
        .map_err(MetadataExtractionError::decode)?;
    let format = reader.format().map(image_format_name).ok_or_else(|| {
        MetadataExtractionError::decode("The image format could not be identified.")
    })?;
    let image = reader.decode().map_err(MetadataExtractionError::decode)?;
    ensure_not_cancelled(cancellation)?;
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return Err(MetadataExtractionError::decode(
            "The image dimensions are invalid.",
        ));
    }

    Ok(ImageMetadata {
        image_id: image_id.to_owned(),
        width: Some(width),
        height: Some(height),
        aspect_ratio: Some(f64::from(width) / f64::from(height)),
        file_size: Some(file_size),
        format: Some(format.to_owned()),
        has_alpha: Some(has_transparency(image)),
        camera_make: None,
        camera_model: None,
        captured_at: None,
        orientation: None,
        extraction_version: METADATA_EXTRACTOR_VERSION,
        status: "ready".to_owned(),
        failure_reason: None,
        extracted_at: timestamp()?,
        source_created_at,
        source_modified_at,
    })
}

fn persist_failure(
    database: &MetadataDatabase,
    image_id: &str,
    source_modified_at: i64,
    source_created_at: Option<i64>,
    error: &MetadataExtractionError,
) -> Result<(), MetadataExtractionError> {
    database
        .replace_image_metadata(&ImageMetadata {
            image_id: image_id.to_owned(),
            width: None,
            height: None,
            aspect_ratio: None,
            file_size: None,
            format: None,
            has_alpha: None,
            camera_make: None,
            camera_model: None,
            captured_at: None,
            orientation: None,
            extraction_version: METADATA_EXTRACTOR_VERSION,
            status: "failed".to_owned(),
            failure_reason: Some(error.code.to_owned()),
            extracted_at: timestamp()?,
            source_created_at,
            source_modified_at,
        })
        .map_err(MetadataExtractionError::persistence)
}

fn has_transparency(image: DynamicImage) -> bool {
    image.into_rgba8().pixels().any(|pixel| pixel[3] < u8::MAX)
}

fn image_format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "PNG",
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::WebP => "WEBP",
        ImageFormat::Gif => "GIF",
        _ => "Unknown",
    }
}

fn ensure_not_cancelled(cancellation: &AtomicBool) -> Result<(), MetadataExtractionError> {
    cancellation
        .load(Ordering::Acquire)
        .then_some(())
        .map_or(Ok(()), |_| Err(MetadataExtractionError::cancelled()))
}

fn source_file_error(error: std::io::Error) -> MetadataExtractionError {
    match error.kind() {
        ErrorKind::NotFound => MetadataExtractionError {
            code: "source_missing",
            message: "The source image no longer exists.".into(),
        },
        ErrorKind::PermissionDenied => MetadataExtractionError {
            code: "source_unreadable",
            message: error.to_string(),
        },
        _ => MetadataExtractionError::decode(error),
    }
}

fn modified_at(metadata: &fs::Metadata) -> Result<i64, MetadataExtractionError> {
    file_time(metadata.modified().map_err(source_file_error)?)
}

fn created_at(metadata: &fs::Metadata) -> Option<i64> {
    metadata
        .created()
        .ok()
        .and_then(|time| file_time(time).ok())
}

fn file_time(time: SystemTime) -> Result<i64, MetadataExtractionError> {
    let milliseconds = time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| MetadataExtractionError::decode("The source image timestamp is invalid."))?
        .as_millis();
    i64::try_from(milliseconds)
        .map_err(|_| MetadataExtractionError::decode("The source image timestamp is too large."))
}

fn timestamp() -> Result<i64, MetadataExtractionError> {
    file_time(SystemTime::now())
}

#[cfg(test)]
mod tests {
    use std::{env, fs, sync::atomic::AtomicBool};

    use image::{DynamicImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
    use uuid::Uuid;

    use super::{extract_metadata_from_source, metadata_is_current, METADATA_EXTRACTOR_VERSION};

    fn temporary_folder(name: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!(
            "illustrate-viewer-image-metadata-{name}-{}",
            Uuid::new_v4()
        ))
    }

    #[test]
    fn extracts_dimensions_format_size_and_real_transparency() {
        let root = temporary_folder("extract");
        fs::create_dir_all(&root).expect("create temporary directory");
        let transparent = root.join("transparent.png");
        DynamicImage::ImageRgba8(RgbaImage::from_fn(3, 2, |x, _| {
            Rgba([20, 40, 60, if x == 0 { 0 } else { 255 }])
        }))
        .save_with_format(&transparent, ImageFormat::Png)
        .expect("write PNG");
        let metadata = extract_metadata_from_source(
            &transparent,
            "image-1",
            fs::metadata(&transparent).expect("read PNG metadata").len(),
            None,
            1,
            &AtomicBool::new(false),
        )
        .expect("extract PNG metadata");
        assert_eq!((metadata.width, metadata.height), (Some(3), Some(2)));
        assert_eq!(metadata.format.as_deref(), Some("PNG"));
        assert_eq!(metadata.has_alpha, Some(true));
        assert_eq!(metadata.camera_make, None);
        assert_eq!(metadata.orientation, None);

        let opaque = root.join("opaque.jpg");
        DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 1, Rgb([20, 40, 60])))
            .save_with_format(&opaque, ImageFormat::Jpeg)
            .expect("write JPEG");
        let metadata = extract_metadata_from_source(
            &opaque,
            "image-2",
            fs::metadata(&opaque).expect("read JPEG metadata").len(),
            None,
            1,
            &AtomicBool::new(false),
        )
        .expect("extract JPEG metadata");
        assert_eq!(metadata.format.as_deref(), Some("JPEG"));
        assert_eq!(metadata.has_alpha, Some(false));
        fs::remove_dir_all(root).expect("remove temporary directory");
    }

    #[test]
    fn invalidates_cached_metadata_for_source_or_extractor_changes() {
        let root = temporary_folder("cache");
        fs::create_dir_all(&root).expect("create temporary directory");
        let path = root.join("image.png");
        DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb([1, 2, 3])))
            .save_with_format(&path, ImageFormat::Png)
            .expect("write image");
        let metadata =
            extract_metadata_from_source(&path, "image-1", 1, None, 42, &AtomicBool::new(false))
                .expect("extract metadata");
        assert!(metadata_is_current(&metadata, 42));
        assert!(!metadata_is_current(&metadata, 43));
        assert_eq!(metadata.extraction_version, METADATA_EXTRACTOR_VERSION);
        fs::remove_dir_all(root).expect("remove temporary directory");
    }

    #[test]
    fn rejects_missing_and_corrupt_sources_without_panicking() {
        let root = temporary_folder("invalid");
        fs::create_dir_all(&root).expect("create temporary directory");
        let missing = root.join("missing.png");
        assert_eq!(
            extract_metadata_from_source(&missing, "image-1", 0, None, 1, &AtomicBool::new(false))
                .expect_err("missing source should fail")
                .code,
            "source_missing"
        );
        let corrupt = root.join("corrupt.png");
        fs::write(&corrupt, b"not an image").expect("write corrupt source");
        assert_eq!(
            extract_metadata_from_source(&corrupt, "image-1", 0, None, 1, &AtomicBool::new(false))
                .expect_err("corrupt source should fail")
                .code,
            "metadata_decode_failed"
        );
        fs::remove_dir_all(root).expect("remove temporary directory");
    }
}
