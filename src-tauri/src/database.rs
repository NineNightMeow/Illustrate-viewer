use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{
    params, params_from_iter, types::Value, Connection, OptionalExtension, Transaction,
    TransactionBehavior,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{library::Library, scanner::ImageAsset};

const DATABASE_FILE_NAME: &str = "metadata.sqlite3";
const SCHEMA_VERSION: i64 = 9;

#[derive(Clone)]
pub struct MetadataDatabase {
    path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataDatabaseError {
    pub code: &'static str,
    pub message: String,
}

impl MetadataDatabaseError {
    fn initialization(message: impl Into<String>) -> Self {
        Self {
            code: "metadata_database_initialization_failed",
            message: message.into(),
        }
    }

    pub(crate) fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "metadata_database_operation_failed",
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataDatabaseStatus {
    pub schema_version: u32,
    pub library_count: u64,
    pub image_asset_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteCommandError {
    pub code: &'static str,
    pub message: String,
}

impl FavoriteCommandError {
    fn image_not_found() -> Self {
        Self {
            code: "favorite_image_not_found",
            message: "The image is no longer available in the metadata database.".into(),
        }
    }

    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "favorite_persistence_failed",
            message: error.to_string(),
        }
    }

    pub(crate) fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "favorite_persistence_failed",
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionCommandError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionMembership {
    pub collection_id: String,
    pub assigned_count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SmartCollectionRule {
    LargeImages { minimum_pixels: u64 },
    MinimumWidth { minimum_width: u32 },
    MinimumHeight { minimum_height: u32 },
    Format { extension: String },
    Landscape,
    Portrait,
    TransparentPng,
    Favorite,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartCollection {
    pub id: String,
    pub name: String,
    pub rule: SmartCollectionRule,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartCollectionCommandError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
}

/// Tags keyed by image id, loaded in one relation query for gallery/search windows.
pub type ImageTagsByImageId = HashMap<String, Vec<Tag>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCommandError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCommandError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadataFilter {
    pub minimum_width: Option<u32>,
    pub maximum_width: Option<u32>,
    pub minimum_height: Option<u32>,
    pub maximum_height: Option<u32>,
    pub minimum_aspect_ratio: Option<f64>,
    pub maximum_aspect_ratio: Option<f64>,
    pub minimum_file_size: Option<u64>,
    pub maximum_file_size: Option<u64>,
    pub has_alpha: Option<bool>,
    pub orientation: Option<ImageOrientation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageOrientation {
    Landscape,
    Portrait,
    Square,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSearchQuery {
    pub query: Option<String>,
    pub filename: Option<String>,
    pub path: Option<String>,
    pub formats: Vec<String>,
    #[serde(default)]
    pub metadata: ImageMetadataFilter,
    pub favorite: Option<bool>,
    pub smart_collection_id: Option<String>,
    pub library_id: Option<String>,
    pub collection_id: Option<String>,
    pub tag_ids: Vec<String>,
    pub favorites_only: bool,
}

pub type ImageFilter = ImageSearchQuery;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageColorMetadata {
    pub image_id: String,
    pub dominant_colors: Vec<Color>,
    pub color_count: u8,
    pub generated_at: i64,
    pub source_modified_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Color {
    pub hex: String,
    pub rgb: String,
    pub hsl: String,
    pub percentage: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    pub image_id: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub aspect_ratio: Option<f64>,
    pub file_size: Option<u64>,
    pub format: Option<String>,
    pub has_alpha: Option<bool>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub captured_at: Option<String>,
    pub orientation: Option<u16>,
    pub extraction_version: u32,
    pub status: String,
    pub failure_reason: Option<String>,
    pub extracted_at: i64,
    pub source_created_at: Option<i64>,
    pub source_modified_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadataError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFingerprint {
    pub image_id: String,
    pub content_hash: String,
    pub perceptual_hash: String,
    pub algorithm_version: u32,
    pub generated_at: i64,
    pub source_modified_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintMetadataError {
    pub code: &'static str,
    pub message: String,
}

impl FingerprintMetadataError {
    fn image_not_found() -> Self {
        Self {
            code: "fingerprint_image_not_found",
            message: "The image is no longer available in the metadata database.".into(),
        }
    }

    fn invalid_fingerprint() -> Self {
        Self {
            code: "invalid_image_fingerprint",
            message: "The image fingerprint is incomplete or invalid.".into(),
        }
    }

    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "fingerprint_persistence_failed",
            message: error.to_string(),
        }
    }
}

impl ImageMetadataError {
    fn image_not_found() -> Self {
        Self {
            code: "image_metadata_image_not_found",
            message: "The image is no longer available in the metadata database.".into(),
        }
    }

    fn invalid_metadata() -> Self {
        Self {
            code: "invalid_image_metadata",
            message: "Image metadata is incomplete or invalid for its extraction status.".into(),
        }
    }

    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "image_metadata_persistence_failed",
            message: error.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorMetadataError {
    pub code: &'static str,
    pub message: String,
}

impl ColorMetadataError {
    fn image_not_found() -> Self {
        Self {
            code: "color_metadata_image_not_found",
            message: "The image is no longer available in the metadata database.".into(),
        }
    }

    fn invalid_metadata() -> Self {
        Self {
            code: "invalid_color_metadata",
            message: "Color metadata must contain at most eight valid colors.".into(),
        }
    }

    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "color_metadata_persistence_failed",
            message: error.to_string(),
        }
    }
}

impl SearchCommandError {
    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "search_persistence_failed",
            message: format!("Unable to search image metadata: {error}"),
        }
    }

    pub fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "search_persistence_failed",
            message: message.into(),
        }
    }

    fn invalid_query(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_search_query",
            message: message.into(),
        }
    }

    fn smart_collection_not_found() -> Self {
        Self {
            code: "smart_collection_not_found",
            message: "The selected smart collection is no longer available.".into(),
        }
    }
}

impl TagCommandError {
    fn invalid_name() -> Self {
        Self {
            code: "invalid_tag_name",
            message: "A tag name is required.".into(),
        }
    }

    fn duplicate_name() -> Self {
        Self {
            code: "duplicate_tag_name",
            message: "A tag with this name already exists.".into(),
        }
    }

    fn tag_not_found() -> Self {
        Self {
            code: "tag_not_found",
            message: "The tag is no longer available in the metadata database.".into(),
        }
    }

    fn image_not_found() -> Self {
        Self {
            code: "tag_image_not_found",
            message: "The image is no longer available in the metadata database.".into(),
        }
    }

    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "tag_persistence_failed",
            message: error.to_string(),
        }
    }

    pub(crate) fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "tag_persistence_failed",
            message: message.into(),
        }
    }
}

impl CollectionCommandError {
    fn invalid_name() -> Self {
        Self {
            code: "invalid_collection_name",
            message: "A collection name is required.".into(),
        }
    }

    fn duplicate_name() -> Self {
        Self {
            code: "duplicate_collection_name",
            message: "A collection with this name already exists.".into(),
        }
    }

    fn empty_collection_selection() -> Self {
        Self {
            code: "empty_collection_selection",
            message: "Select at least one collection.".into(),
        }
    }

    fn empty_image_selection() -> Self {
        Self {
            code: "empty_image_selection",
            message: "Select at least one image.".into(),
        }
    }

    fn collection_not_found() -> Self {
        Self {
            code: "collection_not_found",
            message: "The collection is no longer available in the metadata database.".into(),
        }
    }

    fn image_not_found() -> Self {
        Self {
            code: "collection_image_not_found",
            message: "The image is no longer available in the metadata database.".into(),
        }
    }

    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "collection_persistence_failed",
            message: error.to_string(),
        }
    }

    pub(crate) fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "collection_persistence_failed",
            message: message.into(),
        }
    }
}

impl SmartCollectionCommandError {
    fn invalid_name() -> Self {
        Self {
            code: "invalid_smart_collection_name",
            message: "A smart collection name is required.".into(),
        }
    }

    fn duplicate_name() -> Self {
        Self {
            code: "duplicate_smart_collection_name",
            message: "A smart collection with this name already exists.".into(),
        }
    }

    fn invalid_rule() -> Self {
        Self {
            code: "invalid_smart_collection_rule",
            message: "The smart collection rule is incomplete or unsupported.".into(),
        }
    }

    fn not_found() -> Self {
        Self {
            code: "smart_collection_not_found",
            message: "The smart collection is no longer available in the metadata database.".into(),
        }
    }

    fn persistence(error: rusqlite::Error) -> Self {
        Self {
            code: "smart_collection_persistence_failed",
            message: error.to_string(),
        }
    }

    pub(crate) fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "smart_collection_persistence_failed",
            message: message.into(),
        }
    }
}

impl MetadataDatabase {
    pub fn initialize(app: &AppHandle) -> Result<Self, MetadataDatabaseError> {
        let app_data_dir = app.path().app_data_dir().map_err(|error| {
            MetadataDatabaseError::initialization(format!(
                "Unable to resolve the app data directory: {error}"
            ))
        })?;

        Self::open_at(app_data_dir.join(DATABASE_FILE_NAME))
    }

    fn open_at(path: PathBuf) -> Result<Self, MetadataDatabaseError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                MetadataDatabaseError::initialization(format!(
                    "Unable to create the metadata database directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }

        let database = Self { path };
        let database_path = database.path.display().to_string();
        let parent_path = database
            .path
            .parent()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<none>".to_owned());
        let mut connection = database.connection().map_err(|error| {
            MetadataDatabaseError::initialization(format!(
                "Unable to open or configure metadata database at '{database_path}': {} (parent directory: '{parent_path}'; SQLite mode: read-write/create; check directory/file permissions, file locks, and concurrent app instances)",
                error.message
            ))
        })?;
        let current_version = schema_version(&connection)?;
        migrate(&mut connection).map_err(|error| {
            MetadataDatabaseError::initialization(format!(
                "Database migration v{current_version} -> v{SCHEMA_VERSION} failed: {}",
                error.message
            ))
        })?;
        Ok(database)
    }

    pub fn sync_libraries(&self, libraries: &[Library]) -> Result<(), MetadataDatabaseError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_operation_error)?;

        for library in libraries {
            upsert_library(&transaction, library)?;
        }

        let retained_ids: HashSet<&str> = libraries
            .iter()
            .map(|library| library.id.as_str())
            .collect();
        let existing_ids = select_ids(&transaction, "SELECT id FROM libraries", None)?;
        for id in existing_ids {
            if !retained_ids.contains(id.as_str()) {
                transaction
                    .execute("DELETE FROM libraries WHERE id = ?1", [id])
                    .map_err(database_operation_error)?;
            }
        }

