//! Registry cell type definitions and parsing.
//!
//! Cells are the fundamental data structures within registry hives. Each cell
//! has a 2-byte signature that identifies its type.

use crate::error::{RegistryError, Result};

/// Cell type signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    /// Key node (nk) - represents a registry key.
    KeyNode,
    
    /// Value key (vk) - represents a registry value.
    ValueKey,
    
    /// Security descriptor (sk).
    Security,
    
    /// Index leaf (li) - list of subkey offsets.
    IndexLeaf,
    
    /// Fast leaf (lf) - list of subkeys with name hints.
    FastLeaf,
    
    /// Hash leaf (lh) - list of subkeys with name hashes.
    HashLeaf,
    
    /// Index root (ri) - list of subkey list offsets.
    IndexRoot,
    
    /// Data block (db) - big data block.
    DataBlock,
}

impl CellType {
    /// Parses a cell type from a 2-byte signature.
    pub fn from_signature(sig: &[u8; 2]) -> Result<Self> {
        match sig {
            b"nk" => Ok(CellType::KeyNode),
            b"vk" => Ok(CellType::ValueKey),
            b"sk" => Ok(CellType::Security),
            b"li" => Ok(CellType::IndexLeaf),
            b"lf" => Ok(CellType::FastLeaf),
            b"lh" => Ok(CellType::HashLeaf),
            b"ri" => Ok(CellType::IndexRoot),
            b"db" => Ok(CellType::DataBlock),
            _ => Err(RegistryError::unknown_cell_type(*sig, 0)),
        }
    }

    /// Returns the 2-byte signature for this cell type.
    pub fn signature(&self) -> &'static [u8; 2] {
        match self {
            CellType::KeyNode => b"nk",
            CellType::ValueKey => b"vk",
            CellType::Security => b"sk",
            CellType::IndexLeaf => b"li",
            CellType::FastLeaf => b"lf",
            CellType::HashLeaf => b"lh",
            CellType::IndexRoot => b"ri",
            CellType::DataBlock => b"db",
        }
    }

    /// Returns true if this cell type represents a subkey list.
    pub fn is_subkey_list(&self) -> bool {
        matches!(
            self,
            CellType::IndexLeaf | CellType::FastLeaf | CellType::HashLeaf | CellType::IndexRoot
        )
    }
}

/// Flags for key nodes.
#[derive(Debug, Clone, Copy)]
pub struct KeyNodeFlags(pub u16);

impl KeyNodeFlags {
    /// Key is volatile (not stored on disk).
    pub const VOLATILE: u16 = 0x0001;
    
    /// Key is a mount point for another hive.
    pub const HIVE_EXIT: u16 = 0x0002;
    
    /// Key is the root key.
    pub const ROOT_KEY: u16 = 0x0004;
    
    /// Key cannot be deleted.
    pub const NO_DELETE: u16 = 0x0008;
    
    /// Key is a symbolic link.
    pub const SYM_LINK: u16 = 0x0010;
    
    /// Key name is in compressed format (ASCII).
    pub const COMP_NAME: u16 = 0x0020;
    
    /// Key is a predefined handle.
    pub const PREDEF_HANDLE: u16 = 0x0040;
    
    /// Key is part of a virtual store.
    pub const VIRT_SOURCE: u16 = 0x0080;
    
    /// Key is a virtual target.
    pub const VIRT_TARGET: u16 = 0x0100;
    
    /// Key is part of a virtual store.
    pub const VIRT_STORE: u16 = 0x0200;

    /// Creates a new KeyNodeFlags from a u16 value.
    pub fn new(flags: u16) -> Self {
        Self(flags)
    }

    /// Returns true if the specified flag is set.
    pub fn has_flag(&self, flag: u16) -> bool {
        (self.0 & flag) != 0
    }

    /// Returns true if the key name is compressed (ASCII).
    pub fn is_compressed(&self) -> bool {
        self.has_flag(Self::COMP_NAME)
    }

    /// Returns true if this is a volatile key.
    pub fn is_volatile(&self) -> bool {
        self.has_flag(Self::VOLATILE)
    }

