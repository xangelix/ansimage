//! `ansimage` is a Rust crate for converting images into colorful ASCII and Unicode art
//! for terminal display. It offers a high degree of customization over character sets,
//! color palettes, dithering, and output size.
//!
//! # Example
//!
//! ```no_run
//! use ansimage::{Settings, convert};
//! use std::path::Path;
//!
//! fn main() -> ansimage::error::Result<()> {
//!     let settings = Settings::default();
//!     let path = Path::new("path/to/image.png");
//!     let output = convert(path, &settings)?;
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```
//!
//! The core functionality revolves around the [`convert`] function, which takes an image
//! path and a [`Settings`] configuration object to produce the final styled string.

// Crate-level configuration
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]

pub mod error;
pub mod palettes;
pub mod processing;
pub mod sets;
pub mod settings;

use std::path::Path;

use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, ResizeOptions, Resizer};
use image::{DynamicImage, GenericImageView};
use imagequant::{
    Attributes as LiqAttr, Image as LiqImage, QuantizationResult as LiqResult, RGBA as LiqRGBA,
};
use rayon::iter::{
    IndexedParallelIterator as _, IntoParallelRefMutIterator as _, ParallelIterator as _,
};

// Re-export key types for consumers of the library.
pub use self::settings::{
    Advanced, AsciiCharSet, CharacterMode, Characters, ColorMode, Colors, DitherMatrix, Dithering,
    Settings, Size, SizeMode, UnicodeCharSet,
};

/// The black color constant in the L*u*v* color space, used for brightness calculations.
pub(crate) const BLACK_LUV: processing::LuvColor = palette::Luv::new(0.0, 0.0, 0.0);

/// Converts an image file into a styled terminal string based on the provided settings.
///
/// This is the main entry point for the library. It handles image loading, decoding,
/// resizing, optional dithering, and the core conversion logic.
///
/// # Arguments
///
/// * `path` - A reference to the path of the image file to convert.
/// * `settings` - A reference to the `Settings` struct that configures the conversion process.
///
/// # Returns
///
/// A `Result` containing the ANSI-styled terminal string on success, or an [`error::AnsiImageError`]
/// on failure.
///
/// # Errors
///
/// This function can fail if:
/// * The image file cannot be opened or read.
/// * The image format is unsupported or the data is corrupt.
/// * The provided settings are invalid (e.g., an empty custom character set or color palette).
pub fn convert(path: &Path, settings: &Settings) -> error::Result<String> {
    let img = image::open(path)?;
    convert_image(&img, settings)
}

/// Converts a pre-loaded [`DynamicImage`] into a styled terminal string.
///
/// This function is an alternative to [`convert`] for cases where the image is
/// already in memory.
///
/// # Arguments
///
/// * `img` - A reference to the `DynamicImage` to convert.
/// * `settings` - A reference to the `Settings` struct that configures the conversion process.
///
/// # Returns
///
/// A `Result` containing the ANSI-styled terminal string on success, or an [`error::AnsiImageError`]
/// on failure.
///
/// # Errors
///
/// This function can fail if the provided settings are invalid.
pub fn convert_image(img: &DynamicImage, settings: &Settings) -> error::Result<String> {
    // 1. Validate settings before performing any expensive operations.
    if !settings.colors.is_truecolor && settings.colors.palette.is_empty() {
        return Err(error::AnsiImageError::InvalidSettings(
            "A color palette must be selected when not in truecolor mode.".into(),
        ));
    }
    if let CharacterMode::Custom(chars) = &settings.characters.mode
        && chars.is_empty()
    {
        return Err(error::AnsiImageError::InvalidSettings(
            "Custom character mode requires at least one character.".into(),
        ));
    }

    // 2. Calculate final output dimensions in characters (width, height).
    // The image is resized to 2x this size to sample 2x2 pixel blocks for each character.
    let (img_w, img_h) = img.dimensions();
    let (w, h) = calculate_dimensions(
        img_w,
        img_h,
        settings.size.width,
        settings.size.height,
        settings.size.mode,
        settings.characters.aspect_ratio,
    );
    let target_w = (w * 2) as u32;
    let target_h = (h * 2) as u32;

    // 3. Resize the image using a high-performance resizer.
    let src_image = Image::from_vec_u8(img_w, img_h, img.to_rgb8().into_raw(), PixelType::U8x3)
        .map_err(|e| error::AnsiImageError::Processing(e.to_string()))?;

    let mut dst_image = Image::new(target_w, target_h, src_image.pixel_type());

    let algorithm = fast_image_resize::ResizeAlg::Convolution(settings.advanced.resize_filter);
    let resize_options = ResizeOptions::new().resize_alg(algorithm);

    let mut resizer = Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, Some(&resize_options))
        .map_err(|e| error::AnsiImageError::Processing(e.to_string()))?;

    let resized_buffer = image::RgbImage::from_raw(target_w, target_h, dst_image.into_vec())
        .ok_or_else(|| {
            error::AnsiImageError::Processing("Failed to create image from resized buffer.".into())
        })?;

    // 4. Optionally apply color quantization and dithering if not in truecolor mode.
    let processed_img = if settings.colors.is_truecolor {
        resized_buffer
    } else {
        quantize_with_imagequant(
            &resized_buffer,
            &settings.colors.palette,
            settings.advanced.dithering.is_enabled,
        )?
    };

    // 5. Process the image pixels into styled characters in parallel.
    let mut rows: Vec<String> = vec![String::new(); h];
    rows.par_iter_mut().enumerate().for_each(|(y, row_buf)| {
        *row_buf = processing::process_row(y, w, &processed_img, settings);
    });

    Ok(rows.join("\n"))
}

/// Calculates the target dimensions in characters based on size settings.
///
/// This internal helper computes the final character grid size, respecting
/// the original image's aspect ratio when `SizeMode` is `Fit`.
fn calculate_dimensions(
    img_w: u32,
    img_h: u32,
    width: usize,
    height: usize,
    mode: SizeMode,
    char_ratio: f32,
) -> (usize, usize) {
    if mode == SizeMode::Exact {
        return (width, height);
    }

    let img_w_f = img_w as f32;
    let img_h_f = img_h as f32;
    let width_f = width as f32;
    let height_f = height as f32;

    let fit_height = width_f * (img_h_f / img_w_f) * char_ratio;
    let fit_width = (height_f * (img_w_f / img_h_f)) / char_ratio;

    if fit_height > height_f {
        (fit_width.round() as usize, height)
    } else {
        (width, fit_height.round() as usize)
    }
}

/// Reduces the image's color count to a fixed palette using `imagequant`.
///
/// This function also applies dithering if enabled. It's a necessary step
/// for non-truecolor terminals to approximate the original image's colors.
///
/// # Errors
///
/// Returns a `Processing` error if any step in the `imagequant` pipeline fails.
fn quantize_with_imagequant(
    rgb: &image::RgbImage,
    palette_rgb: &[image::Rgb<u8>],
    dithering_enabled: bool,
) -> error::Result<image::RgbImage> {
    let (w, h) = rgb.dimensions();

    // `imagequant` requires an RGBA buffer, so we convert the input.
    let rgba_pixels: Vec<LiqRGBA> = rgb
        .pixels()
        .map(|p| LiqRGBA {
            r: p[0],
            g: p[1],
            b: p[2],
            a: 255,
        })
        .collect();

    // The fixed palette must also be in RGBA format.
    let fixed_palette: Vec<LiqRGBA> = palette_rgb
        .iter()
        .map(|p| LiqRGBA {
            r: p[0],
            g: p[1],
            b: p[2],
            a: 255,
        })
        .collect();

    let attr = LiqAttr::new();
    let mut liq_img = LiqImage::new_borrowed(
        &attr,
        &rgba_pixels,
        w as usize,
        h as usize,
        0.0, // Treat as sRGB, as recommended by imagequant docs
    )
    .map_err(|e| {
        error::AnsiImageError::Processing(format!("imagequant new_image failed: {e:?}"))
    })?;

    // Use the provided fixed palette instead of generating a new one.
    let mut res = LiqResult::from_palette(&attr, &fixed_palette, 0.0).map_err(|e| {
        error::AnsiImageError::Processing(format!("imagequant from_palette failed: {e:?}"))
    })?;

    // Dithering strength: 1.0 = strongest, 0.0 = none.
    let level = if dithering_enabled { 1.0 } else { 0.0 };
    res.set_dithering_level(level).map_err(|e| {
        error::AnsiImageError::Processing(format!("imagequant set_dithering_level failed: {e:?}"))
    })?;

    // Remap the image to the palette and get the resulting pixel indices.
    let (out_palette, indices) = res.remapped(&mut liq_img).map_err(|e| {
        error::AnsiImageError::Processing(format!("imagequant remapped failed: {e:?}"))
    })?;

    // Expand the indexed pixels back into a full RGB image for the next processing stage.
    let mut out_buffer = Vec::with_capacity(indices.len() * 3);
    for &idx in &indices {
        let c = out_palette[idx as usize];
        out_buffer.extend_from_slice(&[c.r, c.g, c.b]);
    }

    image::RgbImage::from_vec(w, h, out_buffer).ok_or_else(|| {
        error::AnsiImageError::Processing(
            "Failed to construct RgbImage from quantized buffer".into(),
        )
    })
}