        transaction.commit().map_err(database_operation_error)
    }

    pub fn sync_library(&self, library: &Library) -> Result<(), MetadataDatabaseError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_operation_error)?;
        upsert_library(&transaction, library)?;
        transaction.commit().map_err(database_operation_error)
    }

    pub fn remove_library(&self, library_id: &str) -> Result<(), MetadataDatabaseError> {
        let connection = self.connection()?;
        connection
            .execute("DELETE FROM libraries WHERE id = ?1", [library_id])
            .map_err(database_operation_error)?;
        Ok(())
    }

    pub fn sync_images_for_library(
        &self,
        library_id: &str,
        assets: &[ImageAsset],
    ) -> Result<(), MetadataDatabaseError> {
        if assets.iter().any(|asset| asset.library_id != library_id) {
            return Err(MetadataDatabaseError::operation(
                "Refusing to sync image assets for a different library.",
            ));
        }

        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_operation_error)?;

        for asset in assets {
            let size = to_sql_integer(asset.size, "image size")?;
            let modified_at = to_sql_integer(asset.modified_at, "image modified time")?;
            transaction
                .execute(
                    r#"
                    INSERT INTO image_assets (
                        id, library_id, path, filename, extension, size, modified_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ON CONFLICT(id) DO UPDATE SET
                        library_id = excluded.library_id,
                        path = excluded.path,
                        filename = excluded.filename,
                        extension = excluded.extension,
                        size = excluded.size,
                        modified_at = excluded.modified_at
                    "#,
                    params![
                        &asset.id,
                        &asset.library_id,
                        &asset.path,
                        &asset.filename,
                        &asset.extension,
                        size,
                        modified_at,
                    ],
                )
                .map_err(database_operation_error)?;
        }

        let retained_ids: HashSet<&str> = assets.iter().map(|asset| asset.id.as_str()).collect();
        let existing_ids = select_ids(
            &transaction,
            "SELECT id FROM image_assets WHERE library_id = ?1",
            Some(library_id),
        )?;
        for id in existing_ids {
            if !retained_ids.contains(id.as_str()) {
                transaction
                    .execute("DELETE FROM image_assets WHERE id = ?1", [id])
                    .map_err(database_operation_error)?;
            }
        }

        transaction.commit().map_err(database_operation_error)
    }

    pub fn status(&self) -> Result<MetadataDatabaseStatus, MetadataDatabaseError> {
        let connection = self.connection()?;
        let schema_version = schema_version(&connection)?;
        let library_count = count_rows(&connection, "libraries")?;
        let image_asset_count = count_rows(&connection, "image_assets")?;

        Ok(MetadataDatabaseStatus {
            schema_version: schema_version as u32,
            library_count,
            image_asset_count,
        })
    }

    pub fn find_image_asset(
        &self,
        image_id: &str,
    ) -> Result<Option<ImageAsset>, MetadataDatabaseError> {
        let connection = self.connection()?;
        connection
            .query_row(
                r#"
                SELECT id, library_id, path, filename, extension, size, modified_at
                FROM image_assets
                WHERE id = ?1
                "#,
                [image_id],
                |row| {
                    Ok(ImageAsset {
                        id: row.get(0)?,
                        library_id: row.get(1)?,
                        path: row.get(2)?,
                        filename: row.get(3)?,
                        extension: row.get(4)?,
                        size: row.get(5)?,
                        created_at: None,
                        modified_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(database_operation_error)
    }

    pub fn find_image_assets_by_ids(
        &self,
        image_ids: &[String],
    ) -> Result<Vec<ImageAsset>, MetadataDatabaseError> {
        if image_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = (1..=image_ids.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT id, library_id, path, filename, extension, size, modified_at \
                 FROM image_assets WHERE id IN ({placeholders})"
            ))
            .map_err(database_operation_error)?;
        let assets = statement
            .query_map(params_from_iter(image_ids.iter()), |row| {
                Ok(ImageAsset {
                    id: row.get(0)?,
                    library_id: row.get(1)?,
                    path: row.get(2)?,
                    filename: row.get(3)?,
                    extension: row.get(4)?,
                    size: row.get(5)?,
                    created_at: None,
                    modified_at: row.get(6)?,
                })
            })
            .map_err(database_operation_error)?
            .collect::<Result<Vec<ImageAsset>, _>>()
            .map_err(database_operation_error)?;
        Ok(assets)
    }

    pub fn get_image_color_metadata(
        &self,
        image_id: &str,
    ) -> Result<Option<ImageColorMetadata>, ColorMetadataError> {
        let connection = self.connection().map_err(metadata_color_error)?;
        require_color_metadata_image(&connection, image_id)?;
        let metadata = connection
            .query_row(
                "SELECT color_count, generated_at, source_modified_at FROM image_color_metadata WHERE image_id = ?1",
                [image_id],
                |row| Ok((row.get::<_, u8>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
            )
            .optional()
            .map_err(ColorMetadataError::persistence)?;
        let Some((color_count, generated_at, source_modified_at)) = metadata else {
            return Ok(None);
        };

        let mut statement = connection
            .prepare(
                "SELECT hex, rgb, hsl, percentage FROM image_dominant_colors WHERE image_id = ?1 ORDER BY ordinal ASC",
            )
            .map_err(ColorMetadataError::persistence)?;
        let colors = statement
            .query_map([image_id], |row| {
                Ok(Color {
                    hex: row.get(0)?,
                    rgb: row.get(1)?,
                    hsl: row.get(2)?,
                    percentage: row.get(3)?,
                })
            })
            .map_err(ColorMetadataError::persistence)?
            .collect::<Result<Vec<Color>, _>>()
            .map_err(ColorMetadataError::persistence)?;
        Ok(Some(ImageColorMetadata {
            image_id: image_id.to_owned(),
            dominant_colors: colors,
            color_count,
            generated_at,
            source_modified_at,
        }))
    }

    pub fn replace_image_color_metadata(
        &self,
        metadata: &ImageColorMetadata,
    ) -> Result<(), ColorMetadataError> {
        validate_color_metadata(metadata)?;
        let mut connection = self.connection().map_err(metadata_color_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ColorMetadataError::persistence)?;
        require_color_metadata_image(&transaction, &metadata.image_id)?;
        transaction
            .execute(
                r#"
                INSERT INTO image_color_metadata (image_id, color_count, generated_at, source_modified_at)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(image_id) DO UPDATE SET
                    color_count = excluded.color_count,
                    generated_at = excluded.generated_at,
                    source_modified_at = excluded.source_modified_at
                "#,
                params![&metadata.image_id, metadata.color_count, metadata.generated_at, metadata.source_modified_at],
            )
            .map_err(ColorMetadataError::persistence)?;
        transaction
            .execute(
                "DELETE FROM image_dominant_colors WHERE image_id = ?1",
                [&metadata.image_id],
            )
            .map_err(ColorMetadataError::persistence)?;
        for (ordinal, color) in metadata.dominant_colors.iter().enumerate() {
            transaction
                .execute(
                    r#"
                    INSERT INTO image_dominant_colors (image_id, ordinal, hex, rgb, hsl, percentage)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    "#,
                    params![
                        &metadata.image_id,
                        ordinal as u8,
                        &color.hex,
                        &color.rgb,
                        &color.hsl,
                        color.percentage
                    ],
                )
                .map_err(ColorMetadataError::persistence)?;
        }
        transaction
            .commit()
            .map_err(ColorMetadataError::persistence)
    }

    pub fn get_image_metadata(
        &self,
        image_id: &str,
    ) -> Result<Option<ImageMetadata>, ImageMetadataError> {
        let connection = self.connection().map_err(metadata_image_error)?;
        require_image_metadata_image(&connection, image_id)?;
        connection
            .query_row(
                r#"
                SELECT
                    width,
                    height,
                    aspect_ratio,
                    file_size,
                    format,
                    has_alpha,
                    camera_make,
                    camera_model,
                    captured_at,
                    orientation,
                    extraction_version,
                    status,
                    failure_reason,
                    extracted_at,
                    source_created_at,
                    source_modified_at
                FROM image_metadata
                WHERE image_id = ?1
                "#,
                [image_id],
                |row| {
                    Ok(ImageMetadata {
                        image_id: image_id.to_owned(),
                        width: row.get(0)?,
                        height: row.get(1)?,
                        aspect_ratio: row.get(2)?,
                        file_size: row.get(3)?,
                        format: row.get(4)?,
                        has_alpha: row.get(5)?,
                        camera_make: row.get(6)?,
                        camera_model: row.get(7)?,
                        captured_at: row.get(8)?,
                        orientation: row.get(9)?,
                        extraction_version: row.get(10)?,
                        status: row.get(11)?,
                        failure_reason: row.get(12)?,
                        extracted_at: row.get(13)?,
                        source_created_at: row.get(14)?,
                        source_modified_at: row.get(15)?,
                    })
                },
            )
            .optional()
            .map_err(ImageMetadataError::persistence)
    }

    pub fn replace_image_metadata(
        &self,
        metadata: &ImageMetadata,
    ) -> Result<(), ImageMetadataError> {
        validate_image_metadata(metadata)?;
        let file_size = metadata
            .file_size
            .map(|size| to_sql_integer(size, "image metadata file size"))
            .transpose()
            .map_err(metadata_image_error)?;
        let mut connection = self.connection().map_err(metadata_image_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ImageMetadataError::persistence)?;
        require_image_metadata_image(&transaction, &metadata.image_id)?;
        transaction
            .execute(
                r#"
                INSERT INTO image_metadata (
                    image_id,
                    width,
                    height,
                    aspect_ratio,
                    file_size,
                    format,
                    has_alpha,
                    camera_make,
                    camera_model,
                    captured_at,
                    orientation,
                    extraction_version,
                    status,
                    failure_reason,
                    extracted_at,
                    source_created_at,
                    source_modified_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                ON CONFLICT(image_id) DO UPDATE SET
                    width = excluded.width,
                    height = excluded.height,
                    aspect_ratio = excluded.aspect_ratio,
                    file_size = excluded.file_size,
                    format = excluded.format,
                    has_alpha = excluded.has_alpha,
                    camera_make = excluded.camera_make,
                    camera_model = excluded.camera_model,
                    captured_at = excluded.captured_at,
                    orientation = excluded.orientation,
                    extraction_version = excluded.extraction_version,
                    status = excluded.status,
                    failure_reason = excluded.failure_reason,
                    extracted_at = excluded.extracted_at,
                    source_created_at = excluded.source_created_at,
                    source_modified_at = excluded.source_modified_at
                "#,
                params![
                    &metadata.image_id,
                    metadata.width,
                    metadata.height,
                    metadata.aspect_ratio,
                    file_size,
                    &metadata.format,
                    metadata.has_alpha,
                    &metadata.camera_make,
                    &metadata.camera_model,
                    &metadata.captured_at,
                    metadata.orientation,
                    metadata.extraction_version,
                    &metadata.status,
                    &metadata.failure_reason,
                    metadata.extracted_at,
                    metadata.source_created_at,
                    metadata.source_modified_at,
                ],
            )
            .map_err(ImageMetadataError::persistence)?;
        transaction
            .commit()
            .map_err(ImageMetadataError::persistence)
    }

    pub fn get_image_fingerprint(
        &self,
        image_id: &str,
    ) -> Result<Option<ImageFingerprint>, FingerprintMetadataError> {
        let connection = self.connection().map_err(metadata_fingerprint_error)?;
        require_fingerprint_image(&connection, image_id)?;
        connection
            .query_row(
                r#"
                SELECT content_hash, perceptual_hash, algorithm_version, generated_at, source_modified_at
                FROM image_fingerprints
                WHERE image_id = ?1
                "#,
                [image_id],
                |row| {
                    Ok(ImageFingerprint {
                        image_id: image_id.to_owned(),
                        content_hash: row.get(0)?,
                        perceptual_hash: row.get(1)?,
                        algorithm_version: row.get(2)?,
                        generated_at: row.get(3)?,
                        source_modified_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(FingerprintMetadataError::persistence)
    }

    pub fn replace_image_fingerprint(
        &self,
        fingerprint: &ImageFingerprint,
    ) -> Result<(), FingerprintMetadataError> {
        validate_image_fingerprint(fingerprint)?;
        let mut connection = self.connection().map_err(metadata_fingerprint_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(FingerprintMetadataError::persistence)?;
        require_fingerprint_image(&transaction, &fingerprint.image_id)?;
        transaction
            .execute(
                r#"
                INSERT INTO image_fingerprints (
                    image_id,
                    content_hash,
                    perceptual_hash,
                    algorithm_version,
                    generated_at,
                    source_modified_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(image_id) DO UPDATE SET
                    content_hash = excluded.content_hash,
                    perceptual_hash = excluded.perceptual_hash,
                    algorithm_version = excluded.algorithm_version,
                    generated_at = excluded.generated_at,
                    source_modified_at = excluded.source_modified_at
                "#,
                params![
                    &fingerprint.image_id,
                    &fingerprint.content_hash,
                    &fingerprint.perceptual_hash,
                    fingerprint.algorithm_version,
                    fingerprint.generated_at,
                    fingerprint.source_modified_at,
                ],
            )
            .map_err(FingerprintMetadataError::persistence)?;
        transaction
            .commit()
            .map_err(FingerprintMetadataError::persistence)
    }

    pub fn list_other_image_fingerprints(
        &self,
        image_id: &str,
    ) -> Result<Vec<ImageFingerprint>, FingerprintMetadataError> {
        let connection = self.connection().map_err(metadata_fingerprint_error)?;
        require_fingerprint_image(&connection, image_id)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT image_id, content_hash, perceptual_hash, algorithm_version, generated_at, source_modified_at
                FROM image_fingerprints
                WHERE image_id != ?1
                "#,
            )
            .map_err(FingerprintMetadataError::persistence)?;
        let fingerprints = statement
            .query_map([image_id], |row| {
                Ok(ImageFingerprint {
                    image_id: row.get(0)?,
                    content_hash: row.get(1)?,
                    perceptual_hash: row.get(2)?,
                    algorithm_version: row.get(3)?,
                    generated_at: row.get(4)?,
                    source_modified_at: row.get(5)?,
                })
            })
            .map_err(FingerprintMetadataError::persistence)?
            .collect::<Result<Vec<ImageFingerprint>, _>>()
            .map_err(FingerprintMetadataError::persistence)?;
        Ok(fingerprints)
    }

    pub fn list_image_fingerprints(
        &self,
    ) -> Result<Vec<ImageFingerprint>, FingerprintMetadataError> {
        let connection = self.connection().map_err(metadata_fingerprint_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT image_id, content_hash, perceptual_hash, algorithm_version, generated_at, source_modified_at
                FROM image_fingerprints
                ORDER BY image_id ASC
                "#,
            )
            .map_err(FingerprintMetadataError::persistence)?;
        let fingerprints = statement
            .query_map([], |row| {
                Ok(ImageFingerprint {
                    image_id: row.get(0)?,
                    content_hash: row.get(1)?,
                    perceptual_hash: row.get(2)?,
                    algorithm_version: row.get(3)?,
                    generated_at: row.get(4)?,
                    source_modified_at: row.get(5)?,
                })
            })
            .map_err(FingerprintMetadataError::persistence)?
            .collect::<Result<Vec<ImageFingerprint>, _>>()
            .map_err(FingerprintMetadataError::persistence)?;
        Ok(fingerprints)
    }

    pub fn add_favorite(&self, image_id: &str) -> Result<(), FavoriteCommandError> {
        let connection = self.favorite_connection()?;
        let image_exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM image_assets WHERE id = ?1)",
                [image_id],
                |row| row.get(0),
            )
            .map_err(FavoriteCommandError::persistence)?;
        if image_exists == 0 {
            return Err(FavoriteCommandError::image_not_found());
        }

        connection
            .execute(
                "INSERT OR IGNORE INTO favorites (id, image_id, created_at) VALUES (?1, ?2, ?3)",
                params![Uuid::new_v4().to_string(), image_id, favorite_timestamp()?],
            )
            .map_err(FavoriteCommandError::persistence)?;
        Ok(())
    }

    pub fn remove_favorite(&self, image_id: &str) -> Result<(), FavoriteCommandError> {
        let connection = self.favorite_connection()?;
        connection
            .execute("DELETE FROM favorites WHERE image_id = ?1", [image_id])
            .map_err(FavoriteCommandError::persistence)?;
        Ok(())
    }

    pub fn is_favorite(&self, image_id: &str) -> Result<bool, FavoriteCommandError> {
        let connection = self.favorite_connection()?;
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM favorites WHERE image_id = ?1)",
                [image_id],
                |row| row.get(0),
            )
            .map_err(FavoriteCommandError::persistence)?;
        Ok(exists != 0)
    }

    pub fn list_favorites(&self) -> Result<Vec<ImageAsset>, FavoriteCommandError> {
        let connection = self.favorite_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT image_assets.id, image_assets.library_id, image_assets.path,
                    image_assets.filename, image_assets.extension, image_assets.size,
                    image_assets.modified_at
                FROM favorites
                INNER JOIN image_assets ON image_assets.id = favorites.image_id
                ORDER BY favorites.created_at DESC, favorites.id DESC
                "#,
            )
            .map_err(FavoriteCommandError::persistence)?;
        let rows = statement
            .query_map([], |row| {
                Ok(ImageAsset {
                    id: row.get(0)?,
                    library_id: row.get(1)?,
                    path: row.get(2)?,
                    filename: row.get(3)?,
                    extension: row.get(4)?,
                    size: row.get(5)?,
                    created_at: None,
                    modified_at: row.get(6)?,
                })
            })
            .map_err(FavoriteCommandError::persistence)?;

        rows.collect::<Result<Vec<ImageAsset>, _>>()
            .map_err(FavoriteCommandError::persistence)
    }

    pub fn create_collection(&self, name: &str) -> Result<Collection, CollectionCommandError> {
        let name = collection_name(name)?;
        let timestamp = collection_timestamp()?;
        let collection = Collection {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
            created_at: timestamp,
            updated_at: timestamp,
        };
        let mut connection = self.collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(CollectionCommandError::persistence)?;
        if collection_name_exists(&transaction, name, None)? {
            return Err(CollectionCommandError::duplicate_name());
        }
        transaction
            .execute(
                "INSERT INTO collections (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    &collection.id,
                    &collection.name,
                    collection.created_at,
                    collection.updated_at,
                ],
            )
            .map_err(CollectionCommandError::persistence)?;
        transaction
            .commit()
            .map_err(CollectionCommandError::persistence)?;
        Ok(collection)
    }

    pub fn delete_collection(&self, collection_id: &str) -> Result<(), CollectionCommandError> {
        let mut connection = self.collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(CollectionCommandError::persistence)?;
        require_collection(&transaction, collection_id)?;
        transaction
            .execute("DELETE FROM collections WHERE id = ?1", [collection_id])
            .map_err(CollectionCommandError::persistence)?;
        transaction
            .commit()
            .map_err(CollectionCommandError::persistence)
    }

    pub fn rename_collection(
        &self,
        collection_id: &str,
        name: &str,
    ) -> Result<Collection, CollectionCommandError> {
        let name = collection_name(name)?;
        let updated_at = collection_timestamp()?;
        let mut connection = self.collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(CollectionCommandError::persistence)?;
        require_collection(&transaction, collection_id)?;
        if collection_name_exists(&transaction, name, Some(collection_id))? {
            return Err(CollectionCommandError::duplicate_name());
        }
        transaction
            .execute(
                "UPDATE collections SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, updated_at, collection_id],
            )
            .map_err(CollectionCommandError::persistence)?;
        let collection = transaction
            .query_row(
                "SELECT id, name, created_at, updated_at FROM collections WHERE id = ?1",
                [collection_id],
                collection_from_row,
            )
            .map_err(CollectionCommandError::persistence)?;
        transaction
            .commit()
            .map_err(CollectionCommandError::persistence)?;
        Ok(collection)
    }

    pub fn list_collections(&self) -> Result<Vec<Collection>, CollectionCommandError> {
        let connection = self.collection_connection()?;
        let mut statement = connection
            .prepare("SELECT id, name, created_at, updated_at FROM collections ORDER BY updated_at DESC, id DESC")
            .map_err(CollectionCommandError::persistence)?;
        let rows = statement
            .query_map([], collection_from_row)
            .map_err(CollectionCommandError::persistence)?;
        rows.collect::<Result<Vec<Collection>, _>>()
            .map_err(CollectionCommandError::persistence)
    }

    pub fn add_image_to_collection(
        &self,
        collection_id: &str,
        image_id: &str,
    ) -> Result<(), CollectionCommandError> {
        let mut connection = self.collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(CollectionCommandError::persistence)?;
        require_collection(&transaction, collection_id)?;
        require_image(&transaction, image_id)?;
        let inserted = transaction
            .execute(
                "INSERT OR IGNORE INTO collection_items (id, collection_id, image_id, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    Uuid::new_v4().to_string(),
                    collection_id,
                    image_id,
                    collection_timestamp()?,
                ],
            )
            .map_err(CollectionCommandError::persistence)?;
        if inserted == 1 {
            transaction
                .execute(
                    "UPDATE collections SET updated_at = ?1 WHERE id = ?2",
                    params![collection_timestamp()?, collection_id],
                )
                .map_err(CollectionCommandError::persistence)?;
        }
        transaction
            .commit()
            .map_err(CollectionCommandError::persistence)
    }

    pub fn list_collection_memberships(
        &self,
        collection_ids: &[String],
        image_ids: &[String],
    ) -> Result<Vec<CollectionMembership>, CollectionCommandError> {
        let collection_ids = unique_ids(collection_ids);
        let image_ids = unique_ids(image_ids);
        if collection_ids.is_empty() {
            return Err(CollectionCommandError::empty_collection_selection());
        }
        if image_ids.is_empty() {
            return Err(CollectionCommandError::empty_image_selection());
        }
        let connection = self.collection_connection()?;
        for collection_id in &collection_ids {
            require_collection(&connection, collection_id)?;
        }
        for image_id in &image_ids {
            require_image(&connection, image_id)?;
        }
        let image_placeholders = std::iter::repeat("?")
            .take(image_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let mut statement = connection
            .prepare(&format!(
                "SELECT collection_id, COUNT(*) FROM collection_items WHERE collection_id IN ({}) AND image_id IN ({}) GROUP BY collection_id",
                std::iter::repeat("?").take(collection_ids.len()).collect::<Vec<_>>().join(","),
                image_placeholders,
            ))
            .map_err(CollectionCommandError::persistence)?;
        let values = collection_ids
            .iter()
            .chain(image_ids.iter())
            .copied()
            .collect::<Vec<_>>();
        let rows = statement
            .query_map(params_from_iter(values), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
            })
            .map_err(CollectionCommandError::persistence)?;
        let counts = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(CollectionCommandError::persistence)?
            .into_iter()
            .collect::<HashMap<_, _>>();
        Ok(collection_ids
            .into_iter()
            .map(|collection_id| CollectionMembership {
                collection_id: collection_id.to_owned(),
                assigned_count: counts.get(collection_id).copied().unwrap_or_default(),
            })
            .collect())
    }

    pub fn add_images_to_collections(
        &self,
        collection_ids: &[String],
        image_ids: &[String],
    ) -> Result<(), CollectionCommandError> {
        let collection_ids = unique_ids(collection_ids);
        let image_ids = unique_ids(image_ids);
        if collection_ids.is_empty() {
            return Err(CollectionCommandError::empty_collection_selection());
        }
        if image_ids.is_empty() {
            return Err(CollectionCommandError::empty_image_selection());
        }
        let mut connection = self.collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(CollectionCommandError::persistence)?;
        for collection_id in &collection_ids {
            require_collection(&transaction, collection_id)?;
        }
        for image_id in &image_ids {
            require_image(&transaction, image_id)?;
        }
        let timestamp = collection_timestamp()?;
        for collection_id in &collection_ids {
            let mut inserted = 0;
            for image_id in &image_ids {
                inserted += transaction
                    .execute(
                        "INSERT OR IGNORE INTO collection_items (id, collection_id, image_id, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![Uuid::new_v4().to_string(), collection_id, image_id, timestamp],
                    )
                    .map_err(CollectionCommandError::persistence)?;
            }
            if inserted > 0 {
                transaction
                    .execute(
                        "UPDATE collections SET updated_at = ?1 WHERE id = ?2",
                        params![timestamp, collection_id],
                    )
                    .map_err(CollectionCommandError::persistence)?;
            }
        }
        transaction
            .commit()
            .map_err(CollectionCommandError::persistence)
    }

    pub fn create_collection_with_images(
        &self,
        name: &str,
        image_ids: &[String],
    ) -> Result<Collection, CollectionCommandError> {
        let name = collection_name(name)?;
        let image_ids = unique_ids(image_ids);
        if image_ids.is_empty() {
            return Err(CollectionCommandError::empty_image_selection());
        }
        let timestamp = collection_timestamp()?;
        let mut connection = self.collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(CollectionCommandError::persistence)?;
        for image_id in &image_ids {
            require_image(&transaction, image_id)?;
        }
        if collection_name_exists(&transaction, name, None)? {
            return Err(CollectionCommandError::duplicate_name());
        }
        let collection = Collection {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
            created_at: timestamp,
            updated_at: timestamp,
        };
        transaction
            .execute(
                "INSERT INTO collections (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![&collection.id, &collection.name, timestamp, timestamp],
            )
            .map_err(CollectionCommandError::persistence)?;
        for image_id in image_ids {
            transaction
                .execute(
                    "INSERT INTO collection_items (id, collection_id, image_id, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![Uuid::new_v4().to_string(), &collection.id, image_id, timestamp],
                )
                .map_err(CollectionCommandError::persistence)?;
        }
        transaction
            .commit()
            .map_err(CollectionCommandError::persistence)?;
        Ok(collection)
    }

    pub fn remove_image_from_collection(
        &self,
        collection_id: &str,
        image_id: &str,
    ) -> Result<(), CollectionCommandError> {
        let mut connection = self.collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(CollectionCommandError::persistence)?;
        require_collection(&transaction, collection_id)?;
        require_image(&transaction, image_id)?;
        let removed = transaction
            .execute(
                "DELETE FROM collection_items WHERE collection_id = ?1 AND image_id = ?2",
                params![collection_id, image_id],
            )
            .map_err(CollectionCommandError::persistence)?;
        if removed == 1 {
            transaction
                .execute(
                    "UPDATE collections SET updated_at = ?1 WHERE id = ?2",
                    params![collection_timestamp()?, collection_id],
                )
                .map_err(CollectionCommandError::persistence)?;
        }
        transaction
            .commit()
            .map_err(CollectionCommandError::persistence)
    }

    pub fn list_collection_images(
        &self,
        collection_id: &str,
    ) -> Result<Vec<ImageAsset>, CollectionCommandError> {
        let connection = self.collection_connection()?;
        require_collection(&connection, collection_id)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT image_assets.id, image_assets.library_id, image_assets.path,
                    image_assets.filename, image_assets.extension, image_assets.size,
                    image_assets.modified_at
                FROM collection_items
                INNER JOIN image_assets ON image_assets.id = collection_items.image_id
                WHERE collection_items.collection_id = ?1
                ORDER BY collection_items.created_at ASC, collection_items.id ASC
                "#,
            )
            .map_err(CollectionCommandError::persistence)?;
        let rows = statement
            .query_map([collection_id], |row| {
                Ok(ImageAsset {
                    id: row.get(0)?,
                    library_id: row.get(1)?,
                    path: row.get(2)?,
                    filename: row.get(3)?,
                    extension: row.get(4)?,
                    size: row.get(5)?,
                    created_at: None,
                    modified_at: row.get(6)?,
                })
            })
            .map_err(CollectionCommandError::persistence)?;
        rows.collect::<Result<Vec<ImageAsset>, _>>()
            .map_err(CollectionCommandError::persistence)
    }

    pub fn create_smart_collection(
        &self,
        name: &str,
        rule: SmartCollectionRule,
    ) -> Result<SmartCollection, SmartCollectionCommandError> {
        let name = smart_collection_name(name)?;
        let rule = normalize_smart_collection_rule(rule)?;
        let connection = self.smart_collection_connection()?;
        if smart_collection_name_exists(&connection, name, None)? {
            return Err(SmartCollectionCommandError::duplicate_name());
        }
        let timestamp = smart_collection_timestamp()?;
        let collection = SmartCollection {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
            rule,
            created_at: timestamp,
            updated_at: timestamp,
        };
        let rule_payload = serde_json::to_string(&collection.rule)
            .map_err(|error| SmartCollectionCommandError::operation(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO smart_collections (id, name, rule_payload, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &collection.id,
                    &collection.name,
                    rule_payload,
                    collection.created_at,
                    collection.updated_at,
                ],
            )
            .map_err(SmartCollectionCommandError::persistence)?;
        Ok(collection)
    }

    pub fn delete_smart_collection(
        &self,
        collection_id: &str,
    ) -> Result<(), SmartCollectionCommandError> {
        let connection = self.smart_collection_connection()?;
        let deleted = connection
            .execute(
                "DELETE FROM smart_collections WHERE id = ?1",
                [collection_id],
            )
            .map_err(SmartCollectionCommandError::persistence)?;
        if deleted == 0 {
            return Err(SmartCollectionCommandError::not_found());
        }
        Ok(())
    }

    pub fn rename_smart_collection(
        &self,
        collection_id: &str,
        name: &str,
    ) -> Result<SmartCollection, SmartCollectionCommandError> {
        let name = smart_collection_name(name)?;
        let mut connection = self.smart_collection_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(SmartCollectionCommandError::persistence)?;
        require_smart_collection(&transaction, collection_id)?;
        if smart_collection_name_exists(&transaction, name, Some(collection_id))? {
            return Err(SmartCollectionCommandError::duplicate_name());
        }
        transaction
            .execute(
                "UPDATE smart_collections SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, smart_collection_timestamp()?, collection_id],
            )
            .map_err(SmartCollectionCommandError::persistence)?;
        let values = transaction
            .query_row(
                "SELECT id, name, rule_payload, created_at, updated_at FROM smart_collections WHERE id = ?1",
                [collection_id],
                smart_collection_values_from_row,
            )
            .map_err(SmartCollectionCommandError::persistence)?;
        transaction
            .commit()
            .map_err(SmartCollectionCommandError::persistence)?;
        smart_collection_from_values(values)
    }

    pub fn list_smart_collections(
        &self,
    ) -> Result<Vec<SmartCollection>, SmartCollectionCommandError> {
        let connection = self.smart_collection_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, rule_payload, created_at, updated_at FROM smart_collections ORDER BY updated_at DESC, id DESC",
            )
            .map_err(SmartCollectionCommandError::persistence)?;
        let rows = statement
            .query_map([], smart_collection_values_from_row)
            .map_err(SmartCollectionCommandError::persistence)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(SmartCollectionCommandError::persistence)?;
        rows.into_iter().map(smart_collection_from_values).collect()
    }

    pub fn list_smart_collection_images(
        &self,
        collection_id: &str,
    ) -> Result<Vec<ImageAsset>, SmartCollectionCommandError> {
        let connection = self.smart_collection_connection()?;
        let values = connection
            .query_row(
                "SELECT id, name, rule_payload, created_at, updated_at FROM smart_collections WHERE id = ?1",
                [collection_id],
                smart_collection_values_from_row,
            )
            .optional()
            .map_err(SmartCollectionCommandError::persistence)?
            .ok_or_else(SmartCollectionCommandError::not_found)?;
        let collection = smart_collection_from_values(values)?;
        smart_collection_images(&connection, &collection.rule)
    }

    pub fn create_tag(&self, name: &str) -> Result<Tag, TagCommandError> {
        let name = tag_name(name)?;
        let connection = self.tag_connection()?;
        if tag_name_exists(&connection, name, None)? {
            return Err(TagCommandError::duplicate_name());
        }

        let tag = Tag {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
        };
        connection
            .execute(
                "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                params![&tag.id, &tag.name],
            )
            .map_err(TagCommandError::persistence)?;
        Ok(tag)
    }

    pub fn delete_tag(&self, tag_id: &str) -> Result<(), TagCommandError> {
        let mut connection = self.tag_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(TagCommandError::persistence)?;
        require_tag(&transaction, tag_id)?;
        transaction
            .execute("DELETE FROM tags WHERE id = ?1", [tag_id])
            .map_err(TagCommandError::persistence)?;
        transaction.commit().map_err(TagCommandError::persistence)
    }

    pub fn rename_tag(&self, tag_id: &str, name: &str) -> Result<Tag, TagCommandError> {
        let name = tag_name(name)?;
        let mut connection = self.tag_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(TagCommandError::persistence)?;
        require_tag(&transaction, tag_id)?;
        if tag_name_exists(&transaction, name, Some(tag_id))? {
            return Err(TagCommandError::duplicate_name());
        }
        transaction
            .execute(
                "UPDATE tags SET name = ?1 WHERE id = ?2",
                params![name, tag_id],
            )
            .map_err(TagCommandError::persistence)?;
        let tag = transaction
            .query_row(
                "SELECT id, name FROM tags WHERE id = ?1",
                [tag_id],
                tag_from_row,
            )
            .map_err(TagCommandError::persistence)?;
        transaction.commit().map_err(TagCommandError::persistence)?;
        Ok(tag)
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>, TagCommandError> {
        let connection = self.tag_connection()?;
        let mut statement = connection
            .prepare("SELECT id, name FROM tags ORDER BY name COLLATE NOCASE ASC, id ASC")
            .map_err(TagCommandError::persistence)?;
        let rows = statement
            .query_map([], tag_from_row)
            .map_err(TagCommandError::persistence)?;
        rows.collect::<Result<Vec<Tag>, _>>()
            .map_err(TagCommandError::persistence)
    }

    pub fn add_tag_to_image(&self, tag_id: &str, image_id: &str) -> Result<(), TagCommandError> {
        let mut connection = self.tag_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(TagCommandError::persistence)?;
        require_tag(&transaction, tag_id)?;
        require_tag_image(&transaction, image_id)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO image_tags (image_id, tag_id) VALUES (?1, ?2)",
                params![image_id, tag_id],
            )
            .map_err(TagCommandError::persistence)?;
        transaction.commit().map_err(TagCommandError::persistence)
    }

    pub fn remove_tag_from_image(
        &self,
        tag_id: &str,
        image_id: &str,
    ) -> Result<(), TagCommandError> {
        let mut connection = self.tag_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(TagCommandError::persistence)?;
        require_tag(&transaction, tag_id)?;
        require_tag_image(&transaction, image_id)?;
        transaction
            .execute(
                "DELETE FROM image_tags WHERE image_id = ?1 AND tag_id = ?2",
                params![image_id, tag_id],
            )
            .map_err(TagCommandError::persistence)?;
        transaction.commit().map_err(TagCommandError::persistence)
    }

    pub fn list_image_tags(&self, image_id: &str) -> Result<Vec<Tag>, TagCommandError> {
        let connection = self.tag_connection()?;
        require_tag_image(&connection, image_id)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT tags.id, tags.name
                FROM image_tags
                INNER JOIN tags ON tags.id = image_tags.tag_id
                WHERE image_tags.image_id = ?1
                ORDER BY tags.name COLLATE NOCASE ASC, tags.id ASC
                "#,
            )
            .map_err(TagCommandError::persistence)?;
        let rows = statement
            .query_map([image_id], tag_from_row)
            .map_err(TagCommandError::persistence)?;
        rows.collect::<Result<Vec<Tag>, _>>()
            .map_err(TagCommandError::persistence)
    }

    pub fn list_image_tags_for_images(
        &self,
        image_ids: &[String],
    ) -> Result<ImageTagsByImageId, TagCommandError> {
        let ids = image_ids
            .iter()
            .filter(|image_id| !image_id.is_empty())
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        let connection = self.tag_connection()?;
        let mut result = HashMap::<String, Vec<Tag>>::new();
        for ids in ids.chunks(900) {
            let placeholders = (1..=ids.len())
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "SELECT image_tags.image_id, tags.id, tags.name FROM image_tags INNER JOIN tags ON tags.id = image_tags.tag_id WHERE image_tags.image_id IN ({placeholders}) ORDER BY image_tags.image_id, tags.name COLLATE NOCASE ASC, tags.id ASC"
            );
            let mut statement = connection
                .prepare(&sql)
                .map_err(TagCommandError::persistence)?;
            let mut rows = statement
                .query(rusqlite::params_from_iter(ids.iter()))
                .map_err(TagCommandError::persistence)?;
            while let Some(row) = rows.next().map_err(TagCommandError::persistence)? {
                let image_id: String = row.get(0).map_err(TagCommandError::persistence)?;
                result.entry(image_id).or_default().push(Tag {
                    id: row.get(1).map_err(TagCommandError::persistence)?,
                    name: row.get(2).map_err(TagCommandError::persistence)?,
                });
            }
        }
        Ok(result)
    }

    pub fn list_tag_images(&self, tag_id: &str) -> Result<Vec<ImageAsset>, TagCommandError> {
        let connection = self.tag_connection()?;
        require_tag(&connection, tag_id)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT image_assets.id, image_assets.library_id, image_assets.path,
                    image_assets.filename, image_assets.extension, image_assets.size,
                    image_assets.modified_at
                FROM image_tags
                INNER JOIN image_assets ON image_assets.id = image_tags.image_id
                WHERE image_tags.tag_id = ?1
                ORDER BY image_assets.filename COLLATE NOCASE ASC, image_assets.id ASC
                "#,
            )
            .map_err(TagCommandError::persistence)?;
        let rows = statement
            .query_map([tag_id], |row| {
                Ok(ImageAsset {
                    id: row.get(0)?,
                    library_id: row.get(1)?,
                    path: row.get(2)?,
                    filename: row.get(3)?,
                    extension: row.get(4)?,
                    size: row.get(5)?,
                    created_at: None,
                    modified_at: row.get(6)?,
                })
            })
            .map_err(TagCommandError::persistence)?;
        rows.collect::<Result<Vec<ImageAsset>, _>>()
            .map_err(TagCommandError::persistence)
    }

    pub fn search_images(&self, query: &str) -> Result<Vec<ImageAsset>, SearchCommandError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        self.filter_images(&ImageFilter {
            query: Some(query.to_owned()),
            ..ImageFilter::default()
        })
    }

    pub fn filter_images(
        &self,
        filters: &ImageFilter,
    ) -> Result<Vec<ImageAsset>, SearchCommandError> {
        self.query_image_assets(filters)
    }

    pub fn query_image_assets(
        &self,
        query: &ImageSearchQuery,
    ) -> Result<Vec<ImageAsset>, SearchCommandError> {
        validate_image_search_query(query)?;
        let connection = self.connection().map_err(metadata_search_error)?;
        let mut values = Vec::<Value>::new();
        let mut conditions = Vec::<String>::new();
        if let Some(query) = query
            .query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            let placeholder =
                next_search_placeholder(&mut values, format!("%{}%", escape_like_pattern(query)));
            conditions.push(format!(
                r#"(image_assets.filename LIKE {placeholder} ESCAPE '\'
                    OR libraries.name LIKE {placeholder} ESCAPE '\'
                    OR EXISTS (
                        SELECT 1
                        FROM collection_items
                        INNER JOIN collections ON collections.id = collection_items.collection_id
                        WHERE collection_items.image_id = image_assets.id
                            AND collections.name LIKE {placeholder} ESCAPE '\'
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM image_tags
                        INNER JOIN tags ON tags.id = image_tags.tag_id
                        WHERE image_tags.image_id = image_assets.id
                            AND tags.name LIKE {placeholder} ESCAPE '\'
                    ))"#
            ));
        }
        if let Some(filename) = query
            .filename
            .as_deref()
            .map(str::trim)
            .filter(|filename| !filename.is_empty())
        {
            let placeholder = next_search_placeholder(
                &mut values,
                format!("%{}%", escape_like_pattern(filename)),
            );
            conditions.push(format!(
                "image_assets.filename LIKE {placeholder} ESCAPE '\\'"
            ));
        }
        if let Some(path) = query
            .path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let placeholder =
                next_search_placeholder(&mut values, format!("%{}%", escape_like_pattern(path)));
            conditions.push(format!("image_assets.path LIKE {placeholder} ESCAPE '\\'"));
        }
        let mut seen_formats = HashSet::new();
        let mut format_placeholders = Vec::new();
        for format in query
            .formats
            .iter()
            .map(|format| format.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|format| !format.is_empty())
        {
            for format in format_aliases(&format) {
                if seen_formats.insert(format.clone()) {
                    format_placeholders.push(next_search_placeholder(&mut values, format));
                }
            }
        }
        if !format_placeholders.is_empty() {
            conditions.push(format!(
                "LOWER(image_assets.extension) IN ({})",
                format_placeholders.join(", ")
            ));
        }
        if query.metadata.has_conditions() {
            conditions.push("image_metadata.status = 'ready'".to_owned());
            add_metadata_search_conditions(&query.metadata, &mut conditions, &mut values)?;
        }
        if let Some(favorite) = query.favorite {
            conditions.push(favorite_search_condition(favorite));
        } else if query.favorites_only {
            conditions.push(favorite_search_condition(true));
        }
        if let Some(smart_collection_id) = query
            .smart_collection_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            let rule = search_smart_collection_rule(&connection, smart_collection_id)?;
            let (condition, value) = smart_collection_rule_condition(&rule);
            let placeholder = next_search_value_placeholder(&mut values, value);
            conditions.push(condition.replace("?1", &placeholder));
        }
        if let Some(library_id) = query
            .library_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            let placeholder = next_search_placeholder(&mut values, library_id.to_owned());
            conditions.push(format!("image_assets.library_id = {placeholder}"));
        }
        if let Some(collection_id) = query
            .collection_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            let placeholder = next_search_placeholder(&mut values, collection_id.to_owned());
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM collection_items WHERE collection_items.image_id = image_assets.id AND collection_items.collection_id = {placeholder})"
            ));
        }
        let mut seen_tag_ids = HashSet::new();
        for tag_id in query
            .tag_ids
            .iter()
            .map(|id| id.trim())
            .filter(|id| !id.is_empty())
        {
            if !seen_tag_ids.insert(tag_id) {
                continue;
            }
            let placeholder = next_search_placeholder(&mut values, tag_id.to_owned());
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM image_tags WHERE image_tags.image_id = image_assets.id AND image_tags.tag_id = {placeholder})"
            ));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        let mut statement = connection
            .prepare(&format!(
                r#"
                SELECT image_assets.id, image_assets.library_id, image_assets.path,
                    image_assets.filename, image_assets.extension, image_assets.size,
                    image_assets.modified_at
                FROM image_assets
                INNER JOIN libraries ON libraries.id = image_assets.library_id
                LEFT JOIN image_metadata ON image_metadata.image_id = image_assets.id
                {where_clause}
                ORDER BY image_assets.filename COLLATE NOCASE ASC, image_assets.id ASC
                "#
            ))
            .map_err(SearchCommandError::persistence)?;
        let rows = statement
            .query_map(params_from_iter(values), |row| {
                Ok(ImageAsset {
                    id: row.get(0)?,
                    library_id: row.get(1)?,
                    path: row.get(2)?,
                    filename: row.get(3)?,
                    extension: row.get(4)?,
                    size: row.get(5)?,
                    created_at: None,
                    modified_at: row.get(6)?,
                })
            })
            .map_err(SearchCommandError::persistence)?;
        rows.collect::<Result<Vec<ImageAsset>, _>>()
            .map_err(SearchCommandError::persistence)
    }

    fn connection(&self) -> Result<Connection, MetadataDatabaseError> {
        let connection = Connection::open(&self.path).map_err(database_operation_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database_operation_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(database_operation_error)?;
        Ok(connection)
    }

    fn favorite_connection(&self) -> Result<Connection, FavoriteCommandError> {
        let connection = Connection::open(&self.path).map_err(FavoriteCommandError::persistence)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(FavoriteCommandError::persistence)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(FavoriteCommandError::persistence)?;
        Ok(connection)
    }

    fn collection_connection(&self) -> Result<Connection, CollectionCommandError> {
        let connection =
            Connection::open(&self.path).map_err(CollectionCommandError::persistence)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(CollectionCommandError::persistence)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(CollectionCommandError::persistence)?;
        Ok(connection)
    }

    fn smart_collection_connection(&self) -> Result<Connection, SmartCollectionCommandError> {
        let connection =
            Connection::open(&self.path).map_err(SmartCollectionCommandError::persistence)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(SmartCollectionCommandError::persistence)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(SmartCollectionCommandError::persistence)?;
        Ok(connection)
    }

    fn tag_connection(&self) -> Result<Connection, TagCommandError> {
        let connection = Connection::open(&self.path).map_err(TagCommandError::persistence)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(TagCommandError::persistence)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(TagCommandError::persistence)?;
        Ok(connection)
    }
}

