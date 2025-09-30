//! This module defines predefined character sets used for ASCII art generation.
//!
//! The character sets are ordered by their perceived visual "brightness" or density,
//! allowing the processing logic to map image brightness to a suitable character.

/// A comprehensive list of ASCII characters sorted by ascending brightness.
pub const ASCII_CHARS_ALL: &[char] = &[
    ' ', '`', '.', '-', '\'', ':', '_', ',', '^', '=', ';', '>', '<', '+', '!', 'r', 'c', '*', '/',
    'z', '?', 's', 'L', 'T', 'v', ')', 'J', '7', '(', '|', 'F', 'i', '{', 'C', '}', 'f', 'I', '3',
    '1', 't', 'l', 'u', '[', 'n', 'e', 'o', 'Z', '5', 'Y', 'x', 'j', 'y', 'a', ']', '2', 'E', 'S',
    'w', 'q', 'k', 'P', '6', 'h', '9', 'd', '4', 'V', 'p', 'O', 'G', 'b', 'U', 'A', 'K', 'X', 'H',
    'm', '8', 'R', 'D', '#', '$', 'B', 'g', '0', 'M', 'N', 'W', 'Q', '%', '&', '@',
];

/// A list of ASCII characters sorted by ascending brightness, excluding the space character.
pub const ASCII_CHARS_NO_SPACE: &[char] = &[
    '`', '.', '-', '\'', ':', '_', ',', '^', '=', ';', '>', '<', '+', '!', 'r', 'c', '*', '/', 'z',
    '?', 's', 'L', 'T', 'v', ')', 'J', '7', '(', '|', 'F', 'i', '{', 'C', '}', 'f', 'I', '3', '1',
    't', 'l', 'u', '[', 'n', 'e', 'o', 'Z', '5', 'Y', 'x', 'j', 'y', 'a', ']', '2', 'E', 'S', 'w',
    'q', 'k', 'P', '6', 'h', '9', 'd', '4', 'V', 'p', 'O', 'G', 'b', 'U', 'A', 'K', 'X', 'H', 'm',
    '8', 'R', 'D', '#', '$', 'B', 'g', '0', 'M', 'N', 'W', 'Q', '%', '&', '@',
];

/// A subset of `ASCII_CHARS_ALL` containing only alphabetic characters.
pub const ASCII_CHARS_AZ: &[char] = &[
    'r', 'c', 'z', 's', 'L', 'T', 'v', 'J', 'F', 'i', 'C', 'f', 'I', 't', 'l', 'u', 'n', 'e', 'o',
    'Z', 'Y', 'x', 'j', 'y', 'a', 'E', 'S', 'w', 'q', 'k', 'P', 'h', 'd', 'V', 'p', 'O', 'G', 'b',
    'U', 'A', 'K', 'X', 'H', 'm', 'R', 'D', 'B', 'g', 'M', 'N', 'W', 'Q',
];

/// A subset of `ASCII_CHARS_ALL` containing only numeric characters.
pub const ASCII_CHARS_NUM: &[char] = &['7', '3', '1', '5', '2', '6', '9', '4', '8', '0'];

/// A subset of `ASCII_CHARS_ALL` containing only special (non-alphanumeric) characters.
pub const ASCII_CHARS_SPEC: &[char] = &[
    '`', '.', '-', '\'', ':', '_', ',', '^', '=', ';', '>', '<', '+', '!', '*', '/', '?', ')', '(',
    '|', '{', '}', '[', ']', '#', '$', '%', '&', '@',
];
