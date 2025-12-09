//! Integration tests using real registry hive files.

use reg_parser::{Hive, ValueData};
use std::path::PathBuf;

fn test_data_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(filename)
}

#[test]
fn test_open_system_hive() {
    let path = test_data_path("SYSTEM");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open SYSTEM hive: {:?}", result.err());
    
    let hive = result.unwrap();
    let base_block = hive.base_block();
    
    // Verify signature
    assert_eq!(&base_block.signature, b"regf");
    
    // Verify version (should be 1.x)
    assert_eq!(base_block.major_version, 1);
    assert!(base_block.minor_version >= 3 && base_block.minor_version <= 6);
    
    println!("SYSTEM hive:");
    println!("  Version: {}.{}", base_block.major_version, base_block.minor_version);
    println!("  Root offset: {:#x}", base_block.root_cell_offset);
    println!("  Hive length: {} bytes", base_block.hive_length);
}

#[test]
fn test_open_software_hive() {
    let path = test_data_path("SOFTWARE");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open SOFTWARE hive: {:?}", result.err());
}

#[test]
fn test_open_sam_hive() {
    let path = test_data_path("SAM");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open SAM hive: {:?}", result.err());
}

#[test]
fn test_open_security_hive() {
    let path = test_data_path("SECURITY");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open SECURITY hive: {:?}", result.err());
}

#[test]
fn test_open_ntuser_dat() {
    let path = test_data_path("NTUSER.DAT");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open NTUSER.DAT: {:?}", result.err());
}

#[test]
fn test_open_usrclass_dat() {
    let path = test_data_path("UsrClass.dat");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open UsrClass.dat: {:?}", result.err());
}

#[test]
fn test_open_amcache() {
    let path = test_data_path("Amcache.hve");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open Amcache.hve: {:?}", result.err());
}

#[test]
fn test_open_default_hive() {
    let path = test_data_path("DEFAULT");
    let result = Hive::open(&path);
    
    assert!(result.is_ok(), "Failed to open DEFAULT hive: {:?}", result.err());
}

#[test]
fn test_root_key_system() {
    let path = test_data_path("SYSTEM");
    let hive = Hive::open(&path).expect("Failed to open SYSTEM hive");
    
    let root = hive.root_key();
    assert!(root.is_ok(), "Failed to get root key: {:?}", root.err());
    
    let root = root.unwrap();
    let name = root.name();
    assert!(name.is_ok(), "Failed to get root key name: {:?}", name.err());
    
    println!("Root key name: {}", name.unwrap());
}

#[test]
fn test_enumerate_subkeys_system() {
    let path = test_data_path("SYSTEM");
    let hive = Hive::open(&path).expect("Failed to open SYSTEM hive");
    
    let root = hive.root_key().expect("Failed to get root key");
    let subkeys = root.subkeys();
    
    assert!(subkeys.is_ok(), "Failed to enumerate subkeys: {:?}", subkeys.err());
    
    let subkeys = subkeys.unwrap();
    assert!(!subkeys.is_empty(), "Root key should have subkeys");
    
    println!("SYSTEM root subkeys ({} total):", subkeys.len());
    for subkey in subkeys.iter().take(10) {
        if let Ok(name) = subkey.name() {
            println!("  - {}", name);
        }
    }
}

#[test]
fn test_enumerate_values_system() {
    let path = test_data_path("SYSTEM");
    let hive = Hive::open(&path).expect("Failed to open SYSTEM hive");
    
    let root = hive.root_key().expect("Failed to get root key");
    
    // Try to find a key with values
    let subkeys = root.subkeys().expect("Failed to get subkeys");
    
    for subkey in subkeys {
        let values = subkey.values();
        if let Ok(values) = values {
            if !values.is_empty() {
                println!("Key '{}' has {} values:", subkey.name().unwrap_or_else(|_| "?".to_string()), values.len());
                
                for value in values.iter().take(5) {
                    println!("  - {} ({:?})", value.name(), value.data_type());
                    
                    if let Ok(data) = value.data() {
                        let data_str = data.to_string();
                        if data_str.len() < 100 {
                            println!("    = {}", data_str);
                        }
                    }
                }
                
                break;
            }
        }
    }
}

