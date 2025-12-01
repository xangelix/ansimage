//! Core logic for processing image pixels into styled terminal characters.
//!
//! This module contains the functions responsible for analyzing 2x2 pixel blocks,
//! selecting the best character to represent them, and determining the appropriate
//! foreground and background colors according to the user's settings.

use std::fmt::Write as _;

use image::{Rgb, RgbImage};
use palette::{Luv, Srgb, convert::FromColorUnclamped, white_point::D65};

use crate::{
    BLACK_LUV,
    settings::{CharacterMode, ColorMode, ColorPalette, Settings, UnicodeCharSet},
};

/// A type alias for the CIE L*u*v* color type used throughout the processing pipeline.
///
/// L*u*v* is a perceptually uniform color space, which means that the geometric
/// distance between two colors in this space corresponds well to the perceived
/// difference in color by the human eye. This property is crucial for accurate
/// color matching and difference calculations.
pub type LuvColor = Luv<D65, f32>;

/// A type alias for RGB colors represented as tuples of u8 components.
type RGB8 = (u8, u8, u8);

/// Converts an sRGB pixel to the L*u*v* color space.
#[inline]
fn pixel_to_luv(p: Rgb<u8>) -> LuvColor {
    // Normalize sRGB u8 components to f32 values between 0.0 and 1.0.
    let srgb = Srgb::new(
        p.0[0] as f32 / 255.0,
        p.0[1] as f32 / 255.0,
        p.0[2] as f32 / 255.0,
    );
    // Convert the linear sRGB color to the L*u*v* color space.
    LuvColor::from_color_unclamped(srgb)
}