fn migrate(connection: &mut Connection) -> Result<(), MetadataDatabaseError> {
    let current_version = schema_version(connection)?;
    if current_version > SCHEMA_VERSION {
        return Err(MetadataDatabaseError::initialization(format!(
            "Metadata database schema version {current_version} is newer than this app supports."
        )));
    }

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_operation_error)?;

    if current_version == 0 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE libraries (
                    id TEXT PRIMARY KEY NOT NULL,
                    path TEXT NOT NULL COLLATE NOCASE UNIQUE,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );

                CREATE TABLE image_assets (
                    id TEXT PRIMARY KEY NOT NULL,
                    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    extension TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    modified_at INTEGER NOT NULL,
                    UNIQUE(library_id, path)
                );

                CREATE TABLE favorites (
                    id TEXT PRIMARY KEY NOT NULL,
                    image_id TEXT NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    created_at INTEGER NOT NULL,
                    UNIQUE(image_id)
                );

                CREATE TABLE collections (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );

                CREATE TABLE collection_items (
                    id TEXT PRIMARY KEY NOT NULL,
                    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
                    image_id TEXT NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    created_at INTEGER NOT NULL,
                    UNIQUE(collection_id, image_id)
                );

                CREATE TABLE tags (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL COLLATE NOCASE UNIQUE
                );

                CREATE TABLE image_tags (
                    image_id TEXT NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                    PRIMARY KEY(image_id, tag_id)
                );

                CREATE TABLE image_color_metadata (
                    image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    color_count INTEGER NOT NULL CHECK (color_count BETWEEN 0 AND 8),
                    generated_at INTEGER NOT NULL,
                    source_modified_at INTEGER NOT NULL
                );

                CREATE TABLE image_dominant_colors (
                    image_id TEXT NOT NULL REFERENCES image_color_metadata(image_id) ON DELETE CASCADE,
                    ordinal INTEGER NOT NULL CHECK (ordinal BETWEEN 0 AND 7),
                    hex TEXT NOT NULL,
                    rgb TEXT NOT NULL,
                    hsl TEXT NOT NULL,
                    percentage REAL NOT NULL,
                    PRIMARY KEY(image_id, ordinal)
                );

                CREATE TABLE image_metadata (
                    image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    width INTEGER CHECK (width IS NULL OR width > 0),
                    height INTEGER CHECK (height IS NULL OR height > 0),
                    aspect_ratio REAL CHECK (aspect_ratio IS NULL OR aspect_ratio > 0),
                    file_size INTEGER CHECK (file_size IS NULL OR file_size >= 0),
                    format TEXT,
                    has_alpha INTEGER CHECK (has_alpha IS NULL OR has_alpha IN (0, 1)),
                    camera_make TEXT,
                    camera_model TEXT,
                    captured_at TEXT,
                    orientation INTEGER,
                    extraction_version INTEGER NOT NULL CHECK (extraction_version > 0),
                    status TEXT NOT NULL CHECK (status IN ('ready', 'failed')),
                    failure_reason TEXT,
                    extracted_at INTEGER NOT NULL CHECK (extracted_at >= 0),
                    source_created_at INTEGER CHECK (source_created_at IS NULL OR source_created_at >= 0),
                    source_modified_at INTEGER NOT NULL CHECK (source_modified_at >= 0)
                );

                CREATE TABLE image_fingerprints (
                    image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    content_hash TEXT NOT NULL,
                    perceptual_hash TEXT NOT NULL,
                    algorithm_version INTEGER NOT NULL CHECK (algorithm_version > 0),
                    generated_at INTEGER NOT NULL CHECK (generated_at >= 0),
                    source_modified_at INTEGER NOT NULL CHECK (source_modified_at >= 0)
                );

                CREATE TABLE smart_collections (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                    rule_payload TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );

                CREATE INDEX image_assets_library_id_index ON image_assets(library_id);
                CREATE INDEX collection_items_collection_id_index ON collection_items(collection_id);
                CREATE INDEX collection_items_image_id_index ON collection_items(image_id);
                CREATE INDEX image_tags_tag_id_index ON image_tags(tag_id);
                CREATE INDEX image_fingerprints_content_hash_index ON image_fingerprints(content_hash);
                CREATE INDEX image_metadata_dimensions_index ON image_metadata(width, height);
                CREATE INDEX image_metadata_transparency_index ON image_metadata(has_alpha);
                "#,
            )
            .map_err(database_operation_error)?;
    }

    if current_version == 1 {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE collection_items ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;
                CREATE INDEX IF NOT EXISTS collection_items_image_id_index ON collection_items(image_id);
                "#,
            )
            .map_err(database_operation_error)?;
    }

    if current_version > 0 && current_version < 3 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS tags (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL COLLATE NOCASE UNIQUE
                );

                CREATE TABLE IF NOT EXISTS image_tags (
                    image_id TEXT NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                    PRIMARY KEY(image_id, tag_id)
                );

                CREATE INDEX IF NOT EXISTS image_tags_tag_id_index ON image_tags(tag_id);
                "#,
            )
            .map_err(database_operation_error)?;
    }

    if current_version > 0 && current_version < 4 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS image_color_metadata (
                    image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    color_count INTEGER NOT NULL CHECK (color_count BETWEEN 0 AND 8),
                    generated_at INTEGER NOT NULL,
                    source_modified_at INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS image_dominant_colors (
                    image_id TEXT NOT NULL REFERENCES image_color_metadata(image_id) ON DELETE CASCADE,
                    ordinal INTEGER NOT NULL CHECK (ordinal BETWEEN 0 AND 7),
                    hex TEXT NOT NULL,
                    rgb TEXT NOT NULL,
                    hsl TEXT NOT NULL,
                    percentage REAL NOT NULL,
                    PRIMARY KEY(image_id, ordinal)
                );
                "#,
            )
            .map_err(database_operation_error)?;
    }

    if current_version == 4 {
        ensure_column(
            &transaction,
            "image_color_metadata",
            "source_modified_at",
            "INTEGER NOT NULL DEFAULT -1",
        )?;
    }

    if current_version > 0 && current_version < 6 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS image_metadata (
                    image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    width INTEGER CHECK (width IS NULL OR width > 0),
                    height INTEGER CHECK (height IS NULL OR height > 0),
                    aspect_ratio REAL CHECK (aspect_ratio IS NULL OR aspect_ratio > 0),
                    file_size INTEGER CHECK (file_size IS NULL OR file_size >= 0),
                    format TEXT,
                    camera_make TEXT,
                    camera_model TEXT,
                    captured_at TEXT,
                    orientation INTEGER,
                    extraction_version INTEGER NOT NULL CHECK (extraction_version > 0),
                    status TEXT NOT NULL CHECK (status IN ('ready', 'failed')),
                    failure_reason TEXT,
                    extracted_at INTEGER NOT NULL CHECK (extracted_at >= 0),
                    source_modified_at INTEGER NOT NULL CHECK (source_modified_at >= 0)
                );
                "#,
            )
            .map_err(database_operation_error)?;
    }

    if current_version > 0 && current_version < 7 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS image_fingerprints (
                    image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    content_hash TEXT NOT NULL,
                    perceptual_hash TEXT NOT NULL,
                    algorithm_version INTEGER NOT NULL CHECK (algorithm_version > 0),
                    generated_at INTEGER NOT NULL CHECK (generated_at >= 0),
                    source_modified_at INTEGER NOT NULL CHECK (source_modified_at >= 0)
                );
                CREATE INDEX IF NOT EXISTS image_fingerprints_content_hash_index
                    ON image_fingerprints(content_hash);
                "#,
            )
            .map_err(database_operation_error)?;
    }

    if current_version > 0 && current_version < 8 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS smart_collections (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                    rule_payload TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                "#,
            )
            .map_err(database_operation_error)?;
    }

    // Repair databases produced by interrupted or older migrations. A database
    // can report the latest user_version while still missing a table or column.
    repair_current_schema(&transaction)?;
    validate_current_schema(&transaction, current_version)?;

    transaction
        .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))
        .map_err(database_operation_error)?;
    transaction.commit().map_err(database_operation_error)
}

