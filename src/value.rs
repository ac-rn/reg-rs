//! Registry value (vk) parsing and data extraction.

use crate::cell::ValueType;
use crate::error::{RegistryError, Result};
use crate::utils::{read_ascii_string, read_i32_le, read_u16_le, read_u32_le, read_utf16_string};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

/// Value key (vk) structure.
///
/// Represents a registry value with its name, type, and data.
#[derive(Debug, Clone)]
pub struct ValueKey {
    /// Length of value name.
    pub name_length: u16,
    
    /// Length of value data.
    pub data_length: u32,
    
    /// Offset to value data (or inline data if length <= 4).
    pub data_offset: u32,
    
    /// Value data type.
    pub data_type: ValueType,
    
    /// Flags (0x0001 = name is ASCII).
    pub flags: u16,
    
    /// Value name.
    pub name: String,
}

impl ValueKey {
    /// Parses a value key from cell data.
    ///
    /// # Arguments
    ///
    /// * `data` - Cell data (excluding size field, starting with "vk" signature).
    /// * `offset` - Offset of this cell for error reporting.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or truncated.
    pub fn parse(data: &[u8], offset: u32) -> Result<Self> {
        if data.len() < 20 {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: 20,
                actual: data.len(),
            });
        }

        // Verify signature
        if &data[0..2] != b"vk" {
            return Err(RegistryError::InvalidFormat(format!(
                "Expected 'vk' signature at offset {:#x}",
                offset
            )));
        }

        let name_length = read_u16_le(data, 0x02)?;
        
        // Data length is stored as i32, with high bit indicating inline data
        let data_length_raw = read_i32_le(data, 0x04)?;
        let data_length = (data_length_raw & 0x7FFFFFFF) as u32;
        
        let data_offset = read_u32_le(data, 0x08)?;
        let data_type_raw = read_u32_le(data, 0x0C)?;
        let data_type = ValueType::from_u32(data_type_raw)?;
        let flags = read_u16_le(data, 0x10)?;
        
        // Spare field at 0x12 (2 bytes) - unused
        
        // Value name starts at offset 0x14
        let name = if name_length > 0 {
            let name_end = 0x14 + name_length as usize;
            if name_end > data.len() {
                return Err(RegistryError::TruncatedData {
                    offset,
                    expected: name_end,
                    actual: data.len(),
                });
            }
            
            let name_data = &data[0x14..name_end];
            
            // Check if name is ASCII (flag 0x0001)
            if (flags & 0x0001) != 0 {
                read_ascii_string(name_data)
            } else {
                read_utf16_string(name_data, offset)?
            }
        } else {
            // Default value (unnamed) - use lowercase to match regipy convention
            String::from("(default)")
        };

        Ok(ValueKey {
            name_length,
            data_length,
            data_offset,
            data_type,
            flags,
            name,
        })
    }

    /// Returns true if the data is stored inline (in the data_offset field).
    pub fn is_inline_data(&self) -> bool {
        self.data_length <= 4 && self.data_length > 0
    }

    /// Extracts inline data (when data_length <= 4).
    pub fn inline_data(&self) -> Vec<u8> {
        let bytes = self.data_offset.to_le_bytes();
        bytes[..self.data_length as usize].to_vec()
    }
}

/// Parsed registry value data.
#[derive(Debug, Clone)]
pub enum ValueData {
    /// No data.
    None,
    
    /// String value.
    String(String),
    
    /// Expandable string value.
    ExpandString(String),
    
    /// Binary data.
    Binary(Vec<u8>),
    
    /// 32-bit integer.
    Dword(u32),
    
    /// 32-bit big-endian integer.
    DwordBigEndian(u32),
    
    /// Multiple strings.
    MultiString(Vec<String>),
    
    /// 64-bit integer.
    Qword(u64),
    
    /// Unknown or unsupported type.
    Unknown(Vec<u8>),
}

impl ValueData {
    /// Parses value data based on the value type.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw value data bytes.
    /// * `value_type` - Type of the value.
    /// * `offset` - Offset for error reporting.
    pub fn parse(data: &[u8], value_type: ValueType, offset: u32) -> Result<Self> {
        if data.is_empty() {
            return Ok(ValueData::None);
        }

        match value_type {
            ValueType::None => Ok(ValueData::None),
            
            ValueType::String | ValueType::ExpandString => {
                let s = read_utf16_string(data, offset)?;
                if value_type == ValueType::String {
                    Ok(ValueData::String(s))
                } else {
                    Ok(ValueData::ExpandString(s))
                }
            }
            
            ValueType::Binary => Ok(ValueData::Binary(data.to_vec())),
            
            ValueType::Dword => {
                if data.len() < 4 {
                    return Err(RegistryError::TruncatedData {
                        offset,
                        expected: 4,
                        actual: data.len(),
                    });
                }
                let mut cursor = Cursor::new(data);
                let value = cursor.read_u32::<LittleEndian>()?;
                Ok(ValueData::Dword(value))
            }
            
            ValueType::DwordBigEndian => {
                if data.len() < 4 {
                    return Err(RegistryError::TruncatedData {
                        offset,
                        expected: 4,
                        actual: data.len(),
                    });
                }
                let mut cursor = Cursor::new(data);
                let value = cursor.read_u32::<BigEndian>()?;
                Ok(ValueData::DwordBigEndian(value))
            }
            
            ValueType::Qword => {
                if data.len() < 8 {
                    return Err(RegistryError::TruncatedData {
                        offset,
                        expected: 8,
                        actual: data.len(),
                    });
                }
                let mut cursor = Cursor::new(data);
                let value = cursor.read_u64::<LittleEndian>()?;
                Ok(ValueData::Qword(value))
            }
            
            ValueType::MultiString => {
                let full_string = read_utf16_string(data, offset)?;
                let strings: Vec<String> = full_string
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
                Ok(ValueData::MultiString(strings))
            }
            
            // For other types, return raw binary data
            _ => Ok(ValueData::Unknown(data.to_vec())),
        }
    }

    /// Converts the value data to a string representation.
    pub fn to_string(&self) -> String {
        match self {
            ValueData::None => String::from("(none)"),
            ValueData::String(s) | ValueData::ExpandString(s) => s.clone(),
            ValueData::Binary(b) => format!("{:02X?}", b),
            ValueData::Dword(d) => format!("{} (0x{:08X})", d, d),
            ValueData::DwordBigEndian(d) => format!("{} (0x{:08X})", d, d),
            ValueData::Qword(q) => format!("{} (0x{:016X})", q, q),
            ValueData::MultiString(strings) => strings.join(", "),
            ValueData::Unknown(b) => format!("{:02X?}", b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_key_minimum_size() {
        let data = vec![0u8; 19];
        let result = ValueKey::parse(&data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_value_key_invalid_signature() {
        let mut data = vec![0u8; 24];
        data[0..2].copy_from_slice(b"XX");
        let result = ValueKey::parse(&data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_inline_data() {
        let mut data = vec![0u8; 24];
        data[0..2].copy_from_slice(b"vk");
        // Set data length to 4 (inline)
        data[4] = 4;
        // Set data_offset to some value
        data[8..12].copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
        
        let vk = ValueKey::parse(&data, 0).unwrap();
        assert!(vk.is_inline_data());
        assert_eq!(vk.inline_data(), vec![0x01, 0x02, 0x03, 0x04]);
    }
}
