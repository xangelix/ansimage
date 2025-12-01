//! Contains all configuration structures for customizing the image conversion process.

use fast_image_resize::FilterType as ResizeFilter;
use image::Rgb;

/// A type alias for a color palette, represented as a vector of RGB colors.
pub type ColorPalette<C> = Vec<C>;

/// Top-level configuration for the image conversion process.
///
/// This struct aggregates all settings related to size, characters, colors,
/// and advanced algorithms.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Sizing and dimension settings.
    pub size: Size,
    /// Character set and rendering style settings.
    pub characters: Characters,
    /// Color palette and mode settings.
    pub colors: Colors,
    /// Advanced options like resizing and dithering algorithms.
    pub advanced: Advanced,
}

impl Default for Settings {
    /// Creates a default `Settings` configuration.
    ///
    /// - **Size**: 80x40 characters, fitting while preserving aspect ratio.
    /// - **Characters**: Full ASCII set, two-color mode, 0.5 aspect ratio.
    /// - **Colors**: Truecolor enabled.
    /// - **Advanced**: Lanczos3 resize filter, dithering enabled.
    fn default() -> Self {
        Self {
            size: Size::default(),
            characters: Characters::default(),
            colors: Colors::default(),
            advanced: Advanced::default(),
        }
    }
}

/// Defines the target dimensions and sizing mode for the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Size {
    /// The target width in terminal character cells.
    pub width: usize,
    /// The target height in terminal character cells.
    pub height: usize,
    /// The sizing strategy (`Fit` or `Exact`).
    pub mode: SizeMode,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            width: 80,
            height: 40,
            mode: SizeMode::Fit,
        }
    }
}

/// Specifies how to interpret the `width` and `height` fields in [`Size`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeMode {
    /// Scale the image to fit within the `width` and `height` bounds while
    /// preserving the original aspect ratio.
    Fit,
    /// Stretch or shrink the image to the exact `width` and `height`,
    /// potentially altering the aspect ratio.
    Exact,
}

/// Configures the character set, color usage, and aspect ratio compensation.
#[derive(Debug, Clone)]
pub struct Characters {
    /// The primary character mode to use for rendering.
    pub mode: CharacterMode,
    /// The color rendering mode (`OneColor` for foreground only, or `TwoColor`
    /// for both foreground and background).
    pub color_mode: ColorMode,
    /// The width-to-height ratio of a single character in the terminal font.
    /// A typical value is `0.5` for monospaced fonts where characters are
    /// roughly twice as tall as they are wide. This is used to correct the
    /// image aspect ratio.
    pub aspect_ratio: f32,
}

impl Default for Characters {
    fn default() -> Self {
        Self {
            mode: CharacterMode::Ascii(AsciiCharSet::All),
            color_mode: ColorMode::TwoColor,
            aspect_ratio: 0.5,
        }
    }
}

/// Defines which set of characters to use for rendering.
#[derive(Debug, Clone)]
pub enum CharacterMode {
    /// Use a predefined set of standard ASCII characters, chosen based on brightness.
    Ascii(AsciiCharSet),
    /// Use a predefined set of Unicode block-drawing characters.
    Unicode(UnicodeCharSet),
    /// Use a user-provided vector of custom characters. For best results,
    /// the vector should be sorted from darkest to brightest character.
    Custom(Vec<char>),
}

/// Predefined sets of ASCII characters, ordered by perceived brightness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiCharSet {
    /// All available ASCII characters from the brightness ramp.
    All,
    /// All available ASCII characters except the space character.
    NoSpace,
    /// Alphabetical characters only (`a-z`, `A-Z`).
    Az,
    /// Numeric characters only (`0-9`).
    Nums,
    /// Special characters only (`!`, `@`, `#`, etc.).
    Spec,
}

impl AsciiCharSet {
    /// Returns the character slice corresponding to the enum variant.
    #[must_use]
    pub const fn as_slice(&self) -> &'static [char] {
        match self {
            Self::All => crate::sets::ASCII_CHARS_ALL,
            Self::NoSpace => crate::sets::ASCII_CHARS_NO_SPACE,
            Self::Az => crate::sets::ASCII_CHARS_AZ,
            Self::Nums => crate::sets::ASCII_CHARS_NUM,
            Self::Spec => crate::sets::ASCII_CHARS_SPEC,
        }
    }
}

/// Predefined sets of Unicode block-drawing characters for higher-fidelity output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeCharSet {
    /// Solid block (`█`), using only the average color of each 2x2 block.
    Full,
    /// Half blocks (`▀`), using one color for the top half and one for the bottom.
    Half,
    /// Quarter blocks and half blocks (`▖`, `▘`, `▝`, `▗`, `▐`, etc.), offering
    /// the highest spatial resolution.
    Quarter,
    /// Shade characters (`░`, `▒`, `▓`), which represent different brightness levels.
    Shade,
}

/// Determines whether to use both foreground and background colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// Use only a foreground color. The background will be the terminal's default.
    OneColor,
    /// Use both a foreground and a background color for each character cell.
    TwoColor,
}

/// Configures the color palette and mode for the output.
#[derive(Debug, Clone)]
pub struct Colors {
    /// If `true`, output 24-bit RGB ("truecolor") ANSI escape codes. This
    /// provides the highest color fidelity.
    pub is_truecolor: bool,
    /// A palette of colors to quantize the image to if `is_truecolor` is `false`.
    /// Required for terminals that do not support truecolor.
    pub palette: ColorPalette<Rgb<u8>>,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            is_truecolor: true,
            palette: vec![],
        }
    }
}

/// Advanced settings for image processing algorithms.
#[derive(Debug, Clone, Copy)]
pub struct Advanced {
    /// The resampling filter to use when resizing the image.
    /// `Lanczos3` is a high-quality default.
    pub resize_filter: ResizeFilter,
    /// Dithering configuration.
    pub dithering: Dithering,
    /// If true, only emits ANSI codes when colors change.
    /// If false, emits codes for every character (larger output).
    pub compression: bool,
}

impl Default for Advanced {
    fn default() -> Self {
        Self {
            resize_filter: ResizeFilter::Lanczos3,
            dithering: Dithering::default(),
            compression: true,
        }
    }
}

/// Configures the dithering algorithm applied during color quantization.
///
/// Dithering is a technique used to create the illusion of more colors when
/// working with a limited palette. It is only applied when not in truecolor mode.
#[derive(Debug, Clone, Copy)]
pub struct Dithering {
    /// Set to `true` to enable dithering.
    pub is_enabled: bool,
    /// The dithering matrix/algorithm to use. Currently, only one option is
    /// available via `imagequant`, but this field is for future expansion.
    pub matrix: DitherMatrix,
}

impl Default for Dithering {
    fn default() -> Self {
        Self {
            is_enabled: true,
            // imagequant's dithering is similar to Floyd-Steinberg.
            matrix: DitherMatrix::FloydSteinberg,
        }
    }
}

/// Represents a dithering algorithm matrix.
///
/// **Note**: This is currently a placeholder for future extension, as the
/// backend `imagequant` uses its own internal ordered dithering logic which
/// is not selectable beyond on/off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMatrix {
    /// Floyd-Steinberg error-diffusion dithering algorithm.
    FloydSteinberg,
    /// Jarvis, Judice, and Ninke error-diffusion dithering.
    JarvisJudiceNinke,
    /// Stucki error-diffusion dithering algorithm.
    Stucki,
    /// Burkes error-diffusion dithering algorithm.
    Burkes,
}
