//! Transaction log (.LOG1, .LOG2) parsing and application.
//!
//! Transaction logs contain dirty pages that need to be applied to the base hive
//! to recover uncommitted changes. This module provides functionality to parse
//! and apply these transaction logs.

use crate::error::{RegistryError, Result};
use crate::utils::read_u32_le;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Size of a dirty page in the transaction log.
const PAGE_SIZE: usize = 0x1000; // 4KB

/// Maximum reasonable hive size (512 MB)
/// 
/// This limit prevents malicious or corrupted transaction logs from
/// causing excessive memory allocation.
const MAX_HIVE_SIZE: usize = 512 * 1024 * 1024;

/// Maximum size extension per dirty page (16 MB)
/// 
/// This prevents a single dirty page from extending the hive by an
/// unreasonable amount.
const MAX_PAGE_EXTENSION: usize = 16 * 1024 * 1024;

/// Expected signature for transaction log base block ("HvLE").
const HVLE_SIGNATURE: &[u8; 4] = b"HvLE";

/// Expected signature for dirty page vector ("DIRT").
const DIRT_SIGNATURE: &[u8; 4] = b"DIRT";

/// Transaction log entry representing a dirty page.
#[derive(Debug, Clone)]
pub struct DirtyPage {
    /// Offset in the hive where this page should be applied.
    pub offset: u32,
    
    /// Size of the dirty data.
    pub size: u32,
    
    /// The dirty page data.
    pub data: Vec<u8>,
}

/// Transaction log parser.
#[derive(Debug)]
pub struct TransactionLog {
    /// Sequence number of the log.
    pub sequence: u32,
    
    /// Dirty pages to apply to the base hive.
    pub dirty_pages: Vec<DirtyPage>,
}

impl TransactionLog {
    /// Opens and parses a transaction log file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .LOG1 or .LOG2 file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be opened
    /// - File is not a valid transaction log
    /// - Log is corrupted
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        Self::parse(&data)
    }

    /// Parses transaction log data from raw bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes of the transaction log file.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < PAGE_SIZE {
            return Err(RegistryError::InvalidFormat(
                "Transaction log too small".to_string(),
            ));
        }

        // Parse base block
        let signature = &data[0..4];
        if signature != HVLE_SIGNATURE {
            return Err(RegistryError::invalid_signature(HVLE_SIGNATURE, signature));
        }

        // Sequence number at offset 0x04
        let sequence = read_u32_le(data, 0x04)?;

        // Parse dirty page vector
        let mut dirty_pages = Vec::new();
        
        // Look for DIRT signature in the log
        // Transaction logs typically have dirty page vectors starting at offset 0x1000
        let mut offset = PAGE_SIZE;
        
        while offset + 512 <= data.len() {
            // Check for DIRT signature
            if offset + 4 <= data.len() && &data[offset..offset + 4] == DIRT_SIGNATURE {
                // Parse dirty vector header
                if let Ok(pages) = Self::parse_dirty_vector(&data[offset..]) {
                    dirty_pages.extend(pages);
                }
                // Move to next potential dirty vector (they're page-aligned)
                offset += PAGE_SIZE;
            } else {
                offset += PAGE_SIZE;
            }
            
            // Limit search to reasonable size
            if offset > data.len() || dirty_pages.len() > 10000 {
                break;
            }
        }

        Ok(Self {
            sequence,
            dirty_pages,
        })
    }

    /// Parses a dirty page vector starting at the given offset.
    fn parse_dirty_vector(data: &[u8]) -> Result<Vec<DirtyPage>> {
        if data.len() < 16 {
            return Ok(Vec::new());
        }

        // DIRT signature already verified by caller
        
        // Number of dirty pages at offset 0x08
        let num_pages = if data.len() >= 12 {
            read_u32_le(data, 0x08)? as usize
        } else {
            return Ok(Vec::new());
        };

        if num_pages == 0 || num_pages > 1000 {
            return Ok(Vec::new());
        }

        let mut pages = Vec::new();
        let mut offset = 0x10; // Start of dirty page entries

        for _ in 0..num_pages {
            if offset + 8 > data.len() {
                break;
            }

            let page_offset = read_u32_le(data, offset)?;
            let page_size = read_u32_le(data, offset + 4)?;
            
            offset += 8;

            // Validate page size
            if page_size == 0 || page_size > PAGE_SIZE as u32 * 16 {
                continue;
            }

            // Read page data
            if offset + page_size as usize <= data.len() {
                let page_data = data[offset..offset + page_size as usize].to_vec();
                
                pages.push(DirtyPage {
                    offset: page_offset,
                    size: page_size,
                    data: page_data,
                });
                
                offset += page_size as usize;
            } else {
                break;
            }
        }

        Ok(pages)
    }

    /// Applies this transaction log to hive data.
    ///
    /// # Arguments
    ///
    /// * `hive_data` - Mutable reference to the hive data to modify.
    ///
    /// # Returns
    ///
    /// Returns the number of pages applied.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A dirty page offset would overflow
    /// - The resulting hive would exceed maximum size
    /// - A dirty page would extend the hive by too much at once
    /// - Page data size doesn't match the declared size
    pub fn apply_to_hive(&self, hive_data: &mut Vec<u8>) -> Result<usize> {
        let mut applied = 0;

        for (page_idx, page) in self.dirty_pages.iter().enumerate() {
            // Validate offset doesn't overflow
            let start = page.offset as usize;
            let end = start.checked_add(page.size as usize)
                .ok_or_else(|| RegistryError::InvalidFormat(
                    format!(
                        "Dirty page {} offset overflow: {:#x} + {:#x}",
                        page_idx, page.offset, page.size
                    )
                ))?;

            // Validate reasonable total size
            if end > MAX_HIVE_SIZE {
                return Err(RegistryError::InvalidFormat(
                    format!(
                        "Dirty page {} would extend hive beyond maximum size: {:#x} > {:#x}",
                        page_idx, end, MAX_HIVE_SIZE
                    )
                ));
            }

            // Extend hive if necessary, but only within reason
            if end > hive_data.len() {
                let extension = end - hive_data.len();
                
                // Validate extension size
                if extension > MAX_PAGE_EXTENSION {
                    return Err(RegistryError::InvalidFormat(
                        format!(
                            "Dirty page {} would extend hive by too much: {} bytes (max: {})",
                            page_idx, extension, MAX_PAGE_EXTENSION
                        )
                    ));
                }
                
                hive_data.resize(end, 0);
            }

            // Validate page data size matches declared size
            if page.data.len() != page.size as usize {
                return Err(RegistryError::InvalidFormat(
                    format!(
                        "Dirty page {} data size mismatch: {} != {}",
                        page_idx, page.data.len(), page.size
                    )
                ));
            }

            // Apply dirty page
            hive_data[start..end].copy_from_slice(&page.data);
            applied += 1;
        }

        Ok(applied)
    }
}

