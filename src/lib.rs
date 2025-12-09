//! # Windows Registry Hive Parser
//!
//! A high-performance, zero-copy Windows registry hive parser written in Rust.
//!
//! ## Features
//!
//! - **Fast parsing**: Memory-mapped I/O for efficient, zero-copy access
//! - **Complete support**: Handles all common registry hive formats (SYSTEM, SOFTWARE, SAM, SECURITY, NTUSER.DAT, etc.)
//! - **Type-safe**: Strong typing for registry values and structures
//! - **Lazy evaluation**: Parses structures only when accessed
//! - **Comprehensive error handling**: Detailed error types for debugging
//!
//! ## Architecture
//!
//! The parser is built on several layers:
//!
//! 1. **Base Block (Header)**: Contains hive metadata and root key offset
//! 2. **Hive Bins (hbin)**: 4KB-aligned blocks containing cells
//! 3. **Cells**: Variable-sized structures (keys, values, lists, etc.)
//! 4. **Key Nodes (nk)**: Registry keys with subkeys and values
//! 5. **Value Keys (vk)**: Registry values with typed data
//! 6. **Subkey Lists (lf/lh/li/ri)**: Efficient subkey organization
//!
//! ## Binary Layout
//!
//! Registry hives follow this structure:
//!
//! ```text
//! [Base Block - 4KB]
//!   - Signature: "regf"
//!   - Version, timestamps, root offset
//!   - Checksum
//!
//! [Hive Bins - variable size, 4KB aligned]
//!   [Hbin Header - 32 bytes]
//!     - Signature: "hbin"
//!     - Offset, size
//!   
//!   [Cells - variable size]
//!     [Cell Size - 4 bytes, negative if allocated]
//!     [Cell Data]
//!       - Key nodes (nk)
//!       - Value keys (vk)
//!       - Subkey lists (lf/lh/li/ri)
//!       - Security descriptors (sk)
//! ```
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```no_run
//! use reg_parser::Hive;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open a registry hive
//! let mut hive = Hive::open("SYSTEM")?;
//!
//! // Get the root key
//! let mut root = hive.root_key()?;
//! println!("Root key: {}", root.name()?);
//!
//! // Enumerate subkeys
//! for mut subkey in root.subkeys()? {
//!     println!("  Subkey: {}", subkey.name()?);
//! }
//!
//! // Enumerate values
//! for value in root.values()? {
//!     println!("  Value: {} = {}", value.name(), value.data()?.to_string());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Accessing Specific Values
//!
//! ```no_run
//! use reg_parser::{Hive, ValueData};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut hive = Hive::open("SOFTWARE")?;
//! let mut root = hive.root_key()?;
//!
//! // Navigate to a specific key
//! let mut subkeys = root.subkeys()?;
//! let mut microsoft_key = subkeys.into_iter()
//!     .find(|k| k.name().map(|n| n == "Microsoft").unwrap_or(false))
//!     .ok_or("Microsoft key not found")?;
//!
//! // Get a specific value
//! let value = microsoft_key.value("SomeValue")?;
//! match value.data()? {
//!     ValueData::String(s) => println!("String value: {}", s),
//!     ValueData::Dword(d) => println!("DWORD value: {}", d),
//!     _ => println!("Other type"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Supported Features
//!
//! - Full registry hive parsing (SYSTEM, SOFTWARE, SAM, SECURITY, NTUSER.DAT, etc.)
//! - All common value types (REG_SZ, REG_DWORD, REG_BINARY, REG_MULTI_SZ, REG_QWORD, etc.)
//! - Subkey enumeration with efficient list structures (lf/lh/li/ri)
//! - Transaction log (.LOG1, .LOG2) support for recovering uncommitted changes
//! - **Big data block (db) support for values > 16KB** 
//!
//! ## Planned Features
//!
//! - Write support (currently read-only)
//! - Security descriptor parsing
//! - Class name extraction

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod bigdata;
pub mod cell;
pub mod error;
pub mod hbin;
pub mod header;
pub mod hive;
pub mod key;
pub mod subkey_list;
pub mod transaction_log;
pub mod utils;
pub mod value;

// Python bindings (only compiled when python feature is enabled)
#[cfg(feature = "python")]
pub mod python;

// Re-export main types for convenience
pub use cell::{CellType, KeyNodeFlags, ValueType};
pub use error::{RegistryError, Result};
pub use hbin::HbinHeader;
pub use hive::HbinIterator;
pub use header::BaseBlock;
pub use hive::{Hive, RegistryKey, RegistryValue};
pub use key::KeyNode;
pub use subkey_list::{SubkeyList, SubkeyListEntry, SubkeyListType};
pub use transaction_log::{TransactionLog, DirtyPage};
pub use value::{ValueData, ValueKey};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
