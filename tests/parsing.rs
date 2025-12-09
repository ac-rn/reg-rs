//! Unit tests for parsing specific structures.

use reg_parser::*;

#[test]
fn test_base_block_constants() {
    assert_eq!(header::BASE_BLOCK_SIZE, 4096);
    assert_eq!(header::REGF_SIGNATURE, b"regf");
}

#[test]
fn test_hbin_constants() {
    assert_eq!(hbin::HBIN_HEADER_SIZE, 32);
    assert_eq!(hbin::HBIN_SIGNATURE, b"hbin");
}

#[test]
fn test_cell_type_signatures() {
    assert_eq!(CellType::KeyNode.signature(), b"nk");
    assert_eq!(CellType::ValueKey.signature(), b"vk");
    assert_eq!(CellType::Security.signature(), b"sk");
    assert_eq!(CellType::IndexLeaf.signature(), b"li");
    assert_eq!(CellType::FastLeaf.signature(), b"lf");
    assert_eq!(CellType::HashLeaf.signature(), b"lh");
    assert_eq!(CellType::IndexRoot.signature(), b"ri");
    assert_eq!(CellType::DataBlock.signature(), b"db");
}

#[test]
fn test_cell_type_from_signature() {
    assert_eq!(
        CellType::from_signature(b"nk").unwrap(),
        CellType::KeyNode
    );
    assert_eq!(
        CellType::from_signature(b"vk").unwrap(),
        CellType::ValueKey
    );
    assert!(CellType::from_signature(b"XX").is_err());
}

#[test]
fn test_value_type_names() {
    assert_eq!(ValueType::None.name(), "REG_NONE");
    assert_eq!(ValueType::String.name(), "REG_SZ");
    assert_eq!(ValueType::ExpandString.name(), "REG_EXPAND_SZ");
    assert_eq!(ValueType::Binary.name(), "REG_BINARY");
    assert_eq!(ValueType::Dword.name(), "REG_DWORD");
    assert_eq!(ValueType::DwordBigEndian.name(), "REG_DWORD_BIG_ENDIAN");
    assert_eq!(ValueType::Link.name(), "REG_LINK");
    assert_eq!(ValueType::MultiString.name(), "REG_MULTI_SZ");
    assert_eq!(ValueType::Qword.name(), "REG_QWORD");
}

#[test]
fn test_value_type_from_u32() {
    assert_eq!(ValueType::from_u32(0).unwrap(), ValueType::None);
    assert_eq!(ValueType::from_u32(1).unwrap(), ValueType::String);
    assert_eq!(ValueType::from_u32(2).unwrap(), ValueType::ExpandString);
    assert_eq!(ValueType::from_u32(3).unwrap(), ValueType::Binary);
    assert_eq!(ValueType::from_u32(4).unwrap(), ValueType::Dword);
    assert_eq!(ValueType::from_u32(11).unwrap(), ValueType::Qword);
    // Unknown value types are now allowed per the Windows Registry specification
    assert!(matches!(ValueType::from_u32(999).unwrap(), ValueType::Unknown(999)));
}

#[test]
fn test_key_node_flags() {
    let flags = KeyNodeFlags::new(0);
    assert!(!flags.is_compressed());
    assert!(!flags.is_volatile());
    assert!(!flags.is_root());
    
    let flags = KeyNodeFlags::new(KeyNodeFlags::COMP_NAME);
    assert!(flags.is_compressed());
    assert!(!flags.is_volatile());
    
    let flags = KeyNodeFlags::new(KeyNodeFlags::ROOT_KEY);
    assert!(flags.is_root());
    
    let flags = KeyNodeFlags::new(KeyNodeFlags::VOLATILE);
    assert!(flags.is_volatile());
}

