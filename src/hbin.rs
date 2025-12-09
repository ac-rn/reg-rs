//! Hive bin (hbin) block parsing.
//!
//! Hive bins are 4KB-aligned blocks that contain registry cells. Each hbin
//! has a header followed by a sequence of cells.

use crate::error::{RegistryError, Result};
use crate::utils::read_u32_le;

/// Expected signature for hive bins ("hbin").
pub const HBIN_SIGNATURE: &[u8; 4] = b"hbin";

/// Minimum size of an hbin header.
pub const HBIN_HEADER_SIZE: usize = 0x20;

/// Hive bin header structure.
///
/// Each hbin contains a header followed by registry cells. Hbins are always
/// aligned to 4KB boundaries.
#[derive(Debug, Clone)]
pub struct HbinHeader {
    /// Signature, should be "hbin" (0x6E696268).
    pub signature: [u8; 4],
    
    /// Offset of this hbin from the start of the hive bins (relative to 0x1000).
    pub offset: u32,
    
    /// Size of this hbin in bytes (including header).
    pub size: u32,
    
    /// Reserved fields.
    pub reserved: [u32; 2],
    
    /// Timestamp (Windows FILETIME).
    pub timestamp: u64,
    
    /// Spare field.
    pub spare: u32,
}

impl HbinHeader {
    /// Parses an hbin header from raw bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes starting at the hbin header.
    /// * `expected_offset` - Expected offset value for validation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Data is too small
    /// - Signature is invalid
    /// - Offset doesn't match expected value
    pub fn parse(data: &[u8], expected_offset: u32) -> Result<Self> {
        if data.len() < HBIN_HEADER_SIZE {
            return Err(RegistryError::TruncatedData {
                offset: expected_offset,
                expected: HBIN_HEADER_SIZE,
                actual: data.len(),
            });
        }

        // Read signature
        let mut signature = [0u8; 4];
        signature.copy_from_slice(&data[0..4]);

        if &signature != HBIN_SIGNATURE {
            return Err(RegistryError::invalid_signature(HBIN_SIGNATURE, &signature));
        }

        let offset = read_u32_le(data, 0x04)?;
        let size = read_u32_le(data, 0x08)?;
        
        // Validate offset
        if offset != expected_offset {
            return Err(RegistryError::InvalidFormat(format!(
                "Hbin offset mismatch: expected {:#x}, found {:#x}",
                expected_offset, offset
            )));
        }

        let reserved = [
            read_u32_le(data, 0x0C)?,
            read_u32_le(data, 0x10)?,
        ];
        
        let timestamp = u64::from(read_u32_le(data, 0x14)?)
            | (u64::from(read_u32_le(data, 0x18)?) << 32);
        
        let spare = read_u32_le(data, 0x1C)?;

        Ok(HbinHeader {
            signature,
            offset,
            size,
            reserved,
            timestamp,
            spare,
        })
    }

    /// Returns the size of the data area (excluding the header).
    pub fn data_size(&self) -> u32 {
        self.size.saturating_sub(HBIN_HEADER_SIZE as u32)
    }
}

/// Iterator over cells within an hbin.
pub struct HbinCellIterator<'a> {
    data: &'a [u8],
    offset: usize,
    hbin_offset: u32,
}

impl<'a> HbinCellIterator<'a> {
    /// Creates a new cell iterator for an hbin's data area.
    ///
    /// # Arguments
    ///
    /// * `data` - The hbin's data area (excluding header).
    /// * `hbin_offset` - The offset of this hbin from the first hbin.
    pub fn new(data: &'a [u8], hbin_offset: u32) -> Self {
        Self {
            data,
            offset: 0,
            hbin_offset,
        }
    }
}

impl<'a> Iterator for HbinCellIterator<'a> {
    type Item = Result<CellInfo<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        // Read cell size (first 4 bytes)
        let size_result = read_u32_le(self.data, self.offset);
        let size = match size_result {
            Ok(s) => s as i32,
            Err(e) => return Some(Err(e)),
        };

        if size == 0 {
            // End of cells
            return None;
        }

        let abs_size = size.unsigned_abs() as usize;
        
        if abs_size < 4 {
            return Some(Err(RegistryError::invalid_cell_size(
                size,
                self.hbin_offset + self.offset as u32,
            )));
        }

        let cell_offset = self.hbin_offset + self.offset as u32;
        let data_start = self.offset + 4;
        let data_end = self.offset + abs_size;

        if data_end > self.data.len() {
            return Some(Err(RegistryError::TruncatedData {
                offset: cell_offset,
                expected: abs_size,
                actual: self.data.len() - self.offset,
            }));
        }

        let cell_data = &self.data[data_start..data_end];
        let is_allocated = size < 0;

        let cell_info = CellInfo {
            offset: cell_offset,
            size: abs_size as u32,
            is_allocated,
            data: cell_data,
        };

        self.offset = data_end;
        Some(Ok(cell_info))
    }
}

/// Information about a cell within an hbin.
#[derive(Debug)]
pub struct CellInfo<'a> {
    /// Offset of this cell from the first hbin.
    pub offset: u32,
    
    /// Size of the cell (including the size field).
    pub size: u32,
    
    /// Whether this cell is allocated (true) or free (false).
    pub is_allocated: bool,
    
    /// Cell data (excluding the size field).
    pub data: &'a [u8],
}

impl<'a> CellInfo<'a> {
    /// Returns the cell type signature (first 2 bytes of data).
    pub fn cell_type(&self) -> Option<[u8; 2]> {
        if self.data.len() >= 2 {
            Some([self.data[0], self.data[1]])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hbin_header_size() {
        assert_eq!(HBIN_HEADER_SIZE, 32);
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = vec![0u8; HBIN_HEADER_SIZE];
        data[0..4].copy_from_slice(b"XXXX");
        
        let result = HbinHeader::parse(&data, 0);
        assert!(result.is_err());
    }
}