fn repair_current_schema(transaction: &Transaction<'_>) -> Result<(), MetadataDatabaseError> {
    transaction
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS image_metadata (
                image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                width INTEGER CHECK (width IS NULL OR width > 0),
                height INTEGER CHECK (height IS NULL OR height > 0),
                aspect_ratio REAL CHECK (aspect_ratio IS NULL OR aspect_ratio > 0),
                file_size INTEGER CHECK (file_size IS NULL OR file_size >= 0),
                format TEXT,
                has_alpha INTEGER CHECK (has_alpha IS NULL OR has_alpha IN (0, 1)),
                camera_make TEXT,
                camera_model TEXT,
                captured_at TEXT,
                orientation INTEGER,
                extraction_version INTEGER NOT NULL DEFAULT 1 CHECK (extraction_version > 0),
                status TEXT NOT NULL DEFAULT 'failed' CHECK (status IN ('ready', 'failed')),
                failure_reason TEXT,
                extracted_at INTEGER NOT NULL DEFAULT 0 CHECK (extracted_at >= 0),
                source_created_at INTEGER CHECK (source_created_at IS NULL OR source_created_at >= 0),
                source_modified_at INTEGER NOT NULL DEFAULT 0 CHECK (source_modified_at >= 0)
            );

            CREATE TABLE IF NOT EXISTS image_fingerprints (
                image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                content_hash TEXT NOT NULL,
                perceptual_hash TEXT NOT NULL,
                algorithm_version INTEGER NOT NULL CHECK (algorithm_version > 0),
                generated_at INTEGER NOT NULL CHECK (generated_at >= 0),
                source_modified_at INTEGER NOT NULL CHECK (source_modified_at >= 0)
            );

            CREATE TABLE IF NOT EXISTS smart_collections (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                rule_payload TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS image_fingerprints_content_hash_index ON image_fingerprints(content_hash);
            "#,
        )
        .map_err(database_operation_error)?;

    ensure_column(
        transaction,
        "image_metadata",
        "aspect_ratio",
        "REAL CHECK (aspect_ratio IS NULL OR aspect_ratio > 0)",
    )?;
    ensure_column(
        transaction,
        "image_metadata",
        "file_size",
        "INTEGER CHECK (file_size IS NULL OR file_size >= 0)",
    )?;
    ensure_column(transaction, "image_metadata", "format", "TEXT")?;
    ensure_column(
        transaction,
        "image_metadata",
        "has_alpha",
        "INTEGER CHECK (has_alpha IS NULL OR has_alpha IN (0, 1))",
    )?;
    ensure_column(transaction, "image_metadata", "camera_make", "TEXT")?;
    ensure_column(transaction, "image_metadata", "camera_model", "TEXT")?;
    ensure_column(transaction, "image_metadata", "captured_at", "TEXT")?;
    ensure_column(transaction, "image_metadata", "orientation", "INTEGER")?;
    ensure_column(
        transaction,
        "image_metadata",
        "extraction_version",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column(transaction, "image_metadata", "failure_reason", "TEXT")?;
    ensure_column(
        transaction,
        "image_metadata",
        "extracted_at",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        transaction,
        "image_metadata",
        "source_created_at",
        "INTEGER",
    )?;
    ensure_column(
        transaction,
        "image_metadata",
        "source_modified_at",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    transaction
        .execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS image_metadata_dimensions_index ON image_metadata(width, height);
            CREATE INDEX IF NOT EXISTS image_metadata_transparency_index ON image_metadata(has_alpha);
            "#,
        )
        .map_err(database_operation_error)?;
    Ok(())
}

fn validate_current_schema(
    transaction: &Transaction<'_>,
    previous_version: i64,
) -> Result<(), MetadataDatabaseError> {
    for table in ["image_metadata", "image_fingerprints", "smart_collections"] {
        let exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get(0),
            )
            .map_err(database_operation_error)?;
        if exists == 0 {
            return Err(MetadataDatabaseError::initialization(format!(
                "Database migration v{previous_version} -> v{SCHEMA_VERSION} failed during schema validation: required table {table} is missing."
            )));
        }
    }

    for column in [
        "image_id",
        "width",
        "height",
        "aspect_ratio",
        "file_size",
        "format",
        "has_alpha",
        "extraction_version",
        "status",
        "extracted_at",
        "source_modified_at",
    ] {
        let exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('image_metadata') WHERE name = ?1)",
                [column],
                |row| row.get(0),
            )
            .map_err(database_operation_error)?;
        if exists == 0 {
            return Err(MetadataDatabaseError::initialization(format!(
                "Database migration v{previous_version} -> v{SCHEMA_VERSION} failed during schema validation: image_metadata.{column} is missing."
            )));
        }
    }
    Ok(())
}

fn ensure_column(
    transaction: &Transaction<'_>,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), MetadataDatabaseError> {
    let exists: i64 = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
            rusqlite::params![table, column],
            |row| row.get(0),
        )
        .map_err(database_operation_error)?;
    if exists == 0 {
        transaction
            .execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition};"
            ))
            .map_err(database_operation_error)?;
    }
    Ok(())
}

fn upsert_library(
    transaction: &Transaction<'_>,
    library: &Library,
) -> Result<(), MetadataDatabaseError> {
    let created_at = to_sql_integer(library.created_at, "library creation time")?;
    let updated_at = to_sql_integer(library.updated_at, "library update time")?;
    transaction
        .execute(
            r#"
            INSERT INTO libraries (id, path, name, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                path = excluded.path,
                name = excluded.name,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at
            "#,
            params![
                &library.id,
                &library.path,
                &library.name,
                created_at,
                updated_at
            ],
        )
        .map_err(database_operation_error)?;
    Ok(())
}

fn select_ids(
    transaction: &Transaction<'_>,
    query: &str,
    library_id: Option<&str>,
) -> Result<Vec<String>, MetadataDatabaseError> {
    let mut statement = transaction
        .prepare(query)
        .map_err(database_operation_error)?;
    let rows = match library_id {
        Some(library_id) => statement.query_map([library_id], read_id),
        None => statement.query_map([], read_id),
    }
    .map_err(database_operation_error)?;

    rows.collect::<Result<Vec<String>, _>>()
        .map_err(database_operation_error)
}

fn read_id(row: &rusqlite::Row<'_>) -> rusqlite::Result<String> {
    row.get(0)
}

fn schema_version(connection: &Connection) -> Result<i64, MetadataDatabaseError> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(database_operation_error)
}

fn count_rows(connection: &Connection, table: &str) -> Result<u64, MetadataDatabaseError> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    let count: i64 = connection
        .query_row(&query, [], |row| row.get(0))
        .map_err(database_operation_error)?;
    Ok(count as u64)
}

fn to_sql_integer(value: u64, field: &str) -> Result<i64, MetadataDatabaseError> {
    i64::try_from(value).map_err(|_| {
        MetadataDatabaseError::operation(format!("The {field} exceeds SQLite's INTEGER range."))
    })
}

fn favorite_timestamp() -> Result<i64, FavoriteCommandError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FavoriteCommandError::operation("The system clock is invalid."))?
        .as_millis();
    i64::try_from(timestamp).map_err(|_| {
        FavoriteCommandError::operation("The favorite timestamp exceeds SQLite's INTEGER range.")
    })
}

fn collection_name(name: &str) -> Result<&str, CollectionCommandError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CollectionCommandError::invalid_name());
    }
    Ok(name)
}

fn unique_ids(ids: &[String]) -> Vec<&str> {
    let mut seen = HashSet::new();
    ids.iter()
        .filter_map(|id| seen.insert(id.as_str()).then_some(id.as_str()))
        .collect()
}

fn collection_timestamp() -> Result<i64, CollectionCommandError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CollectionCommandError::operation("The system clock is invalid."))?
        .as_millis();
    i64::try_from(timestamp).map_err(|_| {
        CollectionCommandError::operation(
            "The collection timestamp exceeds SQLite's INTEGER range.",
        )
    })
}