/// Applies multiple transaction logs to hive data.
///
/// # Arguments
///
/// * `hive_data` - Mutable reference to the hive data to modify.
/// * `logs` - Transaction logs to apply, in order.
///
/// # Returns
///
/// Returns the total number of pages applied.
pub fn apply_transaction_logs(hive_data: &mut Vec<u8>, logs: &[TransactionLog]) -> Result<usize> {
    let mut total_applied = 0;

    // Sort logs by sequence number
    let mut sorted_logs: Vec<_> = logs.iter().collect();
    sorted_logs.sort_by_key(|log| log.sequence);

    for log in sorted_logs {
        total_applied += log.apply_to_hive(hive_data)?;
    }

    Ok(total_applied)
}

/// Merges hive data with transaction logs from .LOG1 and .LOG2 files.
///
/// # Arguments
///
/// * `hive_data` - Mutable reference to the base hive data.
/// * `log1_path` - Optional path to .LOG1 file.
/// * `log2_path` - Optional path to .LOG2 file.
///
/// # Returns
///
/// Returns the total number of dirty pages applied.
pub fn merge_transaction_logs<P: AsRef<Path>>(
    hive_data: &mut Vec<u8>,
    log1_path: Option<P>,
    log2_path: Option<P>,
) -> Result<usize> {
    let mut logs = Vec::new();

    if let Some(path) = log1_path {
        match TransactionLog::open(path) {
            Ok(log) => logs.push(log),
            Err(_) => {
                // LOG1 might not exist or be invalid, continue
            }
        }
    }

    if let Some(path) = log2_path {
        match TransactionLog::open(path) {
            Ok(log) => logs.push(log),
            Err(_) => {
                // LOG2 might not exist or be invalid, continue
            }
        }
    }

    if logs.is_empty() {
        return Ok(0);
    }

    apply_transaction_logs(hive_data, &logs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_page_creation() {
        let page = DirtyPage {
            offset: 0x1000,
            size: 0x100,
            data: vec![0xAA; 0x100],
        };
        
        assert_eq!(page.offset, 0x1000);
        assert_eq!(page.size, 0x100);
        assert_eq!(page.data.len(), 0x100);
    }

    #[test]
    fn test_apply_dirty_page() {
        let mut hive_data = vec![0u8; 0x2000];
        
        let log = TransactionLog {
            sequence: 1,
            dirty_pages: vec![
                DirtyPage {
                    offset: 0x1000,
                    size: 4,
                    data: vec![0xDE, 0xAD, 0xBE, 0xEF],
                },
            ],
        };

        let applied = log.apply_to_hive(&mut hive_data).unwrap();
        assert_eq!(applied, 1);
        assert_eq!(&hive_data[0x1000..0x1004], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = vec![0u8; PAGE_SIZE];
        data[0..4].copy_from_slice(b"XXXX");
        
        let result = TransactionLog::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_overflow_protection() {
        let mut hive_data = vec![0u8; 0x2000];
        
        let log = TransactionLog {
            sequence: 1,
            dirty_pages: vec![
                DirtyPage {
                    offset: u32::MAX - 100,  // Would overflow
                    size: 200,
                    data: vec![0xAA; 200],
                },
            ],
        };

        let result = log.apply_to_hive(&mut hive_data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::InvalidFormat { .. }));
    }

    #[test]
    fn test_max_size_protection() {
        let mut hive_data = vec![0u8; 0x2000];
        
        let log = TransactionLog {
            sequence: 1,
            dirty_pages: vec![
                DirtyPage {
                    offset: MAX_HIVE_SIZE as u32,  // Beyond max
                    size: 100,
                    data: vec![0xAA; 100],
                },
            ],
        };

        let result = log.apply_to_hive(&mut hive_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_extension_limit() {
        let mut hive_data = vec![0u8; 0x2000];
        
        let log = TransactionLog {
            sequence: 1,
            dirty_pages: vec![
                DirtyPage {
                    offset: 0x2000,
                    size: (MAX_PAGE_EXTENSION + 1) as u32,  // Too large
                    data: vec![0xAA; MAX_PAGE_EXTENSION + 1],
                },
            ],
        };

        let result = log.apply_to_hive(&mut hive_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_size_mismatch() {
        let mut hive_data = vec![0u8; 0x2000];
        
        let log = TransactionLog {
            sequence: 1,
            dirty_pages: vec![
                DirtyPage {
                    offset: 0x1000,
                    size: 100,
                    data: vec![0xAA; 50],  // Size mismatch!
                },
            ],
        };

        let result = log.apply_to_hive(&mut hive_data);
        assert!(result.is_err());
    }
}
