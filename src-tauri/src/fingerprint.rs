use std::{
    collections::HashMap,
    fs,
    io::{BufReader, ErrorKind, Read},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{imageops::FilterType, DynamicImage, ImageReader};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::AppHandle;

use crate::{
    database::{
        FingerprintMetadataError, ImageFingerprint, MetadataDatabase, MetadataDatabaseError,
    },
    scanner::ImageAsset,
    thumbnail::{prepare_image_asset, ThumbnailCommandError, ThumbnailRequest},
};

pub const PERCEPTUAL_HASH_ALGORITHM_VERSION: u32 = 1;
const PERCEPTUAL_HASH_SIDE: u32 = 7;
const MAX_PERCEPTUAL_DISTANCE: u8 = 6;
const MAX_INDICATOR_BATCH_SIZE: usize = 128;
const MAX_CANDIDATE_PAGE_SIZE: usize = 24;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateIndicator {
    pub image_id: String,
    pub match_kind: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCandidate {
    pub asset: ImageAsset,
    pub match_kind: String,
    pub perceptual_distance: u8,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCandidatePage {
    pub exact_count: u32,
    pub similar_count: u32,
    pub total_count: u32,
    pub offset: u32,
    pub candidates: Vec<DuplicateCandidate>,
}

#[derive(Clone, Debug, PartialEq)]
struct CandidateMatch {
    image_id: String,
    match_kind: &'static str,
    perceptual_distance: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintError {
    pub code: &'static str,
    pub message: String,
}

impl FingerprintError {
    fn image_not_found() -> Self {
        Self {
            code: "fingerprint_image_not_found",
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
            code: "fingerprint_decode_failed",
            message: error.to_string(),
        }
    }

    fn metadata(error: MetadataDatabaseError) -> Self {
        Self {
            code: "fingerprint_persistence_failed",
            message: error.message,
        }
    }

    fn fingerprint_metadata(error: FingerprintMetadataError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }

    pub fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "fingerprint_failed",
            message: message.into(),
        }
    }
}

pub fn analyze_image_fingerprint(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
) -> Result<ImageFingerprint, FingerprintError> {
    let source_path = prepare_fingerprint_source(app, database, image_id)?;
    let source_modified_at = source_modified_at(Path::new(&source_path))?;
    if let Some(fingerprint) = usable_cached_fingerprint(
        database
            .get_image_fingerprint(image_id)
            .map_err(FingerprintError::fingerprint_metadata)?,
        source_modified_at,
    ) {
        return Ok(fingerprint);
    }

    let fingerprint = ImageFingerprint {
        image_id: image_id.to_owned(),
        content_hash: sha256_file(Path::new(&source_path))?,
        perceptual_hash: perceptual_hash(decode_image(Path::new(&source_path))?),
        algorithm_version: PERCEPTUAL_HASH_ALGORITHM_VERSION,
        generated_at: generated_at()?,
        source_modified_at,
    };
    database
        .replace_image_fingerprint(&fingerprint)
        .map_err(FingerprintError::fingerprint_metadata)?;
    Ok(fingerprint)
}

pub fn get_cached_image_fingerprint(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
) -> Result<Option<ImageFingerprint>, FingerprintError> {
    let source_path = prepare_fingerprint_source(app, database, image_id)?;
    let source_modified_at = source_modified_at(Path::new(&source_path))?;
    Ok(usable_cached_fingerprint(
        database
            .get_image_fingerprint(image_id)
            .map_err(FingerprintError::fingerprint_metadata)?,
        source_modified_at,
    ))
}

pub fn list_duplicate_candidates(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
    offset: u32,
    limit: u16,
) -> Result<DuplicateCandidatePage, FingerprintError> {
    let source = analyze_image_fingerprint(app, database, image_id)?;
    // ponytail: O(n) cached SQLite rows; add perceptual-hash buckets only if large-library profiling requires it.
    let candidates = database
        .list_other_image_fingerprints(image_id)
        .map_err(FingerprintError::fingerprint_metadata)?;
    duplicate_candidate_page(database, &source, candidates, offset, limit)
}

pub fn get_cached_duplicate_indicators(
    database: &MetadataDatabase,
    image_ids: &[String],
) -> Result<Vec<DuplicateIndicator>, FingerprintError> {
    if image_ids.len() > MAX_INDICATOR_BATCH_SIZE {
        return Err(FingerprintError::operation(
            "Duplicate indicators must be requested in a bounded batch.",
        ));
    }

    let fingerprints = database
        .list_image_fingerprints()
        .map_err(FingerprintError::fingerprint_metadata)?;
    let mut indicators = Vec::new();
    for image_id in image_ids {
        let Some(source) = fingerprints
            .iter()
            .find(|fingerprint| fingerprint.image_id == *image_id)
        else {
            continue;
        };
        let Some(match_kind) = duplicate_indicator(source, &fingerprints) else {
            continue;
        };
        indicators.push(DuplicateIndicator {
            image_id: image_id.clone(),
            match_kind: match_kind.to_owned(),
        });
    }
    Ok(indicators)
}

fn prepare_fingerprint_source(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
) -> Result<String, FingerprintError> {
    let asset = database
        .find_image_asset(image_id)
        .map_err(FingerprintError::metadata)?
        .ok_or_else(FingerprintError::image_not_found)?;
    prepare_image_asset(
        app,
        &ThumbnailRequest {
            image_id: asset.id,
            library_id: asset.library_id,
            source_path: asset.path,
        },
    )
    .map_err(FingerprintError::source)
}

fn source_modified_at(path: &Path) -> Result<i64, FingerprintError> {
    let modified = fs::metadata(path)
        .map_err(source_file_error)?
        .modified()
        .map_err(source_file_error)?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FingerprintError::operation("The source image modification time is invalid."))?
        .as_millis();
    i64::try_from(modified).map_err(|_| {
        FingerprintError::operation(
            "The source image modification time exceeds SQLite's INTEGER range.",
        )
    })
}

fn source_file_error(error: std::io::Error) -> FingerprintError {
    match error.kind() {
        ErrorKind::NotFound => FingerprintError {
            code: "source_missing",
            message: "The source image no longer exists.".into(),
        },
        ErrorKind::PermissionDenied => FingerprintError {
            code: "source_unreadable",
            message: error.to_string(),
        },
        _ => FingerprintError::decode(error),
    }
}

fn usable_cached_fingerprint(
    fingerprint: Option<ImageFingerprint>,
    source_modified_at: i64,
) -> Option<ImageFingerprint> {
    fingerprint.filter(|fingerprint| {
        fingerprint.source_modified_at == source_modified_at
            && fingerprint.algorithm_version == PERCEPTUAL_HASH_ALGORITHM_VERSION
    })
}

fn sha256_file(path: &Path) -> Result<String, FingerprintError> {
    let file = fs::File::open(path).map_err(source_file_error)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let byte_count = reader.read(&mut buffer).map_err(source_file_error)?;
        if byte_count == 0 {
            break;
        }
        hasher.update(&buffer[..byte_count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn decode_image(path: &Path) -> Result<DynamicImage, FingerprintError> {
    ImageReader::open(path)
        .map_err(source_file_error)?
        .with_guessed_format()
        .map_err(FingerprintError::decode)?
        .decode()
        .map_err(FingerprintError::decode)
}

fn perceptual_hash(image: DynamicImage) -> String {
    let image = image
        .resize_exact(
            PERCEPTUAL_HASH_SIDE,
            PERCEPTUAL_HASH_SIDE,
            FilterType::Triangle,
        )
        .into_rgba8();
    let luminance = image
        .pixels()
        .map(|pixel| {
            let luma = (299_u32 * u32::from(pixel[0])
                + 587_u32 * u32::from(pixel[1])
                + 114_u32 * u32::from(pixel[2]))
                / 1_000;
            (luma * u32::from(pixel[3]) / 255, u32::from(pixel[3]))
        })
        .collect::<Vec<_>>();
    let average_luminance =
        luminance.iter().map(|(luma, _)| luma).sum::<u32>() / luminance.len() as u32;
    let average_alpha =
        luminance.iter().map(|(_, alpha)| alpha).sum::<u32>() / luminance.len() as u32;
    let pattern = luminance.into_iter().fold(0_u64, |hash, (luma, _)| {
        (hash << 1) | u64::from(luma >= average_luminance)
    });
    let value =
        (pattern << 15) | (u64::from(average_luminance >> 1) << 8) | u64::from(average_alpha);
    format!("{value:016x}")
}

fn duplicate_candidate_page(
    database: &MetadataDatabase,
    source: &ImageFingerprint,
    fingerprints: Vec<ImageFingerprint>,
    offset: u32,
    limit: u16,
) -> Result<DuplicateCandidatePage, FingerprintError> {
    let matches = duplicate_candidate_matches(source, fingerprints);
    let exact_count = matches
        .iter()
        .filter(|candidate| candidate.match_kind == "exact")
        .count();
    let similar_count = matches.len() - exact_count;
    let total_count = u32::try_from(matches.len())
        .map_err(|_| FingerprintError::operation("Too many duplicate candidates to display."))?;
    let offset = usize::try_from(offset)
        .map_err(|_| FingerprintError::operation("The candidate offset is invalid."))?;
    let limit = usize::from(limit).min(MAX_CANDIDATE_PAGE_SIZE);
    let page_matches = matches
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let asset_ids = page_matches
        .iter()
        .map(|candidate| candidate.image_id.clone())
        .collect::<Vec<_>>();
    let assets = database
        .find_image_assets_by_ids(&asset_ids)
        .map_err(FingerprintError::metadata)?
        .into_iter()
        .map(|asset| (asset.id.clone(), asset))
        .collect::<HashMap<_, _>>();
    let candidates = page_matches
        .into_iter()
        .filter_map(|candidate| {
            assets
                .get(&candidate.image_id)
                .cloned()
                .map(|asset| DuplicateCandidate {
                    asset,
                    match_kind: candidate.match_kind.to_owned(),
                    perceptual_distance: candidate.perceptual_distance,
                })
        })
        .collect();

    Ok(DuplicateCandidatePage {
        exact_count: u32::try_from(exact_count)
            .map_err(|_| FingerprintError::operation("Too many exact duplicate candidates."))?,
        similar_count: u32::try_from(similar_count)
            .map_err(|_| FingerprintError::operation("Too many similar duplicate candidates."))?,
        total_count,
        offset: u32::try_from(offset)
            .map_err(|_| FingerprintError::operation("The candidate offset is invalid."))?,
        candidates,
    })
}

fn duplicate_candidate_matches(
    source: &ImageFingerprint,
    fingerprints: Vec<ImageFingerprint>,
) -> Vec<CandidateMatch> {
    let mut candidates = fingerprints
        .into_iter()
        .filter_map(|candidate| {
            if candidate.content_hash == source.content_hash {
                return Some(CandidateMatch {
                    image_id: candidate.image_id,
                    match_kind: "exact",
                    perceptual_distance: 0,
                });
            }
            if candidate.algorithm_version != source.algorithm_version {
                return None;
            }
            let distance =
                perceptual_distance(&source.perceptual_hash, &candidate.perceptual_hash)?;
            (distance <= MAX_PERCEPTUAL_DISTANCE).then_some(CandidateMatch {
                image_id: candidate.image_id,
                match_kind: "similar",
                perceptual_distance: distance,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable_by(|left, right| {
        (
            left.match_kind,
            left.perceptual_distance,
            left.image_id.as_str(),
        )
            .cmp(&(
                right.match_kind,
                right.perceptual_distance,
                right.image_id.as_str(),
            ))
    });
    candidates
}

fn duplicate_indicator(
    source: &ImageFingerprint,
    fingerprints: &[ImageFingerprint],
) -> Option<&'static str> {
    let mut similar_found = false;
    for candidate in fingerprints {
        if candidate.image_id == source.image_id {
            continue;
        }
        if candidate.content_hash == source.content_hash {
            return Some("exact");
        }
        if candidate.algorithm_version == source.algorithm_version
            && perceptual_distance(&source.perceptual_hash, &candidate.perceptual_hash)
                .is_some_and(|distance| distance <= MAX_PERCEPTUAL_DISTANCE)
        {
            similar_found = true;
        }
    }
    similar_found.then_some("similar")
}

fn perceptual_distance(left: &str, right: &str) -> Option<u8> {
    let left = u64::from_str_radix(left, 16).ok()?;
    let right = u64::from_str_radix(right, 16).ok()?;
    Some((left ^ right).count_ones() as u8)
}

fn generated_at() -> Result<i64, FingerprintError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FingerprintError::operation("The system clock is invalid."))?
        .as_millis();
    i64::try_from(milliseconds).map_err(|_| {
        FingerprintError::operation("The generated timestamp exceeds SQLite's INTEGER range.")
    })
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use image::{DynamicImage, Rgba, RgbaImage};
    use uuid::Uuid;

    use super::{
        duplicate_candidate_matches, duplicate_indicator, perceptual_hash, sha256_file,
        usable_cached_fingerprint, CandidateMatch, ImageFingerprint,
        PERCEPTUAL_HASH_ALGORITHM_VERSION,
    };

    fn fingerprint(image_id: &str, content_hash: &str, perceptual_hash: &str) -> ImageFingerprint {
        ImageFingerprint {
            image_id: image_id.to_owned(),
            content_hash: content_hash.to_owned(),
            perceptual_hash: perceptual_hash.to_owned(),
            algorithm_version: PERCEPTUAL_HASH_ALGORITHM_VERSION,
            generated_at: 1,
            source_modified_at: 2,
        }
    }

    #[test]
    fn uses_sha256_for_exact_file_identity() {
        let path = env::temp_dir().join(format!("illustrate-viewer-hash-{}", Uuid::new_v4()));
        fs::write(&path, b"abc").expect("write source file");
        assert_eq!(
            sha256_file(&path).expect("hash source file"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        fs::remove_file(path).expect("remove source file");
    }

    #[test]
    fn uses_a_versioned_alpha_aware_perceptual_hash() {
        let opaque = DynamicImage::ImageRgba8(RgbaImage::from_fn(8, 8, |x, _| {
            Rgba([255, 0, 0, if x < 4 { 255 } else { 96 }])
        }));
        let transparent = DynamicImage::ImageRgba8(RgbaImage::from_fn(8, 8, |x, _| {
            Rgba([255, 0, 0, if x < 4 { 255 } else { 0 }])
        }));
        assert_ne!(perceptual_hash(opaque), perceptual_hash(transparent));
    }

    #[test]
    fn returns_exact_then_visual_duplicate_candidates_without_mutating_files() {
        let source = fingerprint("image-1", "a", "0000000000000000");
        let candidates = duplicate_candidate_matches(
            &source,
            vec![
                fingerprint("image-2", "a", "ffffffffffffffff"),
                fingerprint("image-3", "b", "0000000000000003"),
                fingerprint("image-4", "c", "ffffffffffffffff"),
            ],
        );
        assert_eq!(
            candidates,
            vec![
                CandidateMatch {
                    image_id: "image-2".to_owned(),
                    match_kind: "exact",
                    perceptual_distance: 0,
                },
                CandidateMatch {
                    image_id: "image-3".to_owned(),
                    match_kind: "similar",
                    perceptual_distance: 2,
                },
            ]
        );
    }

    #[test]
    fn cached_indicator_prefers_an_exact_match_over_a_similar_candidate() {
        let source = fingerprint("image-1", "a", "0000000000000000");
        assert_eq!(
            duplicate_indicator(
                &source,
                &[
                    source.clone(),
                    fingerprint("image-2", "b", "0000000000000003"),
                    fingerprint("image-3", "a", "ffffffffffffffff"),
                ],
            ),
            Some("exact")
        );
    }

    #[test]
    fn reuses_only_current_algorithm_and_source_revision() {
        let fingerprint = fingerprint("image-1", "a", "0000000000000000");
        assert!(usable_cached_fingerprint(Some(fingerprint.clone()), 2).is_some());
        assert!(usable_cached_fingerprint(Some(fingerprint.clone()), 3).is_none());
        assert!(usable_cached_fingerprint(
            Some(ImageFingerprint {
                algorithm_version: PERCEPTUAL_HASH_ALGORITHM_VERSION + 1,
                ..fingerprint
            }),
            2,
        )
        .is_none());
    }
}
