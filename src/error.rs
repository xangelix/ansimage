//! Defines the error types and `Result` alias for the `ansimage` crate.
//!
//! This module consolidates all possible failure modes into a single enum,
//! [`AnsiImageError`], for convenient error handling across the library. It
//! also provides a crate-specific [`Result`] type.

use thiserror::Error;

/// The primary `Result` type used throughout the `ansimage` crate.
pub type Result<T> = std::result::Result<T, AnsiImageError>;

/// The error enum for all fallible operations in the `ansimage` crate.
#[derive(Debug, Error)]
pub enum AnsiImageError {
    /// An error occurred during image loading, decoding, or format handling.
    /// This typically wraps errors from the underlying `image` crate.
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    /// An I/O error occurred, typically when reading an image file from disk.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The provided settings are invalid and cannot be used for conversion.
    /// The contained string provides a user-friendly explanation of the issue.
    ///
    /// For example, this error occurs if `is_truecolor` is false but no
    /// color palette is provided.
    #[error("Invalid settings: {0}")]
    InvalidSettings(String),

    /// An error occurred during an internal image processing step, such as
    /// resizing, color quantization, or buffer manipulation.
    #[error("Image processing failed: {0}")]
    Processing(String),
}
