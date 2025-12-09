//! Big data block (db) parsing.
//!
//! Registry values larger than 16,344 bytes are stored in big data blocks,
//! which consist of a header cell followed by multiple data segments.

use crate::error::{RegistryError, Result};
use crate::utils::read_u16_le;

/// Big data block header structure.
///
/// Format:
/// ```text
/// Offset  Size  Description
/// 0x00    2     Signature ("db")
/// 0x02    2     Number of segments
/// 0x04    4     Offset to segment list
/// ```
#[derive(Debug, Clone)]
pub struct BigDataBlock {
    /// Number of data segments
    pub segment_count: u16,
    
    /// Offset to the list of segment offsets
    pub segment_list_offset: u32,
}

impl BigDataBlock {
    /// Minimum size of a big data block header
    const MIN_SIZE: usize = 8;
    
    /// Parses a big data block header from cell data.
    ///
    /// # Arguments
    ///
    /// * `data` - Cell data (excluding size field, starting with "db" signature)
    /// * `offset` - Offset of this cell for error reporting
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or truncated.
    pub fn parse(data: &[u8], offset: u32) -> Result<Self> {
        if data.len() < Self::MIN_SIZE {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: Self::MIN_SIZE,
                actual: data.len(),
            });
        }

        // Verify signature
        if &data[0..2] != b"db" {
            return Err(RegistryError::InvalidFormat(format!(
                "Expected 'db' signature at offset {:#x}, found {:?}",
                offset,
                &data[0..2]
            )));
        }

        let segment_count = read_u16_le(data, 0x02)?;
        
        // Segment list offset is stored at 0x04 (4 bytes)
        // Note: This is a cell offset, not an absolute offset
        let segment_list_offset = u32::from_le_bytes([
            data[0x04],
            data[0x05],
            data[0x06],
            data[0x07],
        ]);

        Ok(BigDataBlock {
            segment_count,
            segment_list_offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigdata_minimum_size() {
        let data = vec![0u8; 7];
        let result = BigDataBlock::parse(&data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_bigdata_invalid_signature() {
        let mut data = vec![0u8; 8];
        data[0..2].copy_from_slice(b"XX");
        let result = BigDataBlock::parse(&data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_bigdata_valid() {
        let mut data = vec![0u8; 8];
        data[0..2].copy_from_slice(b"db");
        data[2] = 5; // segment_count low byte
        data[3] = 0; // segment_count high byte
        data[4..8].copy_from_slice(&[0x20, 0x00, 0x00, 0x00]); // segment_list_offset
        
        let db = BigDataBlock::parse(&data, 0).unwrap();
        assert_eq!(db.segment_count, 5);
        assert_eq!(db.segment_list_offset, 0x20);
    }
}
