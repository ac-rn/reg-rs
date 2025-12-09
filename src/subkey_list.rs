//! Subkey list parsing (lf, lh, li, ri).
//!
//! Registry keys can have multiple subkeys, which are organized in various
//! list structures for efficient lookup.

use crate::error::{RegistryError, Result};
use crate::utils::read_u32_le;

/// Subkey list types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubkeyListType {
    /// Index leaf (li) - simple list of offsets.
    IndexLeaf,
    
    /// Fast leaf (lf) - list with 4-byte name hints.
    FastLeaf,
    
    /// Hash leaf (lh) - list with name hash hints.
    HashLeaf,
    
    /// Index root (ri) - list of subkey list offsets.
    IndexRoot,
}

impl SubkeyListType {
    /// Parses a subkey list type from a 2-byte signature.
    pub fn from_signature(sig: &[u8; 2]) -> Result<Self> {
        match sig {
            b"li" => Ok(SubkeyListType::IndexLeaf),
            b"lf" => Ok(SubkeyListType::FastLeaf),
            b"lh" => Ok(SubkeyListType::HashLeaf),
            b"ri" => Ok(SubkeyListType::IndexRoot),
            _ => Err(RegistryError::InvalidSubkeyList { list_type: *sig }),
        }
    }
}

/// Subkey list entry (for lf/lh lists).
#[derive(Debug, Clone)]
pub struct SubkeyListEntry {
    /// Offset to the key node.
    pub key_offset: u32,
    
    /// Name hint (first 4 chars for lf, hash for lh).
    pub name_hint: u32,
}

/// Parsed subkey list.
#[derive(Debug, Clone)]
pub enum SubkeyList {
    /// Index leaf - simple list of offsets.
    IndexLeaf(Vec<u32>),
    
    /// Fast leaf or hash leaf - list with hints.
    LeafWithHints(Vec<SubkeyListEntry>),
    
    /// Index root - list of sublist offsets.
    IndexRoot(Vec<u32>),
}

impl SubkeyList {
    /// Parses a subkey list from cell data.
    ///
    /// # Arguments
    ///
    /// * `data` - Cell data (excluding size field).
    /// * `offset` - Offset of this cell for error reporting.
    pub fn parse(data: &[u8], offset: u32) -> Result<Self> {
        if data.len() < 4 {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: 4,
                actual: data.len(),
            });
        }

        let sig = [data[0], data[1]];
        let list_type = SubkeyListType::from_signature(&sig)?;
        
        let count = u16::from_le_bytes([data[2], data[3]]) as usize;

        match list_type {
            SubkeyListType::IndexLeaf => {
                // li: signature (2) + count (2) + offsets (4 * count)
                let expected_size = 4 + (count * 4);
                if data.len() < expected_size {
                    return Err(RegistryError::TruncatedData {
                        offset,
                        expected: expected_size,
                        actual: data.len(),
                    });
                }

                let mut offsets = Vec::with_capacity(count);
                for i in 0..count {
                    let offset_pos = 4 + (i * 4);
                    offsets.push(read_u32_le(data, offset_pos)?);
                }

                Ok(SubkeyList::IndexLeaf(offsets))
            }

            SubkeyListType::FastLeaf | SubkeyListType::HashLeaf => {
                // lf/lh: signature (2) + count (2) + entries (8 * count)
                // Each entry: offset (4) + hint (4)
                let expected_size = 4 + (count * 8);
                if data.len() < expected_size {
                    return Err(RegistryError::TruncatedData {
                        offset,
                        expected: expected_size,
                        actual: data.len(),
                    });
                }

                let mut entries = Vec::with_capacity(count);
                for i in 0..count {
                    let entry_pos = 4 + (i * 8);
                    let key_offset = read_u32_le(data, entry_pos)?;
                    let name_hint = read_u32_le(data, entry_pos + 4)?;
                    
                    entries.push(SubkeyListEntry {
                        key_offset,
                        name_hint,
                    });
                }

                Ok(SubkeyList::LeafWithHints(entries))
            }

            SubkeyListType::IndexRoot => {
                // ri: signature (2) + count (2) + offsets (4 * count)
                let expected_size = 4 + (count * 4);
                if data.len() < expected_size {
                    return Err(RegistryError::TruncatedData {
                        offset,
                        expected: expected_size,
                        actual: data.len(),
                    });
                }

                let mut offsets = Vec::with_capacity(count);
                for i in 0..count {
                    let offset_pos = 4 + (i * 4);
                    offsets.push(read_u32_le(data, offset_pos)?);
                }

                Ok(SubkeyList::IndexRoot(offsets))
            }
        }
    }

    /// Returns all key offsets from this list.
    ///
    /// For IndexRoot lists, this only returns the sublist offsets,
    /// not the actual key offsets.
    ///
    /// This method returns a slice to avoid unnecessary cloning.
    /// For LeafWithHints, use `key_offsets_iter()` instead.
    pub fn key_offsets(&self) -> &[u32] {
        match self {
            SubkeyList::IndexLeaf(offsets) => offsets,
            SubkeyList::IndexRoot(offsets) => offsets,
            SubkeyList::LeafWithHints(_) => {
                // For this variant, offsets need to be extracted
                // Callers should use key_offsets_iter() for this case
                &[]
            }
        }
    }
    
    /// Returns an iterator over key offsets.
    ///
    /// This is more efficient than `key_offsets()` for LeafWithHints
    /// as it avoids allocating a temporary vector.
    pub fn key_offsets_iter(&self) -> impl Iterator<Item = u32> + '_ {
        match self {
            SubkeyList::IndexLeaf(offsets) => {
                Box::new(offsets.iter().copied()) as Box<dyn Iterator<Item = u32> + '_>
            }
            SubkeyList::LeafWithHints(entries) => {
                Box::new(entries.iter().map(|e| e.key_offset))
            }
            SubkeyList::IndexRoot(offsets) => {
                Box::new(offsets.iter().copied())
            }
        }
    }

    /// Returns the number of entries in this list.
    pub fn len(&self) -> usize {
        match self {
            SubkeyList::IndexLeaf(offsets) => offsets.len(),
            SubkeyList::LeafWithHints(entries) => entries.len(),
            SubkeyList::IndexRoot(offsets) => offsets.len(),
        }
    }

    /// Returns true if this list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if this is an index root (contains sublists).
    pub fn is_index_root(&self) -> bool {
        matches!(self, SubkeyList::IndexRoot(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subkey_list_type() {
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
    }

    #[test]
    fn test_invalid_signature() {
        let result = SubkeyListType::from_signature(b"XX");
        assert!(result.is_err());
    }
}
