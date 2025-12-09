//! Error types for registry parsing operations.
//!
//! This module provides comprehensive error handling for all registry parsing
//! operations, including I/O errors, format violations, and data corruption.

use std::io;
use thiserror::Error;

/// Result type alias for registry operations.
pub type Result<T> = std::result::Result<T, RegistryError>;

/// Errors that can occur during registry parsing.
#[derive(Error, Debug)]
pub enum RegistryError {
    /// I/O error occurred while reading the hive file.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Invalid magic signature in header or structure.
    #[error("Invalid signature: expected {expected:?}, found {found:?}")]
    InvalidSignature {
        expected: Vec<u8>,
        found: Vec<u8>,
    },

    /// Invalid hive format or corrupted data.
    #[error("Invalid hive format: {0}")]
    InvalidFormat(String),

    /// Cell offset is out of bounds.
    #[error("Invalid cell offset: {offset:#x} (hive size: {hive_size:#x})")]
    InvalidOffset {
        offset: u32,
        hive_size: usize,
    },

    /// Cell size is invalid or corrupted.
    #[error("Invalid cell size: {size} at offset {offset:#x}")]
    InvalidCellSize {
        size: i32,
        offset: u32,
    },

    /// Unknown or unsupported cell type.
    #[error("Unknown cell type: {cell_type:?} at offset {offset:#x}")]
    UnknownCellType {
        cell_type: [u8; 2],
        offset: u32,
    },

    /// Key or value not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid UTF-16 string data.
    #[error("Invalid UTF-16 string at offset {offset:#x}")]
    InvalidUtf16 {
        offset: u32,
    },

    /// Invalid registry value type.
    #[error("Invalid value type: {0}")]
    InvalidValueType(u32),

    /// Hive is too small to be valid.
    #[error("Hive too small: {size} bytes (minimum: {minimum} bytes)")]
    HiveTooSmall {
        size: usize,
        minimum: usize,
    },

    /// Checksum mismatch in hive header.
    #[error("Checksum mismatch: expected {expected:#x}, calculated {calculated:#x}")]
    ChecksumMismatch {
        expected: u32,
        calculated: u32,
    },

    /// Unsupported hive version.
    #[error("Unsupported hive version: {major}.{minor}")]
    UnsupportedVersion {
        major: u32,
        minor: u32,
    },

    /// Data truncated or incomplete.
    #[error("Truncated data at offset {offset:#x}: expected {expected} bytes, got {actual} bytes")]
    TruncatedData {
        offset: u32,
        expected: usize,
        actual: usize,
    },

    /// Invalid subkey list type.
    #[error("Invalid subkey list type: {list_type:?}")]
    InvalidSubkeyList {
        list_type: [u8; 2],
    },

    /// Big data blocks (values > 16KB) not supported.
    #[error("Big data blocks not supported: value size is {size} bytes (max supported: {max_supported} bytes)")]
    BigDataNotSupported {
        /// Size of the value that was attempted to be read
        size: u32,
        /// Maximum supported size
        max_supported: u32,
    },
}

impl RegistryError {
    /// Creates an invalid signature error with context.
    ///
    /// # Arguments
    ///
    /// * `expected` - Expected signature bytes
    /// * `found` - Actual signature bytes found
    pub fn invalid_signature(expected: &[u8], found: &[u8]) -> Self {
        Self::InvalidSignature {
            expected: expected.to_vec(),
            found: found.to_vec(),
        }
    }

    /// Creates an invalid offset error with context.
    ///
    /// # Arguments
    ///
    /// * `offset` - The invalid offset
    /// * `hive_size` - Total size of the hive for context
    pub fn invalid_offset(offset: u32, hive_size: usize) -> Self {
        Self::InvalidOffset { offset, hive_size }
    }

    /// Creates an invalid cell size error with context.
    ///
    /// # Arguments
    ///
    /// * `size` - The invalid cell size
    /// * `offset` - Offset where the invalid size was found
    pub fn invalid_cell_size(size: i32, offset: u32) -> Self {
        Self::InvalidCellSize { size, offset }
    }

    /// Creates a format error with detailed context.
    ///
    /// # Arguments
    ///
    /// * `message` - Description of the format error
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use reg_parser::error::RegistryError;
    /// let len = 256;
    /// let offset = 0x1000;
    /// let err = RegistryError::format_error(
    ///     format!("Invalid key name length: {} at offset {:#x}", len, offset)
    /// );
    /// ```
    pub fn format_error(message: String) -> Self {
        Self::InvalidFormat(message)
    }

    /// Creates a not found error with context about what was being searched.
    ///
    /// # Arguments
    ///
    /// * `item_type` - Type of item (e.g., "key", "value")
    /// * `name` - Name of the item that wasn't found
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use reg_parser::error::RegistryError;
    /// let err = RegistryError::not_found("value", "DisplayName");
    /// ```
    pub fn not_found(item_type: &str, name: &str) -> Self {
        Self::NotFound(format!("{} '{}'", item_type, name))
    }

    /// Creates an unknown cell type error.
    pub fn unknown_cell_type(cell_type: [u8; 2], offset: u32) -> Self {
        Self::UnknownCellType { cell_type, offset }
    }
}