#[test]
fn test_deep_traversal_system() {
    let path = test_data_path("SYSTEM");
    let hive = Hive::open(&path).expect("Failed to open SYSTEM hive");
    
    let root = hive.root_key().expect("Failed to get root key");
    
    // Count total keys and values
    let mut key_count = 0;
    let mut value_count = 0;
    
    fn count_recursive(
        key: &reg_parser::RegistryKey,
        key_count: &mut usize,
        value_count: &mut usize,
        depth: usize,
    ) {
        if depth > 10 {
            return; // Limit recursion depth for testing
        }
        
        *key_count += 1;
        
        if let Ok(values) = key.values() {
            *value_count += values.len();
        }
        
        if let Ok(subkeys) = key.subkeys() {
            for subkey in subkeys {
                count_recursive(&subkey, key_count, value_count, depth + 1);
            }
        }
    }
    
    count_recursive(&root, &mut key_count, &mut value_count, 0);
    
    println!("SYSTEM hive statistics:");
    println!("  Keys: {}", key_count);
    println!("  Values: {}", value_count);
    
    assert!(key_count > 0, "Should have found some keys");
}

#[test]
fn test_value_types() {
    let path = test_data_path("SOFTWARE");
    let hive = Hive::open(&path).expect("Failed to open SOFTWARE hive");
    
    let root = hive.root_key().expect("Failed to get root key");
    
    // Search for different value types
    fn find_value_types(key: &reg_parser::RegistryKey, depth: usize) {
        if depth > 5 {
            return;
        }
        
        if let Ok(values) = key.values() {
            for value in values {
                if let Ok(data) = value.data() {
                    match data {
                        ValueData::String(s) => {
                            if !s.is_empty() {
                                println!("Found REG_SZ: {} = {}", value.name(), s);
                                return;
                            }
                        }
                        ValueData::Dword(d) => {
                            println!("Found REG_DWORD: {} = {:#x}", value.name(), d);
                            return;
                        }
                        ValueData::Binary(b) => {
                            if !b.is_empty() {
                                println!("Found REG_BINARY: {} ({} bytes)", value.name(), b.len());
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if let Ok(subkeys) = key.subkeys() {
            for subkey in subkeys.into_iter().take(3) {
                find_value_types(&subkey, depth + 1);
            }
        }
    }
    
    find_value_types(&root, 0);
}

#[test]
fn test_hbin_iteration() {
    let path = test_data_path("SYSTEM");
    let hive = Hive::open(&path).expect("Failed to open SYSTEM hive");
    
    let mut hbin_count = 0;
    let mut total_size = 0;
    
    for hbin_result in hive.hbins() {
        assert!(hbin_result.is_ok(), "Failed to parse hbin: {:?}", hbin_result.err());
        
        let hbin = hbin_result.unwrap();
        hbin_count += 1;
        total_size += hbin.size;
        
        if hbin_count <= 3 {
            println!("Hbin #{}: offset={:#x}, size={:#x}", hbin_count, hbin.offset, hbin.size);
        }
    }
    
    println!("Total hbins: {}", hbin_count);
    println!("Total hbin size: {} bytes", total_size);
    
    assert!(hbin_count > 0, "Should have found at least one hbin");
}

#[test]
fn test_all_hives_open() {
    let hive_files = [
        "SYSTEM",
        "SOFTWARE",
        "SAM",
        "SECURITY",
        "DEFAULT",
        "NTUSER.DAT",
        "UsrClass.dat",
        "Amcache.hve",
        "SYSTEM_2",
    ];
    
    for filename in &hive_files {
        let path = test_data_path(filename);
        if path.exists() {
            let result = Hive::open(&path);
            assert!(
                result.is_ok(),
                "Failed to open {}: {:?}",
                filename,
                result.err()
            );
            println!("✓ Successfully opened {}", filename);
        }
    }
}
