//! Utility functions for binary parsing and string conversion.

use crate::error::{RegistryError, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use encoding_rs::UTF_16LE;
use std::io::Cursor;

/// Offset where hive bins start (after base block)
pub const HBIN_START_OFFSET: u32 = 0x1000;

/// Reads an ASCII string from a byte slice, trimming null terminators.
/// 
/// Registry strings are often null-terminated, so we trim trailing nulls.
/// Uses lossy conversion to handle any invalid UTF-8 bytes gracefully.
pub fn read_ascii_string(data: &[u8]) -> String {
    String::from_utf8_lossy(data)
        .trim_end_matches('\0')
        .to_string()
}

/// Reads a UTF-16LE string from a byte slice, trimming null terminators.
/// 
/// Registry strings are typically null-terminated. This function decodes
/// UTF-16LE data and removes trailing null characters.
/// 
/// # Errors
/// 
/// Returns an error if the data length is not even (UTF-16 requires 2-byte units)
/// or if the UTF-16 decoding fails.
pub fn read_utf16_string(data: &[u8], offset: u32) -> Result<String> {
    if data.is_empty() {
        return Ok(String::new());
    }

    // UTF-16 requires even number of bytes
    if data.len() % 2 != 0 {
        return Err(RegistryError::InvalidUtf16 { offset });
    }

    let (decoded, _encoding, had_errors) = UTF_16LE.decode(data);
    
    if had_errors {
        return Err(RegistryError::InvalidUtf16 { offset });
    }

    // Trim null terminators (common in registry strings)
    Ok(decoded.trim_end_matches('\0').to_string())
}

/// Reads a fixed-length ASCII string (not null-terminated).
pub fn read_fixed_ascii(data: &[u8], len: usize) -> String {
    data.iter()
        .take(len)
        .map(|&b| if b == 0 { ' ' } else { b as char })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Reads a u32 from a byte slice at the given offset.
pub fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(RegistryError::TruncatedData {
            offset: offset as u32,
            expected: 4,
            actual: data.len().saturating_sub(offset),
        });
    }
    
    let mut cursor = Cursor::new(&data[offset..offset + 4]);
    Ok(cursor.read_u32::<LittleEndian>()?)
}

/// Reads a u16 from a byte slice at the given offset.
pub fn read_u16_le(data: &[u8], offset: usize) -> Result<u16> {
    if offset + 2 > data.len() {
        return Err(RegistryError::TruncatedData {
            offset: offset as u32,
            expected: 2,
            actual: data.len().saturating_sub(offset),
        });
    }
    
    let mut cursor = Cursor::new(&data[offset..offset + 2]);
    Ok(cursor.read_u16::<LittleEndian>()?)
}

/// Reads an i32 from a byte slice at the given offset.
pub fn read_i32_le(data: &[u8], offset: usize) -> Result<i32> {
    if offset + 4 > data.len() {
        return Err(RegistryError::TruncatedData {
            offset: offset as u32,
            expected: 4,
            actual: data.len().saturating_sub(offset),
        });
    }
    
    let mut cursor = Cursor::new(&data[offset..offset + 4]);
    Ok(cursor.read_i32::<LittleEndian>()?)
}

/// Calculates XOR checksum for the first 508 bytes of the base block.
pub fn calculate_checksum(data: &[u8]) -> u32 {
    let mut checksum: u32 = 0;
    
    // XOR all DWORDs except the checksum field itself (at offset 0x1FC)
    for i in (0..0x1FC).step_by(4) {
        if i + 4 <= data.len() {
            if let Ok(dword) = read_u32_le(data, i) {
                checksum ^= dword;
            }
        }
    }
    
    checksum
}

/// Converts a relative cell offset to an absolute hive offset.
/// 
/// Cell offsets in the registry are relative to the first hbin (at 0x1000).
/// This function adds 0x1000 to convert to an absolute offset.
/// 
/// # Arguments
/// 
/// * `cell_offset` - Cell offset relative to first hbin
/// 
/// # Returns
/// 
/// Returns the absolute offset, or an error if the addition would overflow.
/// 
/// # Errors
/// 
/// Returns `RegistryError::InvalidOffset` if the offset would overflow.
#[inline]
pub fn cell_offset_to_absolute(cell_offset: u32) -> Result<u32> {
    cell_offset
        .checked_add(HBIN_START_OFFSET)
        .ok_or_else(|| RegistryError::InvalidOffset {
            offset: cell_offset,
            hive_size: 0,  // Not known at this point
        })
}

/// Converts an absolute hive offset to a relative cell offset.
/// 
/// # Arguments
/// 
/// * `absolute_offset` - Absolute offset from start of hive
/// 
/// # Returns
/// 
/// Returns the cell offset relative to first hbin, or an error if the
/// absolute offset is before the hbin start.
/// 
/// # Errors
/// 
/// Returns `RegistryError::InvalidFormat` if the offset is before hbin start.
#[inline]
pub fn absolute_to_cell_offset(absolute_offset: u32) -> Result<u32> {
    if absolute_offset < HBIN_START_OFFSET {
        return Err(RegistryError::InvalidFormat(
            format!("Absolute offset {:#x} is before hbin start", absolute_offset)
        ));
    }
    Ok(absolute_offset - HBIN_START_OFFSET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_ascii_string() {
        let data = b"Hello";
        assert_eq!(read_ascii_string(data), "Hello");
        
        // Should trim trailing nulls
        let data_with_null = b"Hello\0\0";
        assert_eq!(read_ascii_string(data_with_null), "Hello");
    }

    #[test]
    fn test_read_ascii_string_with_embedded_null() {
        // Embedded nulls are preserved, only trailing ones are trimmed
        let data = b"Hello\0World\0\0";
        assert_eq!(read_ascii_string(data), "Hello\0World");
    }

    #[test]
    fn test_read_fixed_ascii() {
        let data = b"Test    ";
        assert_eq!(read_fixed_ascii(data, 8), "Test");
    }

    #[test]
    fn test_offset_conversion() {
        assert_eq!(cell_offset_to_absolute(0).unwrap(), 0x1000);
        assert_eq!(cell_offset_to_absolute(0x20).unwrap(), 0x1020);
        assert_eq!(cell_offset_to_absolute(0x1000).unwrap(), 0x2000);
        
        assert_eq!(absolute_to_cell_offset(0x1000).unwrap(), 0);
        assert_eq!(absolute_to_cell_offset(0x1020).unwrap(), 0x20);
        assert_eq!(absolute_to_cell_offset(0x2000).unwrap(), 0x1000);
    }
    
    #[test]
    fn test_offset_overflow() {
        // Test overflow protection
        let result = cell_offset_to_absolute(u32::MAX);
        assert!(result.is_err());
        
        let result = cell_offset_to_absolute(u32::MAX - HBIN_START_OFFSET + 1);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_offset_underflow() {
        // Test underflow protection
        let result = absolute_to_cell_offset(0);
        assert!(result.is_err());
        
        let result = absolute_to_cell_offset(0xFFF);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_u32_le() {
        let data = [0x01, 0x02, 0x03, 0x04];
        assert_eq!(read_u32_le(&data, 0).unwrap(), 0x04030201);
    }
}