fn require_collection(
    connection: &Connection,
    collection_id: &str,
) -> Result<(), CollectionCommandError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM collections WHERE id = ?1)",
            [collection_id],
            |row| row.get(0),
        )
        .map_err(CollectionCommandError::persistence)?;
    if exists == 0 {
        return Err(CollectionCommandError::collection_not_found());
    }
    Ok(())
}

fn collection_name_exists(
    connection: &Connection,
    name: &str,
    except_collection_id: Option<&str>,
) -> Result<bool, CollectionCommandError> {
    let exists: i64 = match except_collection_id {
        Some(collection_id) => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM collections WHERE name = ?1 COLLATE NOCASE AND id != ?2)",
            params![name, collection_id],
            |row| row.get(0),
        ),
        None => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM collections WHERE name = ?1 COLLATE NOCASE)",
            [name],
            |row| row.get(0),
        ),
    }
    .map_err(CollectionCommandError::persistence)?;
    Ok(exists != 0)
}

fn require_image(connection: &Connection, image_id: &str) -> Result<(), CollectionCommandError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM image_assets WHERE id = ?1)",
            [image_id],
            |row| row.get(0),
        )
        .map_err(CollectionCommandError::persistence)?;
    if exists == 0 {
        return Err(CollectionCommandError::image_not_found());
    }
    Ok(())
}

fn collection_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Collection> {
    Ok(Collection {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn smart_collection_name(name: &str) -> Result<&str, SmartCollectionCommandError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(SmartCollectionCommandError::invalid_name());
    }
    Ok(name)
}

fn smart_collection_timestamp() -> Result<i64, SmartCollectionCommandError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SmartCollectionCommandError::operation("The system clock is invalid."))?
        .as_millis();
    i64::try_from(timestamp).map_err(|_| {
        SmartCollectionCommandError::operation(
            "The smart collection timestamp exceeds SQLite's INTEGER range.",
        )
    })
}

fn normalize_smart_collection_rule(
    rule: SmartCollectionRule,
) -> Result<SmartCollectionRule, SmartCollectionCommandError> {
    match rule {
        SmartCollectionRule::LargeImages { minimum_pixels }
            if (1..=1_000_000_000).contains(&minimum_pixels) =>
        {
            Ok(SmartCollectionRule::LargeImages { minimum_pixels })
        }
        SmartCollectionRule::MinimumWidth { minimum_width } if minimum_width > 0 => {
            Ok(SmartCollectionRule::MinimumWidth { minimum_width })
        }
        SmartCollectionRule::MinimumHeight { minimum_height } if minimum_height > 0 => {
            Ok(SmartCollectionRule::MinimumHeight { minimum_height })
        }
        SmartCollectionRule::Format { extension } => {
            let extension = extension
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase();
            matches!(extension.as_str(), "gif" | "jpeg" | "jpg" | "png" | "webp")
                .then_some(SmartCollectionRule::Format { extension })
                .ok_or_else(SmartCollectionCommandError::invalid_rule)
        }
        SmartCollectionRule::Landscape => Ok(SmartCollectionRule::Landscape),
        SmartCollectionRule::Portrait
        | SmartCollectionRule::TransparentPng
        | SmartCollectionRule::Favorite => Ok(rule),
        SmartCollectionRule::LargeImages { .. }
        | SmartCollectionRule::MinimumWidth { .. }
        | SmartCollectionRule::MinimumHeight { .. } => {
            Err(SmartCollectionCommandError::invalid_rule())
        }
    }
}

fn smart_collection_name_exists(
    connection: &Connection,
    name: &str,
    except_collection_id: Option<&str>,
) -> Result<bool, SmartCollectionCommandError> {
    let exists: i64 = match except_collection_id {
        Some(collection_id) => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM smart_collections WHERE name = ?1 COLLATE NOCASE AND id != ?2)",
            params![name, collection_id],
            |row| row.get(0),
        ),
        None => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM smart_collections WHERE name = ?1 COLLATE NOCASE)",
            [name],
            |row| row.get(0),
        ),
    }
    .map_err(SmartCollectionCommandError::persistence)?;
    Ok(exists != 0)
}

fn require_smart_collection(
    connection: &Connection,
    collection_id: &str,
) -> Result<(), SmartCollectionCommandError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM smart_collections WHERE id = ?1)",
            [collection_id],
            |row| row.get(0),
        )
        .map_err(SmartCollectionCommandError::persistence)?;
    if exists == 0 {
        return Err(SmartCollectionCommandError::not_found());
    }
    Ok(())
}

fn smart_collection_values_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, String, i64, i64)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn smart_collection_from_values(
    (id, name, rule_payload, created_at, updated_at): (String, String, String, i64, i64),
) -> Result<SmartCollection, SmartCollectionCommandError> {
    let rule = serde_json::from_str(&rule_payload)
        .map_err(|_| SmartCollectionCommandError::invalid_rule())?;
    Ok(SmartCollection {
        id,
        name,
        rule: normalize_smart_collection_rule(rule)?,
        created_at,
        updated_at,
    })
}

fn smart_collection_images(
    connection: &Connection,
    rule: &SmartCollectionRule,
) -> Result<Vec<ImageAsset>, SmartCollectionCommandError> {
    let (condition, value) = smart_collection_rule_condition(rule);
    let mut statement = connection
        .prepare(&format!(
            r#"
            SELECT image_assets.id, image_assets.library_id, image_assets.path,
                image_assets.filename, image_assets.extension, image_assets.size,
                image_assets.modified_at
            FROM image_assets
            LEFT JOIN image_metadata ON image_metadata.image_id = image_assets.id
            WHERE {condition}
            ORDER BY image_assets.filename COLLATE NOCASE ASC, image_assets.id ASC
            "#
        ))
        .map_err(SmartCollectionCommandError::persistence)?;
    let rows = statement
        .query_map(params_from_iter([value]), |row| {
            Ok(ImageAsset {
                id: row.get(0)?,
                library_id: row.get(1)?,
                path: row.get(2)?,
                filename: row.get(3)?,
                extension: row.get(4)?,
                size: row.get(5)?,
                created_at: None,
                modified_at: row.get(6)?,
            })
        })
        .map_err(SmartCollectionCommandError::persistence)?;
    rows.collect::<Result<Vec<ImageAsset>, _>>()
        .map_err(SmartCollectionCommandError::persistence)
}

fn smart_collection_rule_condition(rule: &SmartCollectionRule) -> (&'static str, Value) {
    match rule {
        SmartCollectionRule::LargeImages { minimum_pixels } => (
            "image_metadata.status = 'ready' AND CAST(image_metadata.width AS REAL) * CAST(image_metadata.height AS REAL) >= ?1",
            Value::Real(*minimum_pixels as f64),
        ),
        SmartCollectionRule::MinimumWidth { minimum_width } => (
            "image_metadata.status = 'ready' AND image_metadata.width >= ?1",
            Value::Integer(i64::from(*minimum_width)),
        ),
        SmartCollectionRule::MinimumHeight { minimum_height } => (
            "image_metadata.status = 'ready' AND image_metadata.height >= ?1",
            Value::Integer(i64::from(*minimum_height)),
        ),
        SmartCollectionRule::Format { extension } => (
            "LOWER(image_assets.extension) = ?1",
            Value::Text(extension.clone()),
        ),
        SmartCollectionRule::Landscape => (
            "image_metadata.status = 'ready' AND image_metadata.width > image_metadata.height AND ?1 = 1",
            Value::Integer(1),
        ),
        SmartCollectionRule::Portrait => (
            "image_metadata.status = 'ready' AND image_metadata.height > image_metadata.width AND ?1 = 1",
            Value::Integer(1),
        ),
        SmartCollectionRule::TransparentPng => (
            "image_metadata.status = 'ready' AND image_metadata.has_alpha = 1 AND LOWER(image_assets.extension) = 'png' AND ?1 = 1",
            Value::Integer(1),
        ),
        SmartCollectionRule::Favorite => (
            "EXISTS(SELECT 1 FROM favorites WHERE favorites.image_id = image_assets.id) AND ?1 = 1",
            Value::Integer(1),
        ),
    }
}

fn tag_name(name: &str) -> Result<&str, TagCommandError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(TagCommandError::invalid_name());
    }
    Ok(name)
}

fn tag_name_exists(
    connection: &Connection,
    name: &str,
    except_tag_id: Option<&str>,
) -> Result<bool, TagCommandError> {
    let exists: i64 = match except_tag_id {
        Some(tag_id) => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1 COLLATE NOCASE AND id != ?2)",
            params![name, tag_id],
            |row| row.get(0),
        ),
        None => connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1 COLLATE NOCASE)",
            [name],
            |row| row.get(0),
        ),
    }
    .map_err(TagCommandError::persistence)?;
    Ok(exists != 0)
}

fn require_tag(connection: &Connection, tag_id: &str) -> Result<(), TagCommandError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
            [tag_id],
            |row| row.get(0),
        )
        .map_err(TagCommandError::persistence)?;
    if exists == 0 {
        return Err(TagCommandError::tag_not_found());
    }
    Ok(())
}

fn require_tag_image(connection: &Connection, image_id: &str) -> Result<(), TagCommandError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM image_assets WHERE id = ?1)",
            [image_id],
            |row| row.get(0),
        )
        .map_err(TagCommandError::persistence)?;
    if exists == 0 {
        return Err(TagCommandError::image_not_found());
    }
    Ok(())
}

fn tag_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
    })
}

fn require_color_metadata_image(
    connection: &Connection,
    image_id: &str,
) -> Result<(), ColorMetadataError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM image_assets WHERE id = ?1)",
            [image_id],
            |row| row.get(0),
        )
        .map_err(ColorMetadataError::persistence)?;
    if exists == 0 {
        return Err(ColorMetadataError::image_not_found());
    }
    Ok(())
}

fn require_image_metadata_image(
    connection: &Connection,
    image_id: &str,
) -> Result<(), ImageMetadataError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM image_assets WHERE id = ?1)",
            [image_id],
            |row| row.get(0),
        )
        .map_err(ImageMetadataError::persistence)?;
    if exists == 0 {
        return Err(ImageMetadataError::image_not_found());
    }
    Ok(())
}

fn require_fingerprint_image(
    connection: &Connection,
    image_id: &str,
) -> Result<(), FingerprintMetadataError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM image_assets WHERE id = ?1)",
            [image_id],
            |row| row.get(0),
        )
        .map_err(FingerprintMetadataError::persistence)?;
    if exists == 0 {
        return Err(FingerprintMetadataError::image_not_found());
    }
    Ok(())
}

fn validate_color_metadata(metadata: &ImageColorMetadata) -> Result<(), ColorMetadataError> {
    if metadata.image_id.trim().is_empty()
        || metadata.generated_at < 0
        || metadata.source_modified_at < 0
        || metadata.dominant_colors.len() > 8
        || metadata.color_count as usize != metadata.dominant_colors.len()
        || metadata.dominant_colors.iter().any(|color| {
            color.hex.len() != 7
                || !color.hex.starts_with('#')
                || !color.hex[1..]
                    .bytes()
                    .all(|value| value.is_ascii_hexdigit())
                || color.rgb.trim().is_empty()
                || color.hsl.trim().is_empty()
                || !color.percentage.is_finite()
                || !(0.0..=100.0).contains(&color.percentage)
        })
    {
        return Err(ColorMetadataError::invalid_metadata());
    }
    Ok(())
}

fn validate_image_metadata(metadata: &ImageMetadata) -> Result<(), ImageMetadataError> {
    let has_non_empty = |value: &Option<String>| {
        value
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    };
    let valid_optional_text = |value: &Option<String>| {
        value
            .as_deref()
            .is_none_or(|value| !value.trim().is_empty())
    };
    let valid_common_fields = !metadata.image_id.trim().is_empty()
        && metadata.extraction_version > 0
        && metadata.extracted_at >= 0
        && metadata.source_modified_at >= 0
        && metadata
            .source_created_at
            .is_none_or(|timestamp| timestamp >= 0)
        && valid_optional_text(&metadata.format)
        && valid_optional_text(&metadata.camera_make)
        && valid_optional_text(&metadata.camera_model)
        && valid_optional_text(&metadata.captured_at);

    let valid_status = match metadata.status.as_str() {
        "ready" => {
            let (Some(width), Some(height), Some(aspect_ratio), Some(_), Some(format)) = (
                metadata.width,
                metadata.height,
                metadata.aspect_ratio,
                metadata.file_size,
                metadata.format.as_deref(),
            ) else {
                return Err(ImageMetadataError::invalid_metadata());
            };
            width > 0
                && height > 0
                && aspect_ratio.is_finite()
                && aspect_ratio > 0.0
                && (aspect_ratio - f64::from(width) / f64::from(height)).abs() < 0.000_001
                && !format.trim().is_empty()
                && metadata.has_alpha.is_some()
                && metadata.failure_reason.is_none()
        }
        "failed" => has_non_empty(&metadata.failure_reason),
        _ => false,
    };

    if !valid_common_fields || !valid_status {
        return Err(ImageMetadataError::invalid_metadata());
    }
    Ok(())
}

fn validate_image_fingerprint(
    fingerprint: &ImageFingerprint,
) -> Result<(), FingerprintMetadataError> {
    if fingerprint.image_id.trim().is_empty()
        || fingerprint.content_hash.len() != 64
        || !fingerprint
            .content_hash
            .bytes()
            .all(|value| value.is_ascii_hexdigit())
        || fingerprint.perceptual_hash.len() != 16
        || !fingerprint
            .perceptual_hash
            .bytes()
            .all(|value| value.is_ascii_hexdigit())
        || fingerprint.algorithm_version == 0
        || fingerprint.generated_at < 0
        || fingerprint.source_modified_at < 0
    {
        return Err(FingerprintMetadataError::invalid_fingerprint());
    }
    Ok(())
}

fn escape_like_pattern(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl ImageMetadataFilter {
    fn has_conditions(&self) -> bool {
        self.minimum_width.is_some()
            || self.maximum_width.is_some()
            || self.minimum_height.is_some()
            || self.maximum_height.is_some()
            || self.minimum_aspect_ratio.is_some()
            || self.maximum_aspect_ratio.is_some()
            || self.minimum_file_size.is_some()
            || self.maximum_file_size.is_some()
            || self.has_alpha.is_some()
            || self.orientation.is_some()
    }
}

fn validate_image_search_query(query: &ImageSearchQuery) -> Result<(), SearchCommandError> {
    let metadata = &query.metadata;
    if query.favorites_only && query.favorite == Some(false) {
        return Err(SearchCommandError::invalid_query(
            "A favorites-only search cannot exclude favorites.",
        ));
    }
    validate_search_range(metadata.minimum_width, metadata.maximum_width, "width")?;
    validate_search_range(metadata.minimum_height, metadata.maximum_height, "height")?;
    validate_search_range(
        metadata.minimum_file_size,
        metadata.maximum_file_size,
        "file size",
    )?;
    validate_search_decimal_range(
        metadata.minimum_aspect_ratio,
        metadata.maximum_aspect_ratio,
        "aspect ratio",
    )
}

fn validate_search_range<T: Ord>(
    minimum: Option<T>,
    maximum: Option<T>,
    label: &str,
) -> Result<(), SearchCommandError> {
    if minimum
        .zip(maximum)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(SearchCommandError::invalid_query(format!(
            "The minimum {label} cannot exceed the maximum {label}."
        )));
    }
    Ok(())
}

fn validate_search_decimal_range(
    minimum: Option<f64>,
    maximum: Option<f64>,
    label: &str,
) -> Result<(), SearchCommandError> {
    if minimum.is_some_and(|value| !value.is_finite() || value <= 0.0)
        || maximum.is_some_and(|value| !value.is_finite() || value <= 0.0)
        || minimum
            .zip(maximum)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(SearchCommandError::invalid_query(format!(
            "The {label} range is invalid."
        )));
    }
    Ok(())
}

fn add_metadata_search_conditions(
    metadata: &ImageMetadataFilter,
    conditions: &mut Vec<String>,
    values: &mut Vec<Value>,
) -> Result<(), SearchCommandError> {
    add_optional_search_integer_condition(
        metadata.minimum_width.map(i64::from),
        "image_metadata.width >=",
        conditions,
        values,
    );
    add_optional_search_integer_condition(
        metadata.maximum_width.map(i64::from),
        "image_metadata.width <=",
        conditions,
        values,
    );
    add_optional_search_integer_condition(
        metadata.minimum_height.map(i64::from),
        "image_metadata.height >=",
        conditions,
        values,
    );
    add_optional_search_integer_condition(
        metadata.maximum_height.map(i64::from),
        "image_metadata.height <=",
        conditions,
        values,
    );
    add_optional_search_decimal_condition(
        metadata.minimum_aspect_ratio,
        "image_metadata.aspect_ratio >=",
        conditions,
        values,
    );
    add_optional_search_decimal_condition(
        metadata.maximum_aspect_ratio,
        "image_metadata.aspect_ratio <=",
        conditions,
        values,
    );
    for (value, operator) in [
        (metadata.minimum_file_size, "image_metadata.file_size >="),
        (metadata.maximum_file_size, "image_metadata.file_size <="),
    ] {
        if let Some(value) = value {
            let value =
                to_sql_integer(value, "image metadata file size").map_err(metadata_search_error)?;
            add_optional_search_integer_condition(Some(value), operator, conditions, values);
        }
    }
    if let Some(has_alpha) = metadata.has_alpha {
        add_optional_search_integer_condition(
            Some(i64::from(has_alpha)),
            "image_metadata.has_alpha =",
            conditions,
            values,
        );
    }
    if let Some(orientation) = &metadata.orientation {
        conditions.push(
            match orientation {
                ImageOrientation::Landscape => "image_metadata.width > image_metadata.height",
                ImageOrientation::Portrait => "image_metadata.height > image_metadata.width",
                ImageOrientation::Square => "image_metadata.width = image_metadata.height",
            }
            .to_owned(),
        );
    }
    Ok(())
}