/// Converts a L*u*v* color back to a simple RGB tuple
#[inline]
fn luv_to_rgb(luv: LuvColor) -> RGB8 {
    // Convert back to sRGB.
    let srgb = Srgb::from_color_unclamped(luv);
    // Denormalize and clamp the f32 components to u8 values (0-255).
    (
        (srgb.red * 255.0).round().clamp(0.0, 255.0) as u8,
        (srgb.green * 255.0).round().clamp(0.0, 255.0) as u8,
        (srgb.blue * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// Processes a single character row of the output image.
///
/// This function iterates over the pixels corresponding to one row of the final
/// output, processing each 2x2 pixel block into a styled character. It is
/// designed to be called in parallel for each row to improve performance.
pub fn process_row(
    y_char: usize,
    width_char: usize,
    img: &RgbImage,
    settings: &Settings,
) -> String {
    // Pre-allocate a reasonable capacity for the row string to reduce reallocations.
    // An average ANSI escape sequence is roughly 15 bytes.
    let mut row_str = String::with_capacity(width_char * 15);
    let y_px = y_char * 2;

    // Pre-convert the sRGB palette to L*u*v* once per row if not in truecolor mode.
    let paletted_colors = if settings.colors.is_truecolor {
        None
    } else {
        Some(
            settings
                .colors
                .palette
                .iter()
                .map(|&c| Srgb::new(c.0[0], c.0[1], c.0[2]).into_format())
                .map(LuvColor::from_color_unclamped)
                .collect::<Vec<_>>(),
        )
    };

    // State tracking for compression of ANSI escape sequences.
    let mut last_fg: Option<RGB8> = None;
    let mut last_bg: Option<RGB8> = None;

    for x_char in 0..width_char {
        let x_px = x_char * 2;

        // Extract the 2x2 pixel block and convert to L*u*v*.
        // The loops are constructed to guarantee these `get_pixel` calls are in-bounds.
        let colors = [
            pixel_to_luv(*img.get_pixel(x_px as u32, y_px as u32)),
            pixel_to_luv(*img.get_pixel(x_px as u32 + 1, y_px as u32)),
            pixel_to_luv(*img.get_pixel(x_px as u32, y_px as u32 + 1)),
            pixel_to_luv(*img.get_pixel(x_px as u32 + 1, y_px as u32 + 1)),
        ];

        // Retrieve raw color data (Options)
        let (character, fg, bg) = if let CharacterMode::Unicode(charset) = settings.characters.mode
        {
            process_unicode(
                &colors,
                charset,
                settings.characters.color_mode,
                paletted_colors.as_ref(),
            )
        } else {
            let char_set: &[char] = match &settings.characters.mode {
                CharacterMode::Ascii(cs) => cs.as_slice(),
                CharacterMode::Custom(v) => v,
                CharacterMode::Unicode(_) => unreachable!(),
            };
            process_ascii(
                &colors,
                char_set,
                settings.characters.color_mode,
                paletted_colors.as_ref(),
            )
        };

        // --- Compression ---

        // Write the code if the color changed OR if compression is disabled.
        let write_fg = fg != last_fg || !settings.advanced.compression;
        let write_bg = bg != last_bg || !settings.advanced.compression;

        // 1. Handle Foreground Change
        if write_fg {
            match fg {
                // ANSI truecolor foreground: \x1b[38;2;R;G;Bm
                Some(c) => write!(row_str, "\x1b[38;2;{};{};{}m", c.0, c.1, c.2).unwrap(),
                // Reset foreground only: \x1b[39m
                None => write!(row_str, "\x1b[39m").unwrap(),
            }
            last_fg = fg;
        }

        // 2. Handle Background Change
        if write_bg {
            match bg {
                // ANSI truecolor background: \x1b[48;2;R;G;Bm
                Some(c) => write!(row_str, "\x1b[48;2;{};{};{}m", c.0, c.1, c.2).unwrap(),
                // Reset background only: \x1b[49m
                None => write!(row_str, "\x1b[49m").unwrap(),
            }
            last_bg = bg;
        }

        // 3. Write the character
        row_str.push(character);
    }

    // Reset everything at the end of the line so the terminal doesn't bleed colors
    row_str.push_str("\x1b[0m");
    row_str
}

/// Determines the best character and style for an ASCII/Custom character block.
///
/// This mode uses brightness ramps to select an appropriate character from the
/// provided character set.
fn process_ascii(
    colors: &[LuvColor; 4],
    char_set: &[char],
    color_mode: ColorMode,
    palette: Option<&ColorPalette<LuvColor>>,
) -> (char, Option<RGB8>, Option<RGB8>) {
    if color_mode == ColorMode::TwoColor {
        let (lightest, darkest) = find_lightest_darkest(colors);

        let (fg_luv, bg_luv) = palette.map_or((lightest, darkest), |p| {
            find_closest_pair(lightest, darkest, p, true)
        });

        let avg = average_color(colors);
        let total_dist = luv_distance(lightest, darkest);
        let avg_dist = luv_distance(avg, darkest);

        let brightness = if total_dist < 1e-5 {
            0.0
        } else {
            (avg_dist / total_dist).min(1.0)
        };
        let index = brightness_to_char_index(brightness, char_set.len());

        (
            char_set[index],
            Some(luv_to_rgb(fg_luv)),
            Some(luv_to_rgb(bg_luv)),
        )
    } else {
        // OneColor mode
        let avg_color = average_color(colors);
        let fg_luv = palette.map_or(avg_color, |p| find_closest(avg_color, p));

        let brightness = 1.0 - (luv_distance(fg_luv, BLACK_LUV) / 100.0).min(1.0);
        let index = brightness_to_char_index(brightness, char_set.len());

        (
            char_set[index],
            Some(luv_to_rgb(fg_luv)),
            None, // No background
        )
    }
}

/// Determines the best character and style for a Unicode block character.
///
/// This mode attempts to find the best-fitting block character by testing
/// several candidates and choosing the one with the lowest perceptual color
/// distance from the original 2x2 pixel block.
fn process_unicode(
    colors: &[LuvColor; 4],
    charset: UnicodeCharSet,
    color_mode: ColorMode,
    palette: Option<&ColorPalette<LuvColor>>,
) -> (char, Option<RGB8>, Option<RGB8>) {
    // Fast path for solid block characters, which don't need complex candidate testing.
    if charset == UnicodeCharSet::Full {
        let avg_color = average_color(colors);
        let final_color = palette.map_or(avg_color, |p| find_closest(avg_color, p));
        // Full block is just FG color
        return ('█', Some(luv_to_rgb(final_color)), None);
    }

    // Generate candidate characters and their ideal foreground/background colors.
    let candidates = match charset {
        UnicodeCharSet::Full => vec![('█', average_color(colors), BLACK_LUV)],
        UnicodeCharSet::Half => {
            vec![(
                '▀',
                average_color(&colors[0..2]),
                average_color(&colors[2..4]),
            )]
        }
        UnicodeCharSet::Quarter => vec![
            (
                '▀',
                average_color(&colors[0..2]),
                average_color(&colors[2..4]),
            ), // Top half
            (
                '▐',
                average_color(&[colors[1], colors[3]]),
                average_color(&[colors[0], colors[2]]),
            ), // Right half
            (
                '▞',
                average_color(&[colors[1], colors[2]]),
                average_color(&[colors[0], colors[3]]),
            ), // Diagonal
            (
                '▖',
                colors[2],
                average_color(&[colors[0], colors[1], colors[3]]),
            ), // Bottom-left
            (
                '▘',
                colors[0],
                average_color(&[colors[1], colors[2], colors[3]]),
            ), // Top-left
            (
                '▝',
                colors[1],
                average_color(&[colors[0], colors[2], colors[3]]),
            ), // Top-right
            (
                '▗',
                colors[3],
                average_color(&[colors[0], colors[1], colors[2]]),
            ), // Bottom-right
        ],
        UnicodeCharSet::Shade => vec![
            (' ', BLACK_LUV, BLACK_LUV),
            ('░', average_color(colors), BLACK_LUV), // Light shade
            ('▒', average_color(colors), BLACK_LUV), // Medium shade
            ('▓', average_color(colors), BLACK_LUV), // Dark shade
        ],
    };

    // Find the candidate that best represents the original 2x2 pixel block.
    let (best_char, best_fg, best_bg) = candidates
        .into_iter()
        .map(|(char_candidate, fg_candidate, bg_candidate)| {
            let (fg, bg) = palette.map_or((fg_candidate, bg_candidate), |p| {
                find_closest_pair(fg_candidate, bg_candidate, p, false)
            });
            let dist = calculate_block_distance(colors, fg, bg, char_candidate);
            (dist, char_candidate, fg, bg)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or((' ', BLACK_LUV, BLACK_LUV), |(_, c, fg, bg)| (c, fg, bg));

    let fg = Some(luv_to_rgb(best_fg));
    let bg = if color_mode == ColorMode::TwoColor {
        Some(luv_to_rgb(best_bg))
    } else {
        None
    };

    (best_char, fg, bg)
}

/// Calculates the Euclidean distance between two L*u*v* colors (CIEDE76).
///
/// The formula is: $\sqrt{\Delta L^2 + \Delta u^2 + \Delta v^2}$
#[inline]
fn luv_distance(c1: LuvColor, c2: LuvColor) -> f32 {
    let (l1, u1, v1) = c1.into_components();
    let (l2, u2, v2) = c2.into_components();
    let dl = l1 - l2;
    let du = u1 - u2;
    let dv = v1 - v2;
    // Use fused multiply-add for a potential performance micro-optimization.
    dv.mul_add(dv, dl.mul_add(dl, du * du)).sqrt()
}

/// Calculates the total perceptual distance of a 2x2 color block against a candidate
/// character's foreground/background pattern.
///
/// This determines how well the character represents the original pixels by summing
/// the squared color distances for each of the four sub-pixels.
fn calculate_block_distance(
    original: &[LuvColor; 4],
    fg: LuvColor,
    bg: LuvColor,
    character: char,
) -> f32 {
    let (c1, c2, c3, c4) = (original[0], original[1], original[2], original[3]);

    // Map the character to its corresponding 2x2 color pattern.
    let (t1, t2, t3, t4) = match character {
        '▀' => (fg, fg, bg, bg),
        '▐' => (bg, fg, bg, fg),
        '▞' => (bg, fg, fg, bg),
        '▖' => (bg, bg, fg, bg),
        '▘' => (fg, bg, bg, bg),
        '▝' => (bg, fg, bg, bg),
        '▗' => (bg, bg, bg, fg),
        '█' => (fg, fg, fg, fg),
        '░' => (
            blend(fg, bg, 0.25),
            blend(fg, bg, 0.25),
            blend(fg, bg, 0.25),
            blend(fg, bg, 0.25),
        ),
        '▒' => (
            blend(fg, bg, 0.50),
            blend(fg, bg, 0.50),
            blend(fg, bg, 0.50),
            blend(fg, bg, 0.50),
        ),
        '▓' => (
            blend(fg, bg, 0.75),
            blend(fg, bg, 0.75),
            blend(fg, bg, 0.75),
            blend(fg, bg, 0.75),
        ),
        _ => (bg, bg, bg, bg), // Includes space ' '
    };

    let d1 = luv_distance(c1, t1);
    let d2 = luv_distance(c2, t2);
    let d3 = luv_distance(c3, t3);
    let d4 = luv_distance(c4, t4);

    // Return sum of squared distances.
    d4.mul_add(d4, d3.mul_add(d3, d1.mul_add(d1, d2 * d2)))
}

/// Linearly interpolates between two colors by a given ratio.
#[inline]
fn blend(a: LuvColor, b: LuvColor, ratio: f32) -> LuvColor {
    Luv::new(
        a.l.mul_add(ratio, b.l * (1.0 - ratio)),
        a.u.mul_add(ratio, b.u * (1.0 - ratio)),
        a.v.mul_add(ratio, b.v * (1.0 - ratio)),
    )
}

/// Computes the average color from a slice of L*u*v* colors.
#[inline]
fn average_color(colors: &[LuvColor]) -> LuvColor {
    let count = colors.len() as f32;
    if count == 0.0 {
        return BLACK_LUV;
    }
    let (l_sum, u_sum, v_sum) = colors
        .iter()
        .fold((0.0, 0.0, 0.0), |(l, u, v), c| (l + c.l, u + c.u, v + c.v));
    Luv::new(l_sum / count, u_sum / count, v_sum / count)
}

/// Finds the colors with the highest and lowest lightness (L*) component in a slice.
#[inline]
fn find_lightest_darkest(colors: &[LuvColor]) -> (LuvColor, LuvColor) {
    let mut lightest = colors[0];
    let mut darkest = colors[0];
    for &c in colors.iter().skip(1) {
        if c.l > lightest.l {
            lightest = c;
        }
        if c.l < darkest.l {
            darkest = c;
        }
    }
    (lightest, darkest)
}

/// Finds the single closest color in a palette to a given color.
fn find_closest(color: LuvColor, palette: &ColorPalette<LuvColor>) -> LuvColor {
    palette
        .iter()
        .min_by(|&&c1, &&c2| {
            let d1 = luv_distance(color, c1);
            let d2 = luv_distance(color, c2);
            d1.total_cmp(&d2)
        })
        .copied()
        .unwrap_or(color)
}

/// Finds the two best-matching colors from a palette for a given pair of colors.
///
/// If `order_by_brightness` is `true`, it ensures the two returned colors are from
/// distinct palette entries and are ordered with the darker color second (for bg).
fn find_closest_pair(
    color1: LuvColor,
    color2: LuvColor,
    palette: &ColorPalette<LuvColor>,
    order_by_brightness: bool,
) -> (LuvColor, LuvColor) {
    if palette.is_empty() {
        return (BLACK_LUV, BLACK_LUV);
    }
    if palette.len() == 1 {
        return (palette[0], palette[0]);
    }

    // For Unicode characters, spatial position matters more than brightness. Find the
    // closest color for fg and bg independently without ensuring they are distinct.
    if !order_by_brightness {
        return (find_closest(color1, palette), find_closest(color2, palette));
    }

    // For ASCII brightness ramps, find the best two *distinct* colors from the palette.
    let (mut closest1, mut min_dist1, mut idx1) = (palette[0], f32::MAX, 0);

    for (i, &p_color) in palette.iter().enumerate() {
        let dist = luv_distance(color1, p_color);
        if dist < min_dist1 {
            min_dist1 = dist;
            closest1 = p_color;
            idx1 = i;
        }
    }

    let mut closest2 = if idx1 == 0 { palette[1] } else { palette[0] };
    let mut min_dist2 = f32::MAX;

    for (i, &p_color) in palette.iter().enumerate() {
        if i == idx1 {
            continue; // Ensure the second color is from a different palette entry.
        }
        let dist = luv_distance(color2, p_color);
        if dist < min_dist2 {
            min_dist2 = dist;
            closest2 = p_color;
        }
    }

    // Return the pair as (foreground, background), where the background is the color
    // perceptually closer to black.
    if luv_distance(closest1, BLACK_LUV) < luv_distance(closest2, BLACK_LUV) {
        (closest2, closest1)
    } else {
        (closest1, closest2)
    }
}

/// Maps a brightness value (0.0 to 1.0) to an index in a character set.
#[inline]
fn brightness_to_char_index(brightness: f32, char_set_len: usize) -> usize {
    let len_f = (char_set_len - 1) as f32;
    let index = (brightness * len_f).round() as usize;
    index.min(char_set_len - 1)
}

#[cfg(test)]
mod tests {
    use super::brightness_to_char_index;

    #[test]
    fn brightness_index_bounds() {
        assert_eq!(brightness_to_char_index(0.0, 10), 0);
        assert_eq!(brightness_to_char_index(1.0, 10), 9);
        assert_eq!(brightness_to_char_index(-0.1, 10), 0);
        assert_eq!(brightness_to_char_index(1.1, 10), 9);
    }
}
