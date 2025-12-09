//! Registry key node (nk) parsing and representation.

use crate::cell::KeyNodeFlags;
use crate::error::{RegistryError, Result};
use crate::utils::{read_ascii_string, read_u16_le, read_u32_le, read_utf16_string};

/// Minimum size of a key node structure in bytes.
const KEY_NODE_MIN_SIZE: usize = 76;

/// Offset of the key name in the key node structure.
const KEY_NAME_OFFSET: usize = 0x4C;

/// Key node (nk) structure.
///
/// Represents a registry key with metadata including name, timestamps,
/// and references to subkeys and values.
#[derive(Debug, Clone)]
pub struct KeyNode {
    /// Flags for this key.
    pub flags: KeyNodeFlags,
    
    /// Last written timestamp (Windows FILETIME).
    pub last_written: u64,
    
    /// Access bits (unused).
    pub access_bits: u32,
    
    /// Offset to parent key node.
    pub parent_offset: u32,
    
    /// Number of subkeys.
    pub subkey_count: u32,
    
    /// Number of volatile subkeys.
    pub volatile_subkey_count: u32,
    
    /// Offset to subkey list.
    pub subkey_list_offset: u32,
    
    /// Offset to volatile subkey list.
    pub volatile_subkey_list_offset: u32,
    
    /// Number of values.
    pub value_count: u32,
    
    /// Offset to value list.
    pub value_list_offset: u32,
    
    /// Offset to security descriptor.
    pub security_offset: u32,
    
    /// Offset to class name.
    pub class_name_offset: u32,
    
    /// Maximum length of subkey name.
    pub max_subkey_name_len: u32,
    
    /// Maximum length of subkey class name.
    pub max_subkey_class_len: u32,
    
    /// Maximum length of value name.
    pub max_value_name_len: u32,
    
    /// Maximum length of value data.
    pub max_value_data_len: u32,
    
    /// Work variable (unused).
    pub work_var: u32,
    
    /// Length of key name.
    pub name_length: u16,
    
    /// Length of class name.
    pub class_name_length: u16,
    
    /// Key name.
    pub name: String,
}

impl KeyNode {
    /// Parses a key node from cell data.
    ///
    /// # Arguments
    ///
    /// * `data` - Cell data (excluding size field, starting with "nk" signature).
    /// * `offset` - Offset of this cell for error reporting.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or truncated.
    pub fn parse(data: &[u8], offset: u32) -> Result<Self> {
        if data.len() < KEY_NODE_MIN_SIZE {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: KEY_NODE_MIN_SIZE,
                actual: data.len(),
            });
        }

        // Verify signature
        if &data[0..2] != b"nk" {
            return Err(RegistryError::InvalidFormat(format!(
                "Expected 'nk' signature at offset {:#x}",
                offset
            )));
        }

        let flags = KeyNodeFlags::new(read_u16_le(data, 0x02)?);
        
        // Last written timestamp at offset 0x04 (8 bytes)
        let last_written = u64::from(read_u32_le(data, 0x04)?)
            | (u64::from(read_u32_le(data, 0x08)?) << 32);
        
        let access_bits = read_u32_le(data, 0x0C)?;
        let parent_offset = read_u32_le(data, 0x10)?;
        let subkey_count = read_u32_le(data, 0x14)?;
        let volatile_subkey_count = read_u32_le(data, 0x18)?;
        let subkey_list_offset = read_u32_le(data, 0x1C)?;
        let volatile_subkey_list_offset = read_u32_le(data, 0x20)?;
        let value_count = read_u32_le(data, 0x24)?;
        let value_list_offset = read_u32_le(data, 0x28)?;
        let security_offset = read_u32_le(data, 0x2C)?;
        let class_name_offset = read_u32_le(data, 0x30)?;
        
        let max_subkey_name_len = read_u32_le(data, 0x34)?;
        let max_subkey_class_len = read_u32_le(data, 0x38)?;
        let max_value_name_len = read_u32_le(data, 0x3C)?;
        let max_value_data_len = read_u32_le(data, 0x40)?;
        let work_var = read_u32_le(data, 0x44)?;
        
        let name_length = read_u16_le(data, 0x48)?;
        let class_name_length = read_u16_le(data, 0x4A)?;
        
        // Key name starts at offset 0x4C
        let name = if name_length > 0 {
            let name_end = 0x4C + name_length as usize;
            if name_end > data.len() {
                return Err(RegistryError::TruncatedData {
                    offset,
                    expected: name_end,
                    actual: data.len(),
                });
            }
            
            let name_data = &data[0x4C..name_end];
            
            if flags.is_compressed() {
                // ASCII name
                read_ascii_string(name_data)
            } else {
                // UTF-16LE name
                read_utf16_string(name_data, offset)?
            }
        } else {
            String::new()
        };

        Ok(KeyNode {
            flags,
            last_written,
            access_bits,
            parent_offset,
            subkey_count,
            volatile_subkey_count,
            subkey_list_offset,
            volatile_subkey_list_offset,
            value_count,
            value_list_offset,
            security_offset,
            class_name_offset,
            max_subkey_name_len,
            max_subkey_class_len,
            max_value_name_len,
            max_value_data_len,
            work_var,
            name_length,
            class_name_length,
            name,
        })
    }

    /// Returns true if this key has subkeys.
    pub fn has_subkeys(&self) -> bool {
        self.subkey_count > 0
    }

    /// Returns true if this key has values.
    pub fn has_values(&self) -> bool {
        self.value_count > 0
    }

    /// Returns true if this is the root key.
    pub fn is_root(&self) -> bool {
        self.flags.is_root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_node_minimum_size() {
        let data = vec![0u8; 75];
        let result = KeyNode::parse(&data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_node_invalid_signature() {
        let mut data = vec![0u8; 80];
        data[0..2].copy_from_slice(b"XX");
        let result = KeyNode::parse(&data, 0);
        assert!(result.is_err());
    }
}
