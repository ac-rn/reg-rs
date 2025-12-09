//! Registry hive base block (header) parsing.
//!
//! The base block is the first 4096 bytes (0x1000) of a registry hive file.
//! It contains metadata about the hive including version, timestamps, and
//! the root key cell offset.

use crate::error::{RegistryError, Result};
use crate::utils::{calculate_checksum, read_fixed_ascii, read_u32_le};
use std::fmt;

/// Size of the base block in bytes.
pub const BASE_BLOCK_SIZE: usize = 4096;

/// Expected signature for a valid registry hive ("regf").
pub const REGF_SIGNATURE: &[u8; 4] = b"regf";

/// Offset of the file name in the base block.
const FILE_NAME_OFFSET: usize = 0x30;

/// Length of the file name field (64 UTF-16LE characters = 128 bytes).
const FILE_NAME_LENGTH: usize = 128;

/// Offset of the checksum field in the base block.
const CHECKSUM_OFFSET: usize = 0x1FC;

/// Registry hive base block header.
///
/// This structure represents the first 4KB of a registry hive file and contains
/// critical metadata about the hive.
#[derive(Debug, Clone)]
pub struct BaseBlock {
    /// Signature, should be "regf" (0x66676572).
    pub signature: [u8; 4],
    
    /// Primary sequence number.
    pub primary_sequence: u32,
    
    /// Secondary sequence number.
    pub secondary_sequence: u32,
    
    /// Last written timestamp (Windows FILETIME).
    pub last_written: u64,
    
    /// Major version of the hive format.
    pub major_version: u32,
    
    /// Minor version of the hive format.
    pub minor_version: u32,
    
    /// File type (0 = normal, 1 = transaction log).
    pub file_type: u32,
    
    /// File format (1 = direct memory load).
    pub file_format: u32,
    
    /// Offset to root key cell (relative to first hbin).
    pub root_cell_offset: u32,
    
    /// Length of hive data in bytes.
    pub hive_length: u32,
    
    /// Clustering factor (always 1).
    pub clustering_factor: u32,
    
    /// File name (embedded, 64 UTF-16LE characters).
    pub file_name: String,
    
    /// Checksum (XOR of first 508 bytes).
    pub checksum: u32,
}

impl BaseBlock {
    /// Parses a base block from raw bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes of the base block (must be at least 4096 bytes).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Data is too small
    /// - Signature is invalid
    /// - Checksum doesn't match
    /// - Version is unsupported
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < BASE_BLOCK_SIZE {
            return Err(RegistryError::HiveTooSmall {
                size: data.len(),
                minimum: BASE_BLOCK_SIZE,
            });
        }

        // Read signature
        let mut signature = [0u8; 4];
        signature.copy_from_slice(&data[0..4]);

        if &signature != REGF_SIGNATURE {
            return Err(RegistryError::invalid_signature(REGF_SIGNATURE, &signature));
        }

        // Parse header fields
        let primary_sequence = read_u32_le(data, 0x04)?;
        let secondary_sequence = read_u32_le(data, 0x08)?;
        
        // Last written timestamp (8 bytes at offset 0x0C)
        let last_written = u64::from(read_u32_le(data, 0x0C)?) 
            | (u64::from(read_u32_le(data, 0x10)?) << 32);
        
        let major_version = read_u32_le(data, 0x14)?;
        let minor_version = read_u32_le(data, 0x18)?;
        let file_type = read_u32_le(data, 0x1C)?;
        let file_format = read_u32_le(data, 0x20)?;
        let root_cell_offset = read_u32_le(data, 0x24)?;
        let hive_length = read_u32_le(data, 0x28)?;
        let clustering_factor = read_u32_le(data, 0x2C)?;
        
        // File name at offset 0x30 (64 UTF-16LE characters = 128 bytes)
        let file_name_bytes = &data[0x30..0xB0];
        let file_name = read_fixed_ascii(file_name_bytes, 64);
        
        // Checksum at offset 0x1FC
        let checksum = read_u32_le(data, 0x1FC)?;

        // Verify checksum
        let calculated = calculate_checksum(data);
        if checksum != calculated {
            return Err(RegistryError::ChecksumMismatch {
                expected: checksum,
                calculated,
            });
        }

        // Verify version (support 1.3, 1.4, 1.5, 1.6)
        if major_version != 1 || minor_version < 3 || minor_version > 6 {
            return Err(RegistryError::UnsupportedVersion {
                major: major_version,
                minor: minor_version,
            });
        }

        Ok(BaseBlock {
            signature,
            primary_sequence,
            secondary_sequence,
            last_written,
            major_version,
            minor_version,
            file_type,
            file_format,
            root_cell_offset,
            hive_length,
            clustering_factor,
            file_name,
            checksum,
        })
    }

    /// Returns true if the hive is in a consistent state.
    ///
    /// The hive is consistent when primary and secondary sequence numbers match.
    pub fn is_consistent(&self) -> bool {
        self.primary_sequence == self.secondary_sequence
    }

    /// Converts the last written timestamp to a human-readable format.
    pub fn last_written_datetime(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        // Windows FILETIME is 100-nanosecond intervals since 1601-01-01
        // Unix epoch is 1970-01-01, difference is 11644473600 seconds
        const FILETIME_UNIX_DIFF: i64 = 11644473600;
        
        let seconds = (self.last_written / 10_000_000) as i64 - FILETIME_UNIX_DIFF;
        let nanos = ((self.last_written % 10_000_000) * 100) as u32;
        
        chrono::DateTime::from_timestamp(seconds, nanos)
    }
}

impl fmt::Display for BaseBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Registry Hive Header:\n\
             - Version: {}.{}\n\
             - Root Cell Offset: {:#x}\n\
             - Hive Length: {} bytes\n\
             - Consistent: {}\n\
             - File Name: {}",
            self.major_version,
            self.minor_version,
            self.root_cell_offset,
            self.hive_length,
            self.is_consistent(),
            self.file_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_block_size() {
        assert_eq!(BASE_BLOCK_SIZE, 4096);
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = vec![0u8; BASE_BLOCK_SIZE];
        data[0..4].copy_from_slice(b"XXXX");
        
        let result = BaseBlock::parse(&data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::InvalidSignature { .. }));
    }

    #[test]
    fn test_too_small() {
        let data = vec![0u8; 100];
        let result = BaseBlock::parse(&data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::HiveTooSmall { .. }));
    }
}
