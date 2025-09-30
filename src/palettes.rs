//! This module contains predefined color palettes for use in non-truecolor mode.

use image::Rgb;

/// The "Sweetie 16" color palette by `GrafxKid`.
///
/// A 16-color palette with a cool, muted, and slightly retro aesthetic.
/// See: <https://lospec.com/palette-list/sweetie-16>
pub const COLOR_PALETTE_SWEETIE16: &[Rgb<u8>] = &[
    Rgb([0x1a, 0x1c, 0x2c]), // Dark purple
    Rgb([0x5d, 0x27, 0x5d]), // Deep magenta
    Rgb([0xb1, 0x3e, 0x53]), // Muted red
    Rgb([0xef, 0x7d, 0x57]), // Orange
    Rgb([0xff, 0xcd, 0x75]), // Yellow
    Rgb([0xa7, 0xf0, 0x70]), // Lime green
    Rgb([0x38, 0xb7, 0x64]), // Green
    Rgb([0x25, 0x71, 0x79]), // Teal
    Rgb([0x29, 0x36, 0x6f]), // Dark blue
    Rgb([0x3b, 0x5d, 0xc9]), // Blue
    Rgb([0x41, 0xa6, 0xf6]), // Sky blue
    Rgb([0x73, 0xef, 0xf7]), // Cyan
    Rgb([0xf4, 0xf4, 0xf4]), // White
    Rgb([0x94, 0xb0, 0xc2]), // Light grey
    Rgb([0x56, 0x6c, 0x86]), // Grey
    Rgb([0x33, 0x3c, 0x57]), // Dark grey
];
