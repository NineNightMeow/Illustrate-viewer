use std::{
    fs,
    io::ErrorKind,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{DynamicImage, ImageReader};
use serde::Serialize;
use tauri::AppHandle;

use crate::{
    database::{
        Color, ColorMetadataError, ImageColorMetadata, MetadataDatabase, MetadataDatabaseError,
    },
    thumbnail::{prepare_image_asset, ThumbnailCommandError, ThumbnailRequest},
};

const MAX_DOMINANT_COLORS: usize = 8;
const MAX_SAMPLED_PIXELS: usize = 250_000;
const COLOR_BIN_COUNT: usize = 4_096;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorAnalysisError {
    pub code: &'static str,
    pub message: String,
}

impl ColorAnalysisError {
    fn image_not_found() -> Self {
        Self {
            code: "color_analysis_image_not_found",
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
            code: "color_decode_failed",
            message: error.to_string(),
        }
    }

    fn metadata(error: MetadataDatabaseError) -> Self {
        Self {
            code: "color_metadata_persistence_failed",
            message: error.message,
        }
    }

    fn color_metadata(error: ColorMetadataError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }

    pub fn operation(message: impl Into<String>) -> Self {
        Self {
            code: "color_analysis_failed",
            message: message.into(),
        }
    }
}

pub fn analyze_image_colors(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
) -> Result<ImageColorMetadata, ColorAnalysisError> {
    let source_path = prepare_color_source(app, database, image_id)?;
    let source_modified_at = source_modified_at(Path::new(&source_path))?;
    if let Some(metadata) = usable_cached_metadata(
        database
            .get_image_color_metadata(image_id)
            .map_err(ColorAnalysisError::color_metadata)?,
        source_modified_at,
    ) {
        return Ok(metadata);
    }

    let image = decode_image(Path::new(&source_path))?;
    let metadata = extract_color_metadata(image, image_id, generated_at()?, source_modified_at)?;
    database
        .replace_image_color_metadata(&metadata)
        .map_err(ColorAnalysisError::color_metadata)?;
    Ok(metadata)
}

pub fn get_cached_image_color_metadata(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
) -> Result<Option<ImageColorMetadata>, ColorAnalysisError> {
    let source_path = prepare_color_source(app, database, image_id)?;
    let source_modified_at = source_modified_at(Path::new(&source_path))?;
    Ok(usable_cached_metadata(
        database
            .get_image_color_metadata(image_id)
            .map_err(ColorAnalysisError::color_metadata)?,
        source_modified_at,
    ))
}

pub fn sample_image_color(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
    x: u32,
    y: u32,
) -> Result<Color, ColorAnalysisError> {
    let source_path = prepare_color_source(app, database, image_id)?;
    sample_color(decode_image(Path::new(&source_path))?, x, y)
}

fn prepare_color_source(
    app: &AppHandle,
    database: &MetadataDatabase,
    image_id: &str,
) -> Result<String, ColorAnalysisError> {
    let asset = database
        .find_image_asset(image_id)
        .map_err(ColorAnalysisError::metadata)?
        .ok_or_else(ColorAnalysisError::image_not_found)?;
    prepare_image_asset(
        app,
        &ThumbnailRequest {
            image_id: asset.id,
            library_id: asset.library_id,
            source_path: asset.path,
        },
    )
    .map_err(ColorAnalysisError::source)
}

fn source_modified_at(path: &Path) -> Result<i64, ColorAnalysisError> {
    let modified = fs::metadata(path)
        .map_err(source_file_error)?
        .modified()
        .map_err(source_file_error)?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            ColorAnalysisError::operation("The source image modification time is invalid.")
        })?
        .as_millis();
    i64::try_from(modified).map_err(|_| {
        ColorAnalysisError::operation(
            "The source image modification time exceeds SQLite's INTEGER range.",
        )
    })
}

fn source_file_error(error: std::io::Error) -> ColorAnalysisError {
    match error.kind() {
        ErrorKind::NotFound => ColorAnalysisError {
            code: "source_missing",
            message: "The source image no longer exists.".into(),
        },
        ErrorKind::PermissionDenied => ColorAnalysisError {
            code: "source_unreadable",
            message: error.to_string(),
        },
        _ => ColorAnalysisError::decode(error),
    }
}

fn usable_cached_metadata(
    metadata: Option<ImageColorMetadata>,
    source_modified_at: i64,
) -> Option<ImageColorMetadata> {
    metadata.filter(|metadata| metadata.source_modified_at == source_modified_at)
}