    /// Returns true if this is the root key.
    pub fn is_root(&self) -> bool {
        self.has_flag(Self::ROOT_KEY)
    }
}

/// Registry value data types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// No value type.
    None,
    
    /// String (null-terminated).
    String,
    
    /// String with environment variables.
    ExpandString,
    
    /// Binary data.
    Binary,
    
    /// 32-bit little-endian integer.
    Dword,
    
    /// 32-bit big-endian integer.
    DwordBigEndian,
    
    /// Symbolic link (Unicode).
    Link,
    
    /// Multiple strings.
    MultiString,
    
    /// Resource list.
    ResourceList,
    
    /// Full resource descriptor.
    FullResourceDescriptor,
    
    /// Resource requirements list.
    ResourceRequirementsList,
    
    /// 64-bit little-endian integer.
    Qword,
    
    /// Unknown or non-standard value type.
    /// Contains the raw type value.
    Unknown(u32),
}

impl ValueType {
    /// Parses a value type from a u32.
    /// 
    /// According to the Windows Registry specification, value types 0-11 are predefined,
    /// but other values are allowed as well. Unknown types are returned as `ValueType::Unknown`.
    pub fn from_u32(value: u32) -> Result<Self> {
        match value {
            0 => Ok(ValueType::None),
            1 => Ok(ValueType::String),
            2 => Ok(ValueType::ExpandString),
            3 => Ok(ValueType::Binary),
            4 => Ok(ValueType::Dword),
            5 => Ok(ValueType::DwordBigEndian),
            6 => Ok(ValueType::Link),
            7 => Ok(ValueType::MultiString),
            8 => Ok(ValueType::ResourceList),
            9 => Ok(ValueType::FullResourceDescriptor),
            10 => Ok(ValueType::ResourceRequirementsList),
            11 => Ok(ValueType::Qword),
            _ => Ok(ValueType::Unknown(value)),
        }
    }

    /// Returns the name of this value type.
    pub fn name(&self) -> String {
        match self {
            ValueType::None => "REG_NONE".to_string(),
            ValueType::String => "REG_SZ".to_string(),
            ValueType::ExpandString => "REG_EXPAND_SZ".to_string(),
            ValueType::Binary => "REG_BINARY".to_string(),
            ValueType::Dword => "REG_DWORD".to_string(),
            ValueType::DwordBigEndian => "REG_DWORD_BIG_ENDIAN".to_string(),
            ValueType::Link => "REG_LINK".to_string(),
            ValueType::MultiString => "REG_MULTI_SZ".to_string(),
            ValueType::ResourceList => "REG_RESOURCE_LIST".to_string(),
            ValueType::FullResourceDescriptor => "REG_FULL_RESOURCE_DESCRIPTOR".to_string(),
            ValueType::ResourceRequirementsList => "REG_RESOURCE_REQUIREMENTS_LIST".to_string(),
            ValueType::Qword => "REG_QWORD".to_string(),
            ValueType::Unknown(value) => format!("REG_UNKNOWN_{:#010x}", value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_type_from_signature() {
        assert_eq!(CellType::from_signature(b"nk").unwrap(), CellType::KeyNode);
        assert_eq!(CellType::from_signature(b"vk").unwrap(), CellType::ValueKey);
        assert_eq!(CellType::from_signature(b"lf").unwrap(), CellType::FastLeaf);
    }

    #[test]
    fn test_cell_type_signature() {
        assert_eq!(CellType::KeyNode.signature(), b"nk");
        assert_eq!(CellType::ValueKey.signature(), b"vk");
    }

    #[test]
    fn test_key_node_flags() {
        let flags = KeyNodeFlags::new(KeyNodeFlags::COMP_NAME | KeyNodeFlags::ROOT_KEY);
        assert!(flags.is_compressed());
        assert!(flags.is_root());
        assert!(!flags.is_volatile());
    }

    #[test]
    fn test_value_type() {
        assert_eq!(ValueType::from_u32(1).unwrap(), ValueType::String);
        assert_eq!(ValueType::from_u32(4).unwrap(), ValueType::Dword);
        assert_eq!(ValueType::String.name(), "REG_SZ");
    }
}