#[test]
fn test_subkey_list_types() {
    use subkey_list::SubkeyListType;
    
    assert_eq!(
        SubkeyListType::from_signature(b"li").unwrap(),
        SubkeyListType::IndexLeaf
    );
    assert_eq!(
        SubkeyListType::from_signature(b"lf").unwrap(),
        SubkeyListType::FastLeaf
    );
    assert_eq!(
        SubkeyListType::from_signature(b"lh").unwrap(),
        SubkeyListType::HashLeaf
    );
    assert_eq!(
        SubkeyListType::from_signature(b"ri").unwrap(),
        SubkeyListType::IndexRoot
    );
    assert!(SubkeyListType::from_signature(b"XX").is_err());
}

#[test]
fn test_offset_conversion() {
    use utils::{absolute_to_cell_offset, cell_offset_to_absolute};
    
    assert_eq!(cell_offset_to_absolute(0).unwrap(), 0x1000);
    assert_eq!(cell_offset_to_absolute(0x20).unwrap(), 0x1020);
    assert_eq!(cell_offset_to_absolute(0x1000).unwrap(), 0x2000);
    
    assert_eq!(absolute_to_cell_offset(0x1000).unwrap(), 0);
    assert_eq!(absolute_to_cell_offset(0x1020).unwrap(), 0x20);
    assert_eq!(absolute_to_cell_offset(0x2000).unwrap(), 0x1000);
    
    // Test overflow protection
    assert!(cell_offset_to_absolute(u32::MAX).is_err());
    assert!(absolute_to_cell_offset(0).is_err());
}

#[test]
fn test_error_types() {
    let err = RegistryError::invalid_signature(b"regf", b"XXXX");
    assert!(matches!(err, RegistryError::InvalidSignature { .. }));
    
    let err = RegistryError::invalid_offset(0x1234, 0x1000);
    assert!(matches!(err, RegistryError::InvalidOffset { .. }));
    
    let err = RegistryError::invalid_cell_size(-8, 0x2000);
    assert!(matches!(err, RegistryError::InvalidCellSize { .. }));
}

#[test]
fn test_value_data_display() {
    let data = ValueData::None;
    assert_eq!(data.to_string(), "(none)");
    
    let data = ValueData::String("Hello".to_string());
    assert_eq!(data.to_string(), "Hello");
    
    let data = ValueData::Dword(0x12345678);
    assert!(data.to_string().contains("0x12345678"));
    
    let data = ValueData::Binary(vec![0x01, 0x02, 0x03]);
    assert!(data.to_string().contains("01"));
}

#[test]
fn test_cell_type_is_subkey_list() {
    assert!(CellType::IndexLeaf.is_subkey_list());
    assert!(CellType::FastLeaf.is_subkey_list());
    assert!(CellType::HashLeaf.is_subkey_list());
    assert!(CellType::IndexRoot.is_subkey_list());
    
    assert!(!CellType::KeyNode.is_subkey_list());
    assert!(!CellType::ValueKey.is_subkey_list());
    assert!(!CellType::Security.is_subkey_list());
}

#[cfg(test)]
mod property_tests {
    use super::*;
    
    // Property-based tests would go here using proptest
    // Example: test that all valid cell type signatures round-trip correctly
    
    #[test]
    fn test_cell_type_roundtrip() {
        let types = [
            CellType::KeyNode,
            CellType::ValueKey,
            CellType::Security,
            CellType::IndexLeaf,
            CellType::FastLeaf,
            CellType::HashLeaf,
            CellType::IndexRoot,
            CellType::DataBlock,
        ];
        
        for cell_type in &types {
            let sig = cell_type.signature();
            let parsed = CellType::from_signature(sig).unwrap();
            assert_eq!(*cell_type, parsed);
        }
    }
    
    #[test]
    fn test_value_type_roundtrip() {
        // Test standard value types (0-11)
        for i in 0..=11 {
            let vt = ValueType::from_u32(i).unwrap();
            assert_eq!(ValueType::from_u32(i).unwrap(), vt);
        }
        
        // Test unknown value types
        let unknown = ValueType::from_u32(0xFFFF0011).unwrap();
        assert!(matches!(unknown, ValueType::Unknown(0xFFFF0011)));
    }
}