fn add_optional_search_integer_condition(
    value: Option<i64>,
    operator: &str,
    conditions: &mut Vec<String>,
    values: &mut Vec<Value>,
) {
    if let Some(value) = value {
        let placeholder = next_search_value_placeholder(values, Value::Integer(value));
        conditions.push(format!("{operator} {placeholder}"));
    }
}

fn add_optional_search_decimal_condition(
    value: Option<f64>,
    operator: &str,
    conditions: &mut Vec<String>,
    values: &mut Vec<Value>,
) {
    if let Some(value) = value {
        let placeholder = next_search_value_placeholder(values, Value::Real(value));
        conditions.push(format!("{operator} {placeholder}"));
    }
}

fn format_aliases(format: &str) -> Vec<String> {
    match format {
        "jpg" | "jpeg" => vec!["jpg".to_owned(), "jpeg".to_owned()],
        "tif" | "tiff" => vec!["tif".to_owned(), "tiff".to_owned()],
        format => vec![format.to_owned()],
    }
}

fn favorite_search_condition(favorite: bool) -> String {
    let exists = "EXISTS (SELECT 1 FROM favorites WHERE favorites.image_id = image_assets.id)";
    if favorite {
        exists.to_owned()
    } else {
        format!("NOT {exists}")
    }
}