fn decode_image(path: &Path) -> Result<DynamicImage, ColorAnalysisError> {
    let reader = ImageReader::open(path).map_err(source_file_error)?;
    reader
        .with_guessed_format()
        .map_err(ColorAnalysisError::decode)?
        .decode()
        .map_err(ColorAnalysisError::decode)
}

fn extract_color_metadata(
    image: DynamicImage,
    image_id: &str,
    generated_at: i64,
    source_modified_at: i64,
) -> Result<ImageColorMetadata, ColorAnalysisError> {
    let rgba = image.into_rgba8();
    let pixel_count = rgba.width() as usize * rgba.height() as usize;
    let stride = pixel_count.saturating_add(MAX_SAMPLED_PIXELS - 1) / MAX_SAMPLED_PIXELS;
    let stride = stride.max(1);
    let mut bins = [ColorBin::default(); COLOR_BIN_COUNT];
    let mut total_weight = 0_u64;

    for (index, pixel) in rgba.pixels().enumerate() {
        if index % stride != 0 || pixel[3] == 0 {
            continue;
        }
        let weight = u64::from(pixel[3]);
        let bin = &mut bins[color_bin_index(pixel[0], pixel[1], pixel[2])];
        bin.weight += weight;
        bin.red += u64::from(pixel[0]) * weight;
        bin.green += u64::from(pixel[1]) * weight;
        bin.blue += u64::from(pixel[2]) * weight;
        total_weight += weight;
    }

    let mut indices = (0..COLOR_BIN_COUNT)
        .filter(|index| bins[*index].weight > 0)
        .collect::<Vec<_>>();
    indices.sort_unstable_by(|left, right| {
        bins[*right]
            .weight
            .cmp(&bins[*left].weight)
            .then_with(|| left.cmp(right))
    });
    let dominant_colors = indices
        .into_iter()
        .take(MAX_DOMINANT_COLORS)
        .map(|index| color_from_bin(bins[index], total_weight))
        .collect::<Vec<_>>();
    let color_count = u8::try_from(dominant_colors.len())
        .map_err(|_| ColorAnalysisError::operation("Too many dominant colors were extracted."))?;

    Ok(ImageColorMetadata {
        image_id: image_id.to_owned(),
        dominant_colors,
        color_count,
        generated_at,
        source_modified_at,
    })
}

fn sample_color(image: DynamicImage, x: u32, y: u32) -> Result<Color, ColorAnalysisError> {
    let rgba = image.into_rgba8();
    if x >= rgba.width() || y >= rgba.height() {
        return Err(ColorAnalysisError::operation(
            "The selected pixel is outside the source image.",
        ));
    }

    let pixel = rgba.get_pixel(x, y);
    Ok(color_from_rgb(pixel[0], pixel[1], pixel[2], 0.0))
}

fn generated_at() -> Result<i64, ColorAnalysisError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ColorAnalysisError::operation("The system clock is invalid."))?
        .as_millis();
    i64::try_from(milliseconds).map_err(|_| {
        ColorAnalysisError::operation("The generated timestamp exceeds SQLite's INTEGER range.")
    })
}

#[derive(Clone, Copy, Default)]
struct ColorBin {
    weight: u64,
    red: u64,
    green: u64,
    blue: u64,
}

fn color_bin_index(red: u8, green: u8, blue: u8) -> usize {
    (usize::from(red >> 4) << 8) | (usize::from(green >> 4) << 4) | usize::from(blue >> 4)
}

fn color_from_bin(bin: ColorBin, total_weight: u64) -> Color {
    let red = (bin.red / bin.weight) as u8;
    let green = (bin.green / bin.weight) as u8;
    let blue = (bin.blue / bin.weight) as u8;
    color_from_rgb(
        red,
        green,
        blue,
        ((bin.weight as f64 / total_weight as f64) * 10_000.0).round() / 100.0,
    )
}

fn color_from_rgb(red: u8, green: u8, blue: u8, percentage: f64) -> Color {
    let (hue, saturation, lightness) = rgb_to_hsl(red, green, blue);
    Color {
        hex: format!("#{red:02X}{green:02X}{blue:02X}"),
        rgb: format!("{red},{green},{blue}"),
        hsl: format!("{hue},{saturation}%,{lightness}%"),
        percentage,
    }
}