fn search_smart_collection_rule(
    connection: &Connection,
    collection_id: &str,
) -> Result<SmartCollectionRule, SearchCommandError> {
    let payload = connection
        .query_row(
            "SELECT rule_payload FROM smart_collections WHERE id = ?1",
            [collection_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(SearchCommandError::persistence)?
        .ok_or_else(SearchCommandError::smart_collection_not_found)?;
    let rule = serde_json::from_str(&payload).map_err(|_| {
        SearchCommandError::invalid_query("The selected smart collection has an invalid rule.")
    })?;
    normalize_smart_collection_rule(rule).map_err(|_| {
        SearchCommandError::invalid_query("The selected smart collection has an invalid rule.")
    })
}

fn next_search_placeholder(values: &mut Vec<Value>, value: String) -> String {
    next_search_value_placeholder(values, Value::Text(value))
}

fn next_search_value_placeholder(values: &mut Vec<Value>, value: Value) -> String {
    values.push(value);
    format!("?{}", values.len())
}

fn metadata_search_error(error: MetadataDatabaseError) -> SearchCommandError {
    SearchCommandError {
        code: "search_persistence_failed",
        message: error.message,
    }
}

fn metadata_color_error(error: MetadataDatabaseError) -> ColorMetadataError {
    ColorMetadataError {
        code: "color_metadata_persistence_failed",
        message: error.message,
    }
}

fn metadata_image_error(error: MetadataDatabaseError) -> ImageMetadataError {
    ImageMetadataError {
        code: "image_metadata_persistence_failed",
        message: error.message,
    }
}

fn metadata_fingerprint_error(error: MetadataDatabaseError) -> FingerprintMetadataError {
    FingerprintMetadataError {
        code: "fingerprint_persistence_failed",
        message: error.message,
    }
}

fn database_operation_error(error: rusqlite::Error) -> MetadataDatabaseError {
    MetadataDatabaseError::operation(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn test_database() -> (PathBuf, MetadataDatabase) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("illustrate-viewer-metadata-{unique}"));
        let database = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("create metadata database");
        (directory, database)
    }

    fn library(id: &str, path: &str) -> Library {
        Library {
            id: id.to_owned(),
            name: "Reference library".to_owned(),
            path: path.to_owned(),
            created_at: 1,
            updated_at: 2,
            image_count: None,
            cover: None,
        }
    }

    fn asset(id: &str, library_id: &str, path: &str) -> ImageAsset {
        ImageAsset {
            id: id.to_owned(),
            library_id: library_id.to_owned(),
            path: path.to_owned(),
            filename: "image.png".to_owned(),
            extension: "png".to_owned(),
            size: 42,
            created_at: Some(3),
            modified_at: 3,
        }
    }

    fn ready_image_metadata(image_id: &str) -> ImageMetadata {
        ImageMetadata {
            image_id: image_id.to_owned(),
            width: Some(1920),
            height: Some(1080),
            aspect_ratio: Some(16.0 / 9.0),
            file_size: Some(42),
            format: Some("image/png".to_owned()),
            has_alpha: Some(true),
            camera_make: Some("Illustrate Camera".to_owned()),
            camera_model: Some("Reference 1".to_owned()),
            captured_at: Some("2026-08-25T12:00:00Z".to_owned()),
            orientation: Some(1),
            extraction_version: 1,
            status: "ready".to_owned(),
            failure_reason: None,
            extracted_at: 42,
            source_created_at: Some(40),
            source_modified_at: 41,
        }
    }

    fn image_fingerprint(image_id: &str, content_hash: &str) -> ImageFingerprint {
        ImageFingerprint {
            image_id: image_id.to_owned(),
            content_hash: content_hash.to_owned(),
            perceptual_hash: "0123456789abcdef".to_owned(),
            algorithm_version: 1,
            generated_at: 42,
            source_modified_at: 41,
        }
    }

    fn search_ids(database: &MetadataDatabase, query: &str) -> Vec<String> {
        database
            .search_images(query)
            .expect("search images")
            .into_iter()
            .map(|asset| asset.id)
            .collect()
    }

    fn filter_ids(database: &MetadataDatabase, filters: ImageFilter) -> Vec<String> {
        database
            .filter_images(&filters)
            .expect("filter images")
            .into_iter()
            .map(|asset| asset.id)
            .collect()
    }

    fn query_ids(database: &MetadataDatabase, query: ImageSearchQuery) -> Vec<String> {
        database
            .query_image_assets(&query)
            .expect("query images")
            .into_iter()
            .map(|asset| asset.id)
            .collect()
    }

    fn table_exists(connection: &Connection, table: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get::<_, i64>(0),
            )
            .expect("read table existence")
            == 1
    }

    fn row_count(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("read table row count")
    }

    #[test]
    fn creates_and_reopens_the_current_schema() {
        let (directory, database) = test_database();
        assert_eq!(database.status().expect("initial status").schema_version, 9);
        drop(database);

        let connection =
            Connection::open(directory.join(DATABASE_FILE_NAME)).expect("open fresh database");
        for table in ["image_metadata", "image_fingerprints", "smart_collections"] {
            assert!(
                table_exists(&connection, table),
                "{table} exists in fresh schema"
            );
        }
        drop(connection);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert_eq!(
            reopened.status().expect("reopened status").schema_version,
            9
        );
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn library_path_persists_across_reopen() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        drop(database);

        let connection = Connection::open(directory.join(DATABASE_FILE_NAME)).expect("open sqlite");
        let path: String = connection
            .query_row(
                "SELECT path FROM libraries WHERE id = ?1",
                ["library-1"],
                |row| row.get(0),
            )
            .expect("read library path");
        assert_eq!(path, "C:\\Pictures\\Reference");
        drop(connection);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn image_assets_are_linked_to_their_library_and_pruned_after_scan() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        database
            .sync_images_for_library(
                "library-1",
                &[asset(
                    "image-1",
                    "library-1",
                    "C:\\Pictures\\Reference\\image.png",
                )],
            )
            .expect("sync image");
        assert_eq!(
            database.status().expect("asset status").image_asset_count,
            1
        );

        database
            .sync_images_for_library("library-1", &[])
            .expect("prune missing image");
        assert_eq!(
            database.status().expect("pruned status").image_asset_count,
            0
        );
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn favorites_persist_idempotently_and_follow_library_deletion() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        database
            .sync_images_for_library(
                "library-1",
                &[asset(
                    "image-1",
                    "library-1",
                    "C:\\Pictures\\Reference\\image.png",
                )],
            )
            .expect("sync image");

        database.add_favorite("image-1").expect("add favorite");
        database
            .add_favorite("image-1")
            .expect("add favorite twice");
        assert!(database.is_favorite("image-1").expect("check favorite"));
        assert_eq!(database.list_favorites().expect("list favorites").len(), 1);
        drop(database);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert!(reopened.is_favorite("image-1").expect("reopened favorite"));
        reopened
            .remove_library("library-1")
            .expect("remove library metadata");
        assert!(reopened
            .list_favorites()
            .expect("favorites after removal")
            .is_empty());
        assert!(!reopened.is_favorite("image-1").expect("removed favorite"));
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn rejects_favorites_for_unknown_images() {
        let (directory, database) = test_database();
        let error = database
            .add_favorite("missing-image")
            .expect_err("missing image should not be favorited");
        assert_eq!(error.code, "favorite_image_not_found");
        drop(database);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn migrates_v1_without_losing_libraries_or_favorites() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("illustrate-viewer-metadata-v1-{unique}"));
        fs::create_dir_all(&directory).expect("create test directory");
        let database_path = directory.join(DATABASE_FILE_NAME);
        let connection = Connection::open(&database_path).expect("create v1 database");
        connection
            .execute_batch(
                r#"
                PRAGMA foreign_keys = ON;
                CREATE TABLE libraries (
                    id TEXT PRIMARY KEY NOT NULL,
                    path TEXT NOT NULL COLLATE NOCASE UNIQUE,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE image_assets (
                    id TEXT PRIMARY KEY NOT NULL,
                    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    extension TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    modified_at INTEGER NOT NULL,
                    UNIQUE(library_id, path)
                );
                CREATE TABLE favorites (
                    id TEXT PRIMARY KEY NOT NULL,
                    image_id TEXT NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    created_at INTEGER NOT NULL,
                    UNIQUE(image_id)
                );
                CREATE TABLE collections (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE collection_items (
                    id TEXT PRIMARY KEY NOT NULL,
                    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
                    image_id TEXT NOT NULL REFERENCES image_assets(id) ON DELETE CASCADE,
                    UNIQUE(collection_id, image_id)
                );
                INSERT INTO libraries VALUES ('library-1', 'C:\\Pictures\\Reference', 'Reference', 1, 1);
                INSERT INTO image_assets VALUES ('image-1', 'library-1', 'C:\\Pictures\\Reference\\image.png', 'image.png', 'png', 1, 1);
                INSERT INTO favorites VALUES ('favorite-1', 'image-1', 1);
                INSERT INTO collections VALUES ('collection-1', 'Legacy collection', 1, 1);
                INSERT INTO collection_items VALUES ('item-1', 'collection-1', 'image-1');
                PRAGMA user_version = 1;
                "#,
            )
            .expect("create v1 schema");
        drop(connection);

        let database = MetadataDatabase::open_at(database_path).expect("migrate v1 database");
        assert_eq!(
            database.status().expect("migrated status").schema_version,
            9
        );
        assert!(database.is_favorite("image-1").expect("preserved favorite"));
        assert_eq!(
            database
                .list_collection_images("collection-1")
                .expect("legacy images")
                .len(),
            1
        );
        assert_eq!(
            database
                .create_tag("Legacy tag")
                .expect("create migrated tag")
                .name,
            "Legacy tag"
        );
        drop(database);

        let connection =
            Connection::open(directory.join(DATABASE_FILE_NAME)).expect("reopen migrated sqlite");
        let created_at: i64 = connection
            .query_row(
                "SELECT created_at FROM collection_items WHERE id = 'item-1'",
                [],
                |row| row.get(0),
            )
            .expect("read migrated item timestamp");
        assert_eq!(created_at, 0);
        drop(connection);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn migrates_v4_color_metadata_to_the_source_revision_cache() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("illustrate-viewer-metadata-v4-{unique}"));
        fs::create_dir_all(&directory).expect("create test directory");
        let database_path = directory.join(DATABASE_FILE_NAME);
        let connection = Connection::open(&database_path).expect("create v4 database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE image_color_metadata (
                    image_id TEXT PRIMARY KEY NOT NULL,
                    color_count INTEGER NOT NULL,
                    generated_at INTEGER NOT NULL
                );
                INSERT INTO image_color_metadata VALUES ('image-1', 1, 42);
                PRAGMA user_version = 4;
                "#,
            )
            .expect("create v4 color metadata");
        drop(connection);

        let database = MetadataDatabase::open_at(database_path).expect("migrate v4 database");
        drop(database);

        let connection =
            Connection::open(directory.join(DATABASE_FILE_NAME)).expect("open migrated database");
        let schema_version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read migrated schema version");
        assert_eq!(schema_version, 9);
        let source_modified_at: i64 = connection
            .query_row(
                "SELECT source_modified_at FROM image_color_metadata WHERE image_id = 'image-1'",
                [],
                |row| row.get(0),
            )
            .expect("read cache revision");
        assert_eq!(source_modified_at, -1);
        let image_metadata_table_exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'image_metadata')",
                [],
                |row| row.get(0),
            )
            .expect("read image metadata table");
        assert_eq!(image_metadata_table_exists, 1);
        let image_fingerprints_table_exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'image_fingerprints')",
                [],
                |row| row.get(0),
            )
            .expect("read image fingerprints table");
        assert_eq!(image_fingerprints_table_exists, 1);
        drop(connection);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn migrates_v7_to_add_smart_collections_without_losing_assets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("illustrate-viewer-metadata-v7-{unique}"));
        fs::create_dir_all(&directory).expect("create test directory");
        let database_path = directory.join(DATABASE_FILE_NAME);
        let connection = Connection::open(&database_path).expect("create v7 database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE libraries (id TEXT PRIMARY KEY, path TEXT NOT NULL, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE image_assets (id TEXT PRIMARY KEY, library_id TEXT NOT NULL, path TEXT NOT NULL, filename TEXT NOT NULL, extension TEXT NOT NULL, size INTEGER NOT NULL, modified_at INTEGER NOT NULL);
                CREATE TABLE favorites (id TEXT PRIMARY KEY, image_id TEXT NOT NULL, created_at INTEGER NOT NULL);
                CREATE TABLE image_metadata (image_id TEXT PRIMARY KEY, width INTEGER, height INTEGER, status TEXT NOT NULL);
                INSERT INTO libraries VALUES ('library-1', 'C:\\Pictures\\Reference', 'Reference', 1, 1);
                INSERT INTO image_assets VALUES ('image-1', 'library-1', 'C:\\Pictures\\Reference\\image.png', 'image.png', 'png', 42, 1);
                INSERT INTO favorites VALUES ('favorite-1', 'image-1', 1);
                PRAGMA user_version = 7;
                "#,
            )
            .expect("create v7 schema");
        drop(connection);

        let database = MetadataDatabase::open_at(database_path).expect("migrate v7 database");
        assert_eq!(
            database.status().expect("migrated status").schema_version,
            9
        );
        assert!(database.is_favorite("image-1").expect("preserved favorite"));
        let collection = database
            .create_smart_collection(
                "PNG images",
                SmartCollectionRule::Format {
                    extension: "png".to_owned(),
                },
            )
            .expect("create migrated smart collection");
        assert_eq!(
            database
                .list_smart_collection_images(&collection.id)
                .expect("list migrated assets")
                .len(),
            1
        );
        drop(database);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn migrates_v8_to_v9_without_losing_existing_data() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("illustrate-viewer-metadata-v8-{unique}"));
        fs::create_dir_all(&directory).expect("create test directory");
        let database_path = directory.join(DATABASE_FILE_NAME);
        let connection = Connection::open(&database_path).expect("create v8 database");
        connection
            .execute_batch(
                r#"
                PRAGMA foreign_keys = ON;
                CREATE TABLE libraries (id TEXT PRIMARY KEY, path TEXT NOT NULL, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE image_assets (id TEXT PRIMARY KEY, library_id TEXT NOT NULL REFERENCES libraries(id), path TEXT NOT NULL, filename TEXT NOT NULL, extension TEXT NOT NULL, size INTEGER NOT NULL, modified_at INTEGER NOT NULL);
                CREATE TABLE favorites (id TEXT PRIMARY KEY, image_id TEXT NOT NULL REFERENCES image_assets(id), created_at INTEGER NOT NULL, UNIQUE(image_id));
                CREATE TABLE collections (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE collection_items (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL REFERENCES collections(id), image_id TEXT NOT NULL REFERENCES image_assets(id), created_at INTEGER NOT NULL, UNIQUE(collection_id, image_id));
                CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL COLLATE NOCASE UNIQUE);
                CREATE TABLE image_tags (image_id TEXT NOT NULL REFERENCES image_assets(id), tag_id TEXT NOT NULL REFERENCES tags(id), PRIMARY KEY(image_id, tag_id));
                CREATE TABLE image_fingerprints (image_id TEXT PRIMARY KEY NOT NULL REFERENCES image_assets(id), content_hash TEXT NOT NULL, perceptual_hash TEXT NOT NULL, algorithm_version INTEGER NOT NULL, generated_at INTEGER NOT NULL, source_modified_at INTEGER NOT NULL);
                CREATE TABLE smart_collections (id TEXT PRIMARY KEY, name TEXT NOT NULL COLLATE NOCASE UNIQUE, rule_payload TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                INSERT INTO libraries VALUES ('library-1', 'C:\\Pictures\\Reference', 'Reference', 1, 1);
                INSERT INTO image_assets VALUES ('image-1', 'library-1', 'C:\\Pictures\\Reference\\image.png', 'image.png', 'png', 42, 1);
                INSERT INTO favorites VALUES ('favorite-1', 'image-1', 1);
                INSERT INTO collections VALUES ('collection-1', 'Legacy collection', 1, 1);
                INSERT INTO collection_items VALUES ('item-1', 'collection-1', 'image-1', 1);
                INSERT INTO tags VALUES ('tag-1', 'legacy');
                INSERT INTO image_tags VALUES ('image-1', 'tag-1');
                INSERT INTO image_fingerprints VALUES ('image-1', 'content-hash', 'perceptual-hash', 1, 1, 1);
                INSERT INTO smart_collections VALUES ('smart-1', 'PNG images', '{"kind":"format","extension":"png"}', 1, 1);
                PRAGMA user_version = 8;
                "#,
            )
            .expect("create v8 schema");
        drop(connection);

        let database =
            MetadataDatabase::open_at(database_path.clone()).expect("migrate v8 database");
        assert_eq!(
            database.status().expect("migrated status").schema_version,
            9
        );
        assert!(database.is_favorite("image-1").expect("preserved favorite"));
        assert_eq!(
            database
                .list_collection_images("collection-1")
                .expect("preserved collection")
                .len(),
            1
        );
        assert_eq!(database.list_tags().expect("preserved tags").len(), 1);
        assert!(database
            .get_image_fingerprint("image-1")
            .expect("preserved fingerprint")
            .is_some());
        assert_eq!(
            database
                .list_smart_collections()
                .expect("preserved smart collection")
                .len(),
            1
        );
        assert_eq!(
            database
                .get_image_metadata("image-1")
                .expect("query created metadata table"),
            None
        );
        drop(database);

        let connection = Connection::open(&database_path).expect("open migrated database");
        for table in ["image_metadata", "image_fingerprints", "smart_collections"] {
            assert!(
                table_exists(&connection, table),
                "{table} exists after v8 upgrade"
            );
        }
        for (table, expected) in [
            ("libraries", 1),
            ("image_assets", 1),
            ("favorites", 1),
            ("collections", 1),
            ("collection_items", 1),
            ("tags", 1),
            ("image_tags", 1),
            ("image_fingerprints", 1),
            ("smart_collections", 1),
        ] {
            assert_eq!(
                row_count(&connection, table),
                expected,
                "{table} rows are preserved"
            );
        }
        drop(connection);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn repairs_incomplete_v8_schema_idempotently_before_metadata_queries() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("illustrate-viewer-metadata-v8-repair-{unique}"));
        fs::create_dir_all(&directory).expect("create test directory");
        let database_path = directory.join(DATABASE_FILE_NAME);
        let connection = Connection::open(&database_path).expect("create incomplete v8 database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE libraries (id TEXT PRIMARY KEY, path TEXT NOT NULL, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE image_assets (id TEXT PRIMARY KEY, library_id TEXT NOT NULL, path TEXT NOT NULL, filename TEXT NOT NULL, extension TEXT NOT NULL, size INTEGER NOT NULL, modified_at INTEGER NOT NULL);
                CREATE TABLE favorites (id TEXT PRIMARY KEY, image_id TEXT NOT NULL, created_at INTEGER NOT NULL);
                CREATE TABLE collections (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE collection_items (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, image_id TEXT NOT NULL, created_at INTEGER NOT NULL);
                CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL);
                CREATE TABLE image_tags (image_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY(image_id, tag_id));
                CREATE TABLE image_fingerprints (image_id TEXT PRIMARY KEY, content_hash TEXT NOT NULL, perceptual_hash TEXT NOT NULL, algorithm_version INTEGER NOT NULL, generated_at INTEGER NOT NULL, source_modified_at INTEGER NOT NULL);
                INSERT INTO libraries VALUES ('library-1', 'C:\\Pictures', 'Pictures', 1, 1);
                INSERT INTO image_assets VALUES ('image-1', 'library-1', 'C:\\Pictures\\image.png', 'image.png', 'png', 1, 1);
                INSERT INTO favorites VALUES ('favorite-1', 'image-1', 1);
                INSERT INTO collections VALUES ('collection-1', 'Legacy collection', 1, 1);
                INSERT INTO tags VALUES ('tag-1', 'legacy');
                INSERT INTO image_tags VALUES ('image-1', 'tag-1');
                INSERT INTO image_fingerprints VALUES ('image-1', 'content-hash', 'perceptual-hash', 1, 1, 1);
                PRAGMA user_version = 8;
                "#,
            )
            .expect("create incomplete v8 schema");
        drop(connection);

        let database =
            MetadataDatabase::open_at(database_path.clone()).expect("repair incomplete v8 schema");
        assert_eq!(
            database.status().expect("repaired status").schema_version,
            9
        );
        assert_eq!(
            database
                .get_image_metadata("image-1")
                .expect("query repaired metadata"),
            None
        );
        drop(database);

        let reopened =
            MetadataDatabase::open_at(database_path.clone()).expect("repeat repair safely");
        assert!(reopened
            .is_favorite("image-1")
            .expect("favorite survives repeat repair"));
        assert_eq!(
            reopened
                .list_tags()
                .expect("tag survives repeat repair")
                .len(),
            1
        );
        drop(reopened);

        let connection = Connection::open(database_path).expect("open repaired v8 database");
        for table in ["image_metadata", "image_fingerprints", "smart_collections"] {
            assert!(
                table_exists(&connection, table),
                "{table} repaired from incomplete v8"
            );
        }
        assert_eq!(row_count(&connection, "libraries"), 1);
        assert_eq!(row_count(&connection, "image_assets"), 1);
        assert_eq!(row_count(&connection, "favorites"), 1);
        assert_eq!(row_count(&connection, "tags"), 1);
        assert_eq!(row_count(&connection, "image_tags"), 1);
        assert_eq!(row_count(&connection, "image_fingerprints"), 1);
        drop(connection);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn repairs_latest_schema_when_metadata_table_is_missing() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("illustrate-viewer-metadata-repair-{unique}"));
        fs::create_dir_all(&directory).expect("create test directory");
        let database_path = directory.join(DATABASE_FILE_NAME);
        let connection = Connection::open(&database_path).expect("create incomplete database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE libraries (
                    id TEXT PRIMARY KEY,
                    path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE image_assets (
                    id TEXT PRIMARY KEY,
                    library_id TEXT NOT NULL,
                    path TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    extension TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    modified_at INTEGER NOT NULL
                );
                INSERT INTO libraries VALUES ('library-1', 'C:\\Pictures', 'Pictures', 1, 1);
                INSERT INTO image_assets VALUES ('image-1', 'library-1', 'C:\\Pictures\\image.png', 'image.png', 'png', 1, 1);
                PRAGMA user_version = 9;
                "#,
            )
            .expect("create incomplete latest schema");
        drop(connection);

        let database =
            MetadataDatabase::open_at(database_path.clone()).expect("repair latest schema");
        assert_eq!(
            database
                .get_image_metadata("image-1")
                .expect("query repaired metadata table"),
            None
        );
        drop(database);

        let connection = Connection::open(database_path).expect("reopen repaired database");
        let table_exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'image_metadata')",
                [],
                |row| row.get(0),
            )
            .expect("read repaired table");
        assert_eq!(table_exists, 1);
        drop(connection);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn collections_persist_with_unique_memberships_and_keep_image_assets() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        database
            .sync_images_for_library(
                "library-1",
                &[
                    asset(
                        "image-1",
                        "library-1",
                        "C:\\Pictures\\Reference\\image-1.png",
                    ),
                    asset(
                        "image-2",
                        "library-1",
                        "C:\\Pictures\\Reference\\image-2.png",
                    ),
                ],
            )
            .expect("sync images");

        let collection = database
            .create_collection("  References  ")
            .expect("create collection");
        let second_collection = database
            .create_collection("More references")
            .expect("create second collection");
        let collection = database
            .rename_collection(&collection.id, "Renamed references")
            .expect("rename collection");
        database
            .add_image_to_collection(&collection.id, "image-1")
            .expect("add image");
        database
            .add_image_to_collection(&collection.id, "image-1")
            .expect("add image twice");
        database
            .add_image_to_collection(&second_collection.id, "image-1")
            .expect("add image to another collection");
        assert_eq!(collection.name, "Renamed references");
        assert_eq!(
            database
                .list_collection_images(&collection.id)
                .expect("list collection")
                .len(),
            1
        );
        assert_eq!(
            database
                .list_collection_images(&second_collection.id)
                .expect("list second collection")
                .len(),
            1
        );

        database
            .remove_image_from_collection(&collection.id, "image-1")
            .expect("remove image relation");
        assert!(database
            .list_collection_images(&collection.id)
            .expect("list removed relation")
            .is_empty());
        database
            .add_image_to_collection(&collection.id, "image-2")
            .expect("add replacement image");
        drop(database);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert_eq!(
            reopened.list_collections().expect("list collections").len(),
            2
        );
        assert_eq!(
            reopened
                .list_collection_images(&collection.id)
                .expect("reopened images")
                .len(),
            1
        );
        reopened
            .delete_collection(&collection.id)
            .expect("delete collection");
        assert_eq!(
            reopened.status().expect("images remain").image_asset_count,
            2
        );
        assert_eq!(
            reopened
                .list_collections()
                .expect("remaining collections")
                .len(),
            1
        );
        let connection = Connection::open(directory.join(DATABASE_FILE_NAME)).expect("open sqlite");
        let remaining_items: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM collection_items WHERE collection_id = ?1",
                [&collection.id],
                |row| row.get(0),
            )
            .expect("count removed collection items");
        assert_eq!(remaining_items, 0);
        drop(connection);
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn advanced_collection_assignments_are_atomic_idempotent_and_preserve_assets() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        let first_path = "C:\\Pictures\\Reference\\first.png";
        let second_path = "C:\\Pictures\\Reference\\second.png";
        database
            .sync_images_for_library(
                "library-1",
                &[
                    asset("image-1", "library-1", first_path),
                    asset("image-2", "library-1", second_path),
                ],
            )
            .expect("sync images");
        let first = database.create_collection("First").expect("create first");
        let second = database.create_collection("Second").expect("create second");

        database
            .add_image_to_collection(&first.id, "image-1")
            .expect("seed first relation");
        assert_eq!(
            database
                .list_collection_memberships(
                    &[first.id.clone(), second.id.clone()],
                    &["image-1".to_owned(), "image-2".to_owned()],
                )
                .expect("read membership states"),
            vec![
                CollectionMembership {
                    collection_id: first.id.clone(),
                    assigned_count: 1,
                },
                CollectionMembership {
                    collection_id: second.id.clone(),
                    assigned_count: 0,
                },
            ]
        );

        database
            .add_images_to_collections(
                &[first.id.clone(), second.id.clone(), second.id.clone()],
                &[
                    "image-1".to_owned(),
                    "image-2".to_owned(),
                    "image-2".to_owned(),
                ],
            )
            .expect("assign to both collections");
        database
            .add_images_to_collections(
                &[first.id.clone(), second.id.clone()],
                &["image-1".to_owned(), "image-2".to_owned()],
            )
            .expect("repeat assignment idempotently");
        for collection_id in [&first.id, &second.id] {
            let mut paths = database
                .list_collection_images(collection_id)
                .expect("list assigned images")
                .into_iter()
                .map(|image| image.path)
                .collect::<Vec<_>>();
            paths.sort();
            assert_eq!(paths, vec![first_path.to_owned(), second_path.to_owned()]);
        }

        let empty = database.create_collection("Empty").expect("create empty");
        assert_eq!(
            database
                .add_images_to_collections(
                    &[empty.id.clone()],
                    &["image-1".to_owned(), "missing-image".to_owned()],
                )
                .expect_err("missing image must abort the assignment")
                .code,
            "collection_image_not_found"
        );
        assert!(database
            .list_collection_images(&empty.id)
            .expect("read untouched collection")
            .is_empty());

        let direct = database
            .create_collection_with_images(
                "Direct",
                &[
                    "image-1".to_owned(),
                    "image-2".to_owned(),
                    "image-2".to_owned(),
                ],
            )
            .expect("create and assign atomically");
        let mut direct_paths = database
            .list_collection_images(&direct.id)
            .expect("read direct collection")
            .into_iter()
            .map(|image| image.path)
            .collect::<Vec<_>>();
        direct_paths.sort();
        assert_eq!(
            direct_paths,
            vec![first_path.to_owned(), second_path.to_owned()]
        );
        assert_eq!(
            database
                .create_collection_with_images(" direct ", &["image-1".to_owned()])
                .expect_err("duplicate name must fail")
                .code,
            "duplicate_collection_name"
        );
        let collection_count = database.list_collections().expect("list collections").len();
        assert_eq!(
            database
                .create_collection_with_images("Broken", &["missing-image".to_owned()])
                .expect_err("missing image must not create a collection")
                .code,
            "collection_image_not_found"
        );
        assert_eq!(
            database.list_collections().expect("list collections").len(),
            collection_count
        );
        assert_eq!(
            database
                .add_images_to_collections(&[], &["image-1".to_owned()])
                .expect_err("target selection is required")
                .code,
            "empty_collection_selection"
        );
        assert_eq!(
            database
                .list_collection_memberships(&[first.id.clone()], &[])
                .expect_err("image selection is required")
                .code,
            "empty_image_selection"
        );
        assert_eq!(
            database
                .create_collection_with_images("No images", &[])
                .expect_err("image selection is required")
                .code,
            "empty_image_selection"
        );
        drop(database);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn smart_collections_persist_rules_without_owning_images() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        let mut landscape = asset("image-1", "library-1", "C:\\Pictures\\Reference\\wide.png");
        landscape.filename = "wide.png".to_owned();
        let mut jpeg = asset("image-2", "library-1", "C:\\Pictures\\Reference\\photo.jpg");
        jpeg.filename = "photo.jpg".to_owned();
        jpeg.extension = "jpg".to_owned();
        let mut portrait = asset("image-3", "library-1", "C:\\Pictures\\Reference\\tall.png");
        portrait.filename = "tall.png".to_owned();
        database
            .sync_images_for_library("library-1", &[landscape, jpeg, portrait])
            .expect("sync images");
        database
            .replace_image_metadata(&ready_image_metadata("image-1"))
            .expect("store landscape metadata");
        let mut portrait_metadata = ready_image_metadata("image-3");
        portrait_metadata.width = Some(1080);
        portrait_metadata.height = Some(1920);
        portrait_metadata.aspect_ratio = Some(9.0 / 16.0);
        portrait_metadata.has_alpha = Some(false);
        database
            .replace_image_metadata(&portrait_metadata)
            .expect("store portrait metadata");
        database.add_favorite("image-2").expect("favorite image");

        let large = database
            .create_smart_collection(
                "Large images",
                SmartCollectionRule::LargeImages {
                    minimum_pixels: 2_000_000,
                },
            )
            .expect("create large rule");
        let jpeg_rule = database
            .create_smart_collection(
                "JPEG images",
                SmartCollectionRule::Format {
                    extension: "JPG".to_owned(),
                },
            )
            .expect("create format rule");
        let landscape_rule = database
            .create_smart_collection("Landscape", SmartCollectionRule::Landscape)
            .expect("create landscape rule");
        let width_rule = database
            .create_smart_collection(
                "Wide images",
                SmartCollectionRule::MinimumWidth {
                    minimum_width: 1920,
                },
            )
            .expect("create width rule");
        let height_rule = database
            .create_smart_collection(
                "Tall images",
                SmartCollectionRule::MinimumHeight {
                    minimum_height: 1920,
                },
            )
            .expect("create height rule");
        let portrait_rule = database
            .create_smart_collection("Portrait", SmartCollectionRule::Portrait)
            .expect("create portrait rule");
        let transparent_rule = database
            .create_smart_collection("Transparent PNG", SmartCollectionRule::TransparentPng)
            .expect("create transparency rule");
        let favorite_rule = database
            .create_smart_collection("Favorites", SmartCollectionRule::Favorite)
            .expect("create favorite rule");
        assert_eq!(
            database
                .list_smart_collection_images(&large.id)
                .expect("list large images")
                .into_iter()
                .map(|asset| asset.id)
                .collect::<Vec<_>>(),
            vec!["image-3", "image-1"]
        );
        assert_eq!(
            database
                .list_smart_collection_images(&jpeg_rule.id)
                .expect("list JPEG images")
                .into_iter()
                .map(|asset| asset.id)
                .collect::<Vec<_>>(),
            vec!["image-2"]
        );
        assert_eq!(
            database
                .list_smart_collection_images(&landscape_rule.id)
                .expect("list landscape images")
                .into_iter()
                .map(|asset| asset.id)
                .collect::<Vec<_>>(),
            vec!["image-1"]
        );
        for (rule, expected) in [
            (&width_rule, "image-1"),
            (&height_rule, "image-3"),
            (&portrait_rule, "image-3"),
            (&transparent_rule, "image-1"),
            (&favorite_rule, "image-2"),
        ] {
            assert_eq!(
                database
                    .list_smart_collection_images(&rule.id)
                    .expect("list dynamic smart collection")
                    .into_iter()
                    .map(|asset| asset.id)
                    .collect::<Vec<_>>(),
                vec![expected]
            );
        }
        assert_eq!(
            database
                .create_smart_collection(" large IMAGES ", SmartCollectionRule::Landscape)
                .expect_err("duplicate name should fail")
                .code,
            "duplicate_smart_collection_name"
        );
        assert_eq!(
            database
                .create_smart_collection(
                    "Invalid rule",
                    SmartCollectionRule::LargeImages { minimum_pixels: 0 },
                )
                .expect_err("empty size rule should fail")
                .code,
            "invalid_smart_collection_rule"
        );
        database
            .rename_smart_collection(&large.id, "Large reference images")
            .expect("rename smart collection");
        drop(database);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert_eq!(
            reopened
                .list_smart_collections()
                .expect("list saved smart collections")
                .len(),
            8
        );
        reopened
            .delete_smart_collection(&large.id)
            .expect("delete smart collection");
        assert_eq!(
            reopened
                .status()
                .expect("images stay indexed")
                .image_asset_count,
            3
        );
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn tags_persist_with_unique_relations_and_keep_image_assets() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        database
            .sync_images_for_library(
                "library-1",
                &[asset(
                    "image-1",
                    "library-1",
                    "C:\\Pictures\\Reference\\image.png",
                )],
            )
            .expect("sync image");

        let tag = database.create_tag("  Character  ").expect("create tag");
        assert_eq!(tag.name, "Character");
        assert_eq!(
            database
                .create_tag("  \t  ")
                .expect_err("empty tag should fail")
                .code,
            "invalid_tag_name"
        );
        assert_eq!(
            database
                .create_tag("character")
                .expect_err("duplicate tag should fail")
                .code,
            "duplicate_tag_name"
        );
        let chinese_tag = database.create_tag("角色").expect("create Chinese tag");
        let japanese_tag = database
            .create_tag("キャラクター")
            .expect("create Japanese tag");
        let long_name = "x".repeat(1024);
        let long_tag = database.create_tag(&long_name).expect("create long tag");
        let tag = database.rename_tag(&tag.id, "Hero").expect("rename tag");
        assert_eq!(
            database
                .add_tag_to_image(&tag.id, "missing")
                .expect_err("missing image should fail")
                .code,
            "tag_image_not_found"
        );
        database
            .add_tag_to_image(&tag.id, "image-1")
            .expect("add tag");
        database
            .add_tag_to_image(&tag.id, "image-1")
            .expect("add tag twice");
        assert_eq!(
            database
                .list_image_tags("image-1")
                .expect("list tags")
                .len(),
            1
        );
        let batch = database
            .list_image_tags_for_images(&["image-1".to_owned(), "missing".to_owned()])
            .expect("list tags in batch");
        assert_eq!(batch.get("image-1").map(Vec::len), Some(1));
        assert!(!batch.contains_key("missing"));
        assert_eq!(
            database
                .list_tag_images(&tag.id)
                .expect("list tag images")
                .len(),
            1
        );
        database.add_favorite("image-1").expect("add favorite");
        let collection = database
            .create_collection("Tagged references")
            .expect("create collection");
        database
            .add_image_to_collection(&collection.id, "image-1")
            .expect("add collection image");
        drop(database);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert_eq!(
            reopened
                .list_tags()
                .expect("list reopened tags")
                .into_iter()
                .find(|item| item.id == tag.id)
                .expect("renamed tag")
                .name,
            "Hero"
        );
        reopened
            .remove_tag_from_image(&tag.id, "image-1")
            .expect("remove tag relation");
        assert!(reopened
            .list_image_tags("image-1")
            .expect("list removed relation")
            .is_empty());
        reopened
            .add_tag_to_image(&tag.id, "image-1")
            .expect("add relation again");
        reopened.delete_tag(&tag.id).expect("delete tag");
        assert_eq!(
            reopened.status().expect("images remain").image_asset_count,
            1
        );
        assert!(reopened.is_favorite("image-1").expect("favorite remains"));
        assert_eq!(
            reopened
                .list_collection_images(&collection.id)
                .expect("collection remains")
                .len(),
            1
        );
        assert!(reopened
            .list_image_tags("image-1")
            .expect("tag relation cascaded")
            .is_empty());
        reopened
            .add_tag_to_image(&chinese_tag.id, "image-1")
            .expect("add relation before library deletion");
        reopened
            .remove_library("library-1")
            .expect("remove library");
        assert!(reopened
            .list_image_tags_for_images(&["image-1".to_owned()])
            .expect("relations cascade with image deletion")
            .is_empty());
        assert!(reopened
            .list_favorites()
            .expect("favorite cascades with image deletion")
            .is_empty());
        assert!(reopened
            .list_collection_images(&collection.id)
            .expect("collection membership cascades with image deletion")
            .is_empty());
        reopened
            .delete_tag(&chinese_tag.id)
            .expect("delete Chinese tag");
        reopened
            .delete_tag(&japanese_tag.id)
            .expect("delete Japanese tag");
        reopened.delete_tag(&long_tag.id).expect("delete long tag");
        assert!(reopened.list_tags().expect("tags deleted").is_empty());
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn color_metadata_is_independent_persistent_and_bounded() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        database
            .sync_images_for_library(
                "library-1",
                &[asset(
                    "image-1",
                    "library-1",
                    "C:\\Pictures\\Reference\\image.png",
                )],
            )
            .expect("sync image");
        assert!(database
            .get_image_color_metadata("image-1")
            .expect("read empty color metadata")
            .is_none());

        let metadata = ImageColorMetadata {
            image_id: "image-1".to_owned(),
            dominant_colors: vec![
                Color {
                    hex: "#D98A23".to_owned(),
                    rgb: "217,138,35".to_owned(),
                    hsl: "35,72%,49%".to_owned(),
                    percentage: 22.0,
                },
                Color {
                    hex: "#1D3557".to_owned(),
                    rgb: "29,53,87".to_owned(),
                    hsl: "216,50%,23%".to_owned(),
                    percentage: 18.0,
                },
            ],
            color_count: 2,
            generated_at: 42,
            source_modified_at: 41,
        };
        database
            .replace_image_color_metadata(&metadata)
            .expect("store color metadata");
        assert_eq!(
            database
                .get_image_color_metadata("image-1")
                .expect("read stored color metadata"),
            Some(metadata.clone())
        );

        let invalid = ImageColorMetadata {
            color_count: 1,
            ..metadata.clone()
        };
        assert_eq!(
            database
                .replace_image_color_metadata(&invalid)
                .expect_err("mismatched color count should fail")
                .code,
            "invalid_color_metadata"
        );
        drop(database);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert_eq!(
            reopened
                .get_image_color_metadata("image-1")
                .expect("read reopened color metadata"),
            Some(metadata)
        );
        reopened
            .sync_images_for_library("library-1", &[])
            .expect("remove image");
        let connection = Connection::open(directory.join(DATABASE_FILE_NAME)).expect("open sqlite");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM image_color_metadata", [], |row| {
                row.get(0)
            })
            .expect("count cascaded color metadata");
        assert_eq!(count, 0);
        drop(connection);
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn image_metadata_is_independent_persistent_validated_and_cascades() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        database
            .sync_images_for_library(
                "library-1",
                &[asset(
                    "image-1",
                    "library-1",
                    "C:\\Pictures\\Reference\\image.png",
                )],
            )
            .expect("sync image");
        assert!(database
            .get_image_metadata("image-1")
            .expect("read empty image metadata")
            .is_none());

        let metadata = ready_image_metadata("image-1");
        database
            .replace_image_metadata(&metadata)
            .expect("store image metadata");
        assert_eq!(
            database
                .get_image_metadata("image-1")
                .expect("read stored image metadata"),
            Some(metadata.clone())
        );

        let invalid = ImageMetadata {
            aspect_ratio: Some(2.0),
            ..metadata.clone()
        };
        assert_eq!(
            database
                .replace_image_metadata(&invalid)
                .expect_err("mismatched aspect ratio should fail")
                .code,
            "invalid_image_metadata"
        );
        let failed_without_reason = ImageMetadata {
            status: "failed".to_owned(),
            failure_reason: None,
            ..metadata.clone()
        };
        assert_eq!(
            database
                .replace_image_metadata(&failed_without_reason)
                .expect_err("failed metadata requires a reason")
                .code,
            "invalid_image_metadata"
        );
        drop(database);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert_eq!(
            reopened
                .get_image_metadata("image-1")
                .expect("read reopened image metadata"),
            Some(metadata)
        );
        reopened
            .sync_images_for_library("library-1", &[])
            .expect("remove image");
        assert_eq!(
            reopened
                .get_image_metadata("image-1")
                .expect_err("removed image should not resolve metadata")
                .code,
            "image_metadata_image_not_found"
        );
        let connection = Connection::open(directory.join(DATABASE_FILE_NAME)).expect("open sqlite");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM image_metadata", [], |row| row.get(0))
            .expect("count cascaded image metadata");
        assert_eq!(count, 0);
        drop(connection);
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn image_fingerprints_are_independent_persistent_and_cascade() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        database
            .sync_images_for_library(
                "library-1",
                &[
                    asset("image-1", "library-1", "C:\\Pictures\\Reference\\one.png"),
                    asset("image-2", "library-1", "C:\\Pictures\\Reference\\two.png"),
                ],
            )
            .expect("sync images");
        let first = image_fingerprint(
            "image-1",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let second = image_fingerprint(
            "image-2",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        database
            .replace_image_fingerprint(&first)
            .expect("store first fingerprint");
        database
            .replace_image_fingerprint(&second)
            .expect("store second fingerprint");
        assert_eq!(
            database
                .get_image_fingerprint("image-1")
                .expect("read stored fingerprint"),
            Some(first.clone())
        );
        assert_eq!(
            database
                .list_other_image_fingerprints("image-1")
                .expect("list other fingerprints"),
            vec![second.clone()]
        );
        assert_eq!(
            database
                .list_image_fingerprints()
                .expect("list cached fingerprints"),
            vec![first.clone(), second]
        );
        assert_eq!(
            database
                .find_image_assets_by_ids(&["image-2".to_owned()])
                .expect("load one candidate asset")
                .into_iter()
                .map(|asset| asset.id)
                .collect::<Vec<_>>(),
            vec!["image-2"]
        );
        let invalid = ImageFingerprint {
            content_hash: "not-a-sha256".to_owned(),
            ..first.clone()
        };
        assert_eq!(
            database
                .replace_image_fingerprint(&invalid)
                .expect_err("invalid content hash should fail")
                .code,
            "invalid_image_fingerprint"
        );
        drop(database);

        let reopened = MetadataDatabase::open_at(directory.join(DATABASE_FILE_NAME))
            .expect("reopen metadata database");
        assert_eq!(
            reopened
                .get_image_fingerprint("image-1")
                .expect("read reopened fingerprint"),
            Some(first)
        );
        reopened
            .sync_images_for_library(
                "library-1",
                &[asset(
                    "image-2",
                    "library-1",
                    "C:\\Pictures\\Reference\\two.png",
                )],
            )
            .expect("remove first image");
        let connection = Connection::open(directory.join(DATABASE_FILE_NAME)).expect("open sqlite");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM image_fingerprints", [], |row| {
                row.get(0)
            })
            .expect("count cascaded fingerprints");
        assert_eq!(count, 1);
        drop(connection);
        drop(reopened);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn searches_image_filename_library_collection_and_tag_metadata() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        let mut portrait = asset(
            "image-1",
            "library-1",
            "C:\\Pictures\\Reference\\portrait.png",
        );
        portrait.filename = "portrait.png".to_owned();
        let mut skyline = asset(
            "image-2",
            "library-1",
            "C:\\Pictures\\Reference\\skyline.png",
        );
        skyline.filename = "skyline.png".to_owned();
        database
            .sync_images_for_library("library-1", &[portrait, skyline])
            .expect("sync images");

        let collection = database
            .create_collection("Character studies")
            .expect("create collection");
        database
            .add_image_to_collection(&collection.id, "image-1")
            .expect("add collection image");
        let tag = database.create_tag("Night scene").expect("create tag");
        database
            .add_tag_to_image(&tag.id, "image-2")
            .expect("add tag image");

        assert_eq!(search_ids(&database, "portrait"), vec!["image-1"]);
        assert_eq!(
            search_ids(&database, "reference"),
            vec!["image-1", "image-2"]
        );
        assert_eq!(search_ids(&database, "character"), vec!["image-1"]);
        assert_eq!(search_ids(&database, "night"), vec!["image-2"]);
        assert!(database
            .search_images("%")
            .expect("escaped wildcard")
            .is_empty());
        assert!(database
            .search_images("  ")
            .expect("empty search")
            .is_empty());
        database.add_favorite("image-1").expect("add favorite");
        assert_eq!(
            filter_ids(
                &database,
                ImageFilter {
                    library_id: Some("library-1".to_owned()),
                    ..ImageFilter::default()
                }
            ),
            vec!["image-1", "image-2"],
        );
        assert_eq!(
            filter_ids(
                &database,
                ImageFilter {
                    collection_id: Some(collection.id.clone()),
                    ..ImageFilter::default()
                }
            ),
            vec!["image-1"],
        );
        assert_eq!(
            filter_ids(
                &database,
                ImageFilter {
                    tag_ids: vec![tag.id.clone(), tag.id.clone()],
                    ..ImageFilter::default()
                }
            ),
            vec!["image-2"],
        );
        assert_eq!(
            filter_ids(
                &database,
                ImageFilter {
                    favorites_only: true,
                    ..ImageFilter::default()
                }
            ),
            vec!["image-1"],
        );
        assert!(filter_ids(
            &database,
            ImageFilter {
                collection_id: Some(collection.id),
                tag_ids: vec![tag.id],
                ..ImageFilter::default()
            },
        )
        .is_empty());
        drop(database);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn queries_assets_by_filename_path_format_metadata_and_favorite_state() {
        let (directory, database) = test_database();
        database
            .sync_libraries(&[library("library-1", "C:\\Pictures\\Reference")])
            .expect("sync library");
        let mut portrait = asset(
            "image-1",
            "library-1",
            "C:\\Pictures\\Reference\\Characters\\portrait.png",
        );
        portrait.filename = "portrait.png".to_owned();
        portrait.extension = "png".to_owned();
        let mut landscape = asset(
            "image-2",
            "library-1",
            "C:\\Pictures\\Reference\\Scenes\\wide.jpg",
        );
        landscape.filename = "wide.jpg".to_owned();
        landscape.extension = "jpg".to_owned();
        let mut pending = asset(
            "image-3",
            "library-1",
            "C:\\Pictures\\Reference\\Characters\\sketch.webp",
        );
        pending.filename = "sketch.webp".to_owned();
        pending.extension = "webp".to_owned();
        database
            .sync_images_for_library("library-1", &[portrait, landscape, pending])
            .expect("sync images");

        let mut portrait_metadata = ready_image_metadata("image-1");
        portrait_metadata.width = Some(1200);
        portrait_metadata.height = Some(2400);
        portrait_metadata.aspect_ratio = Some(0.5);
        portrait_metadata.file_size = Some(512);
        portrait_metadata.has_alpha = Some(true);
        database
            .replace_image_metadata(&portrait_metadata)
            .expect("store portrait metadata");
        let mut landscape_metadata = ready_image_metadata("image-2");
        landscape_metadata.width = Some(4000);
        landscape_metadata.height = Some(2000);
        landscape_metadata.aspect_ratio = Some(2.0);
        landscape_metadata.file_size = Some(4096);
        landscape_metadata.has_alpha = Some(false);
        landscape_metadata.format = Some("image/jpeg".to_owned());
        database
            .replace_image_metadata(&landscape_metadata)
            .expect("store landscape metadata");
        database
            .add_favorite("image-2")
            .expect("favorite landscape");
        let collection = database
            .create_collection("Wide scenes")
            .expect("create collection");
        database
            .add_image_to_collection(&collection.id, "image-2")
            .expect("add landscape to collection");
        let tag = database.create_tag("Wide").expect("create tag");
        database
            .add_tag_to_image(&tag.id, "image-2")
            .expect("add landscape tag");
        let smart_collection = database
            .create_smart_collection("Landscape", SmartCollectionRule::Landscape)
            .expect("create smart collection");

        assert_eq!(
            query_ids(
                &database,
                ImageSearchQuery {
                    filename: Some("PORT".to_owned()),
                    ..ImageSearchQuery::default()
                },
            ),
            vec!["image-1"],
        );
        assert_eq!(
            query_ids(
                &database,
                ImageSearchQuery {
                    path: Some("\\Characters\\".to_owned()),
                    ..ImageSearchQuery::default()
                },
            ),
            vec!["image-1", "image-3"],
        );
        assert_eq!(
            query_ids(
                &database,
                ImageSearchQuery {
                    library_id: Some("library-1".to_owned()),
                    collection_id: Some(collection.id),
                    tag_ids: vec![tag.id],
                    favorites_only: true,
                    formats: vec!["jpeg".to_owned()],
                    metadata: ImageMetadataFilter {
                        minimum_width: Some(3840),
                        orientation: Some(ImageOrientation::Landscape),
                        ..ImageMetadataFilter::default()
                    },
                    smart_collection_id: Some(smart_collection.id),
                    ..ImageSearchQuery::default()
                },
            ),
            vec!["image-2"],
        );
        assert_eq!(
            query_ids(
                &database,
                ImageSearchQuery {
                    formats: vec!["JPEG".to_owned(), ".webp".to_owned()],
                    ..ImageSearchQuery::default()
                },
            ),
            vec!["image-3", "image-2"],
        );
        assert_eq!(
            query_ids(
                &database,
                ImageSearchQuery {
                    metadata: ImageMetadataFilter {
                        minimum_width: Some(3000),
                        minimum_file_size: Some(2048),
                        has_alpha: Some(false),
                        ..ImageMetadataFilter::default()
                    },
                    ..ImageSearchQuery::default()
                },
            ),
            vec!["image-2"],
        );
        assert_eq!(
            query_ids(
                &database,
                ImageSearchQuery {
                    favorite: Some(false),
                    ..ImageSearchQuery::default()
                },
            ),
            vec!["image-1", "image-3"],
        );
        let invalid = match database.query_image_assets(&ImageSearchQuery {
            metadata: ImageMetadataFilter {
                minimum_height: Some(2000),
                maximum_height: Some(1000),
                ..ImageMetadataFilter::default()
            },
            ..ImageSearchQuery::default()
        }) {
            Err(error) => error,
            Ok(_) => panic!("invalid metadata range should fail"),
        };
        assert_eq!(invalid.code, "invalid_search_query");
        drop(database);
        fs::remove_dir_all(directory).expect("remove test database");
    }

    #[test]
    fn rejects_invalid_collection_names_and_missing_ids() {
        let (directory, database) = test_database();
        let invalid_name_error = match database.create_collection("   ") {
            Err(error) => error,
            Ok(_) => panic!("empty name should fail"),
        };
        assert_eq!(invalid_name_error.code, "invalid_collection_name");

        let collection = database
            .create_collection("References")
            .expect("create collection");
        assert_eq!(
            database
                .add_image_to_collection(&collection.id, "missing-image")
                .expect_err("missing image should fail")
                .code,
            "collection_image_not_found"
        );
        let missing_collection_error = match database.list_collection_images("missing-collection") {
            Err(error) => error,
            Ok(_) => panic!("missing collection should fail"),
        };
        assert_eq!(missing_collection_error.code, "collection_not_found");
        drop(database);
        fs::remove_dir_all(directory).expect("remove test database");
    }
}