fn rgb_to_hsl(red: u8, green: u8, blue: u8) -> (u16, u8, u8) {
    let red = f64::from(red) / 255.0;
    let green = f64::from(green) / 255.0;
    let blue = f64::from(blue) / 255.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let lightness = (maximum + minimum) / 2.0;
    let saturation = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * lightness - 1.0).abs())
    };
    let hue = if delta == 0.0 {
        0.0
    } else if maximum == red {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if maximum == green {
        60.0 * ((blue - red) / delta + 2.0)
    } else {
        60.0 * ((red - green) / delta + 4.0)
    };
    (
        hue.round() as u16 % 360,
        (saturation * 100.0).round() as u8,
        (lightness * 100.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use std::{env, fs, fs::File, path::Path};

    use image::{
        codecs::gif::GifEncoder, Delay, DynamicImage, Frame, GenericImageView, ImageFormat, Rgb,
        RgbImage, Rgba, RgbaImage,
    };
    use uuid::Uuid;

    use super::{decode_image, extract_color_metadata, sample_color, usable_cached_metadata};

    fn temporary_folder(name: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!("illustrate-viewer-color-{name}-{}", Uuid::new_v4()))
    }

    #[test]
    fn quantizes_alpha_aware_colors_and_limits_the_palette() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(10, 1, |index, _| {
            if index < 8 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 0])
            }
        }));
        let metadata = extract_color_metadata(image, "image-1", 1, 2).expect("extract colors");
        assert_eq!(metadata.color_count, 1);
        assert_eq!(metadata.dominant_colors[0].hex, "#FF0000");
        assert_eq!(metadata.dominant_colors[0].hsl, "0,100%,50%");
        assert_eq!(metadata.dominant_colors[0].percentage, 100.0);

        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(9, 1, |index, _| {
            Rgba([(index * 25) as u8, 0, 0, 255])
        }));
        let metadata = extract_color_metadata(image, "image-1", 1, 2).expect("extract palette");
        assert_eq!(metadata.color_count, 8);
    }

    #[test]
    fn decodes_png_jpeg_webp_and_the_first_gif_frame() {
        let root = temporary_folder("formats");
        fs::create_dir_all(&root).expect("create temporary directory");
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 2, Rgb([20, 40, 60])));
        for (filename, format) in [
            ("image.png", ImageFormat::Png),
            ("image.jpg", ImageFormat::Jpeg),
            ("image.webp", ImageFormat::WebP),
        ] {
            let path = root.join(filename);
            image
                .save_with_format(&path, format)
                .expect("write supported image");
            assert_eq!(
                decode_image(&path)
                    .expect("decode supported image")
                    .dimensions(),
                (2, 2)
            );
        }

        let gif = root.join("animated.gif");
        let mut encoder = GifEncoder::new(File::create(&gif).expect("create gif"));
        encoder
            .encode_frame(Frame::from_parts(
                RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])),
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            ))
            .expect("write first gif frame");
        encoder
            .encode_frame(Frame::from_parts(
                RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255])),
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            ))
            .expect("write second gif frame");
        let metadata = extract_color_metadata(
            decode_image(&gif).expect("decode first gif frame"),
            "image-1",
            1,
            2,
        )
        .expect("extract gif colors");
        assert_eq!(metadata.dominant_colors[0].hex, "#FF0000");

        fs::remove_dir_all(root).expect("remove temporary directory");
    }

    #[test]
    fn rejects_corrupt_files_without_panicking() {
        let root = temporary_folder("corrupt");
        fs::create_dir_all(&root).expect("create temporary directory");
        let corrupt = root.join("corrupt.png");
        fs::write(&corrupt, b"not an image").expect("write corrupt image");
        let error = decode_image(Path::new(&corrupt)).expect_err("reject corrupt image");
        assert_eq!(error.code, "color_decode_failed");
        fs::remove_dir_all(root).expect("remove temporary directory");
    }

    #[test]
    fn samples_the_original_pixel_without_using_the_rendered_image() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 255, 0, 128])
            }
        }));

        let color = sample_color(image, 1, 0).expect("sample source pixel");
        assert_eq!(color.hex, "#00FF00");
        assert_eq!(color.rgb, "0,255,0");
        assert_eq!(color.hsl, "120,100%,50%");
        assert_eq!(color.percentage, 0.0);
    }

    #[test]
    fn reuses_only_metadata_from_the_current_source_revision() {
        let metadata = extract_color_metadata(
            DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb([1, 2, 3]))),
            "image-1",
            1,
            2,
        )
        .expect("extract metadata");

        assert!(usable_cached_metadata(Some(metadata.clone()), 2).is_some());
        assert!(usable_cached_metadata(Some(metadata), 3).is_none());
    }
}
