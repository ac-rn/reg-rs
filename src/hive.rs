//! Main registry hive parser with memory-mapped file support.

use crate::bigdata::BigDataBlock;
use crate::error::{RegistryError, Result};
use crate::hbin::HbinHeader;
use crate::header::{BaseBlock, BASE_BLOCK_SIZE};
use crate::key::KeyNode;
use crate::subkey_list::SubkeyList;
use crate::transaction_log::{TransactionLog, merge_transaction_logs};
use crate::utils::{cell_offset_to_absolute, calculate_checksum};
use crate::value::{ValueData, ValueKey};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn, instrument};

/// Main registry hive parser.
///
/// This structure provides access to a Windows registry hive file using
/// memory-mapped I/O for efficient, zero-copy parsing.
///
/// # Caching
///
/// The hive maintains an internal cache of parsed key nodes to avoid redundant
/// parsing during traversal. The cache uses interior mutability via `RwLock`
/// to allow caching while keeping the API immutable and thread-safe.
pub struct Hive {
    /// Hive data - either memory-mapped or owned.
    data: HiveData,
    
    /// Parsed base block header.
    base_block: BaseBlock,
    
    /// Cache of parsed key nodes (offset -> KeyNode).
    /// Uses RwLock for interior mutability to allow thread-safe caching with &self.
    key_cache: RwLock<HashMap<u32, KeyNode>>,
}

/// Represents hive data storage.
enum HiveData {
    /// Memory-mapped file data.
    Mapped(Mmap),
    /// Owned data (used after applying transaction logs).
    Owned(Arc<Vec<u8>>),
}

impl HiveData {
    /// Returns a slice of the hive data.
    fn as_slice(&self) -> &[u8] {
        match self {
            HiveData::Mapped(mmap) => mmap,
            HiveData::Owned(data) => data,
        }
    }

    /// Returns the length of the hive data.
    fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Converts to a Vec<u8>.
    fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl Hive {
    /// Opens a registry hive file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the registry hive file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be opened
    /// - File is not a valid registry hive
    /// - Header is corrupted
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reg_parser::Hive;
    ///
    /// let hive = Hive::open("SYSTEM").unwrap();
    /// ```
    #[instrument(skip(path), fields(path = %path.as_ref().display()))]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        info!("Opening registry hive");
        let file = File::open(&path)?;
        debug!("File opened successfully");
        
        // Validate file size BEFORE creating memory map
        let metadata = file.metadata()?;
        let file_size = metadata.len() as usize;
        
        if file_size < BASE_BLOCK_SIZE {
            return Err(RegistryError::HiveTooSmall {
                size: file_size,
                minimum: BASE_BLOCK_SIZE,
            });
        }
        
        debug!(size = file_size, "File size validated");
        
        // SAFETY: This is safe because:
        // 1. The file is opened in read-only mode (no write access)
        // 2. The file size has been validated to be at least BASE_BLOCK_SIZE
        // 3. The mmap lifetime is tied to the Hive lifetime
        // 4. All access to the mmap is bounds-checked via read_cell() and other methods
        // 5. The file descriptor remains valid for the lifetime of the mmap
        // 6. No other code has mutable access to the underlying file
        let mmap = unsafe { Mmap::map(&file)? };
        debug!(size = mmap.len(), "Memory mapped hive file");
        
        Self::from_data(HiveData::Mapped(mmap))
    }

    /// Creates a hive parser from a memory-mapped region.
    ///
    /// # Arguments
    ///
    /// * `mmap` - Memory-mapped hive data.
    pub fn from_mmap(mmap: Mmap) -> Result<Self> {
        Self::from_data(HiveData::Mapped(mmap))
    }

    /// Creates a hive parser from owned data.
    ///
    /// # Arguments
    ///
    /// * `data` - Owned hive data.
    pub fn from_vec(data: Vec<u8>) -> Result<Self> {
        Self::from_data(HiveData::Owned(Arc::new(data)))
    }

    /// Creates a hive parser from hive data.
    fn from_data(data: HiveData) -> Result<Self> {
        // Parse base block
        let base_block = BaseBlock::parse(data.as_slice())?;
        
        Ok(Self {
            data,
            base_block,
            key_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Returns a reference to the base block header.
    pub fn base_block(&self) -> &BaseBlock {
        &self.base_block
    }

    /// Returns the root key of the hive.
    ///
    /// # Errors
    ///
    /// Returns an error if the root key cannot be parsed.
    #[instrument(skip(self))]
    pub fn root_key(&self) -> Result<RegistryKey> {
        debug!(offset = %format!("{:#x}", self.base_block.root_cell_offset), "Accessing root key");
        let offset = self.base_block.root_cell_offset;
        self.get_key(offset)
    }

    /// Gets a key node by its cell offset.
    ///
    /// # Arguments
    ///
    /// * `offset` - Cell offset (relative to first hbin).
    ///
    /// # Caching
    ///
    /// This method uses an internal cache to avoid re-parsing the same key node
    /// multiple times during traversal. The cache is transparent to the caller.
    pub fn get_key(&self, offset: u32) -> Result<RegistryKey> {
        // Check cache first (read lock)
        if let Some(key_node) = self.key_cache.read()
            .expect("key cache lock poisoned")
            .get(&offset) 
        {
            debug!(offset = %format!("{:#x}", offset), "Cache hit for key node");
            return Ok(RegistryKey {
                hive: self,
                offset,
                key_node: key_node.clone(),
            });
        }
        
        // Parse and cache (write lock)
        debug!(offset = %format!("{:#x}", offset), "Cache miss, parsing key node");
        let key_node = self.parse_key_node(offset)?;
        self.key_cache.write()
            .expect("key cache lock poisoned")
            .insert(offset, key_node.clone());
        
        Ok(RegistryKey {
            hive: self,
            offset,
            key_node,
        })
    }

    /// Parses a key node at the given offset.
    fn parse_key_node(&self, offset: u32) -> Result<KeyNode> {
        let cell_data = self.read_cell(offset)?;
        KeyNode::parse(cell_data, offset)
    }

    /// Reads a cell at the given offset.
    ///
    /// # Arguments
    ///
    /// * `offset` - Cell offset (relative to first hbin).
    ///
    /// # Returns
    ///
    /// Returns the cell data (excluding the size field).
    fn read_cell(&self, offset: u32) -> Result<&[u8]> {
        let abs_offset = cell_offset_to_absolute(offset)? as usize;
        let data = self.data.as_slice();
        
        if abs_offset >= data.len() {
            return Err(RegistryError::invalid_offset(offset, data.len()));
        }

        // Read cell size
        if abs_offset + 4 > data.len() {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: 4,
                actual: data.len() - abs_offset,
            });
        }

        let size_bytes = &data[abs_offset..abs_offset + 4];
        let size = i32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]);
        
        let abs_size = size.unsigned_abs() as usize;
        
        if abs_size < 4 {
            return Err(RegistryError::invalid_cell_size(size, offset));
        }

        let data_start = abs_offset + 4;
        let data_end = abs_offset + abs_size;

        if data_end > data.len() {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: abs_size,
                actual: data.len() - abs_offset,
            });
        }

        Ok(&data[data_start..data_end])
    }

    /// Parses a subkey list at the given offset.
    fn parse_subkey_list(&self, offset: u32) -> Result<SubkeyList> {
        let cell_data = self.read_cell(offset)?;
        SubkeyList::parse(cell_data, offset)
    }

    /// Parses a value key at the given offset.
    fn parse_value_key(&self, offset: u32) -> Result<ValueKey> {
        let cell_data = self.read_cell(offset)?;
        ValueKey::parse(cell_data, offset)
    }

    /// Reads value data at the given offset.
    ///
    /// This method handles both regular values and big data blocks (values > 16KB).
    /// Big data blocks are stored in a "db" structure with multiple segments.
    ///
    /// # Arguments
    ///
    /// * `offset` - Cell offset of the value data
    /// * `length` - Length of the value data in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the data cannot be read or is corrupted.
    fn read_value_data(&self, offset: u32, length: u32) -> Result<Vec<u8>> {
        if length == 0 {
            return Ok(Vec::new());
        }

        // Maximum size for direct cell storage (before big data blocks are used)
        const MAX_DIRECT_DATA_SIZE: u32 = 16344;

        // For large data (>16344 bytes), data is stored in a db structure
        if length > MAX_DIRECT_DATA_SIZE {
            return self.read_big_data(offset, length);
        }

        // Regular data - read directly from cell
        Ok(self.read_cell(offset)?.to_vec())
    }
    
    /// Reads big data block (values > 16KB).
    ///
    /// Big data blocks consist of a header cell ("db" signature) followed by
    /// a list of segment offsets. Each segment contains a portion of the data.
    ///
    /// # Arguments
    ///
    /// * `offset` - Cell offset of the big data block header
    /// * `expected_length` - Expected total length of the data
    ///
    /// # Errors
    ///
    /// Returns an error if the big data structure is corrupted or segments are missing.
    fn read_big_data(&self, offset: u32, expected_length: u32) -> Result<Vec<u8>> {
        debug!("Reading big data block at offset {:#x}, expected length {}", offset, expected_length);
        
        // Read the big data block header
        let header_cell = self.read_cell(offset)?;
        let db_header = BigDataBlock::parse(header_cell, offset)?;
        
        debug!("Big data block has {} segments", db_header.segment_count);
        
        // Read the segment list (array of u32 offsets)
        let segment_list_cell = self.read_cell(db_header.segment_list_offset)?;
        
        // Each segment offset is 4 bytes
        let expected_list_size = db_header.segment_count as usize * 4;
        if segment_list_cell.len() < expected_list_size {
            return Err(RegistryError::TruncatedData {
                offset: db_header.segment_list_offset,
                expected: expected_list_size,
                actual: segment_list_cell.len(),
            });
        }
        
        // Parse segment offsets
        let mut segment_offsets = Vec::with_capacity(db_header.segment_count as usize);
        for i in 0..db_header.segment_count {
            let offset_pos = (i as usize) * 4;
            let segment_offset = u32::from_le_bytes([
                segment_list_cell[offset_pos],
                segment_list_cell[offset_pos + 1],
                segment_list_cell[offset_pos + 2],
                segment_list_cell[offset_pos + 3],
            ]);
            
            // High bit indicates the segment is part of the big data
            // Clear it to get the actual offset
            let actual_offset = segment_offset & 0x7FFFFFFF;
            segment_offsets.push(actual_offset);
        }
        
        // Read and concatenate all segments
        let mut data = Vec::with_capacity(expected_length as usize);
        for (i, segment_offset) in segment_offsets.iter().enumerate() {
            debug!("Reading segment {} at offset {:#x}", i, segment_offset);
            
            let segment_data = self.read_cell(*segment_offset)?;
            data.extend_from_slice(segment_data);
            
            // Stop if we've read enough data
            if data.len() >= expected_length as usize {
                break;
            }
        }
        
        // Truncate to expected length (segments might contain extra data)
        data.truncate(expected_length as usize);
        
        debug!("Successfully read {} bytes from big data block", data.len());
        
        Ok(data)
    }

    /// Iterates over all hbins in the hive.
    pub fn hbins(&self) -> HbinIterator {
        let data = self.data.as_slice();
        HbinIterator {
            data: &data[BASE_BLOCK_SIZE..],
            offset: 0,
        }
    }

    /// Debug method: Read raw bytes at an absolute offset.
    /// This is for debugging purposes only.
    #[doc(hidden)]
    pub fn read_raw_bytes(&self, abs_offset: usize, length: usize) -> Result<&[u8]> {
        let data = self.data.as_slice();
        if abs_offset + length > data.len() {
            return Err(RegistryError::TruncatedData {
                offset: abs_offset as u32,
                expected: length,
                actual: data.len() - abs_offset,
            });
        }
        Ok(&data[abs_offset..abs_offset + length])
    }

    /// Debug method: Read cell with size field included.
    /// This is for debugging purposes only.
    #[doc(hidden)]
    pub fn read_cell_with_size(&self, offset: u32) -> Result<&[u8]> {
        let abs_offset = cell_offset_to_absolute(offset)? as usize;
        let data = self.data.as_slice();
        
        if abs_offset >= data.len() {
            return Err(RegistryError::invalid_offset(offset, data.len()));
        }

        if abs_offset + 4 > data.len() {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: 4,
                actual: data.len() - abs_offset,
            });
        }

        let size_bytes = &data[abs_offset..abs_offset + 4];
        let size = i32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]);
        let abs_size = size.unsigned_abs() as usize;
        
        if abs_size < 4 {
            return Err(RegistryError::invalid_cell_size(size, offset));
        }

        let data_end = abs_offset + abs_size;

        if data_end > data.len() {
            return Err(RegistryError::TruncatedData {
                offset,
                expected: abs_size,
                actual: data.len() - abs_offset,
            });
        }

        Ok(&data[abs_offset..data_end])
    }

    /// Opens a hive with transaction logs applied.
    ///
    /// This method opens a base hive and applies transaction logs (.LOG1, .LOG2)
    /// to recover uncommitted changes. The resulting hive is loaded into memory.
    ///
    /// # Arguments
    ///
    /// * `hive_path` - Path to the base hive file.
    /// * `log1_path` - Optional path to .LOG1 file.
    /// * `log2_path` - Optional path to .LOG2 file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Hive file cannot be opened
    /// - Transaction logs are invalid
    /// - Merged hive is corrupted
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reg_parser::Hive;
    ///
    /// let hive = Hive::open_with_logs(
    ///     "SYSTEM",
    ///     Some("SYSTEM.LOG1"),
    ///     Some("SYSTEM.LOG2")
    /// ).unwrap();
    /// ```
    pub fn open_with_logs<P: AsRef<Path>>(
        hive_path: P,
        log1_path: Option<P>,
        log2_path: Option<P>,
    ) -> Result<Self> {
        // Read base hive into memory
        let mut file = File::open(hive_path)?;
        let mut hive_data = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut hive_data)?;

        // Apply transaction logs
        let applied = merge_transaction_logs(&mut hive_data, log1_path, log2_path)?;
        
        if applied > 0 {
            // Recalculate checksum after applying logs
            Self::update_checksum(&mut hive_data)?;
        }

        // Create hive from modified data
        Self::from_vec(hive_data)
    }

    /// Applies a transaction log to this hive.
    ///
    /// This method creates a new Hive instance with the transaction log applied.
    /// The original hive is not modified.
    ///
    /// # Arguments
    ///
    /// * `log_path` - Path to the transaction log file (.LOG1 or .LOG2).
    ///
    /// # Returns
    ///
    /// Returns a new Hive instance with the transaction log applied.
    pub fn apply_transaction_log<P: AsRef<Path>>(&self, log_path: P) -> Result<Self> {
        // Read current hive data
        let mut hive_data = self.data.to_vec();

        // Parse and apply transaction log
        let log = TransactionLog::open(log_path)?;
        log.apply_to_hive(&mut hive_data)?;

        // Recalculate checksum
        Self::update_checksum(&mut hive_data)?;

        // Create new hive from modified data
        Self::from_vec(hive_data)
    }

    /// Saves the hive to a new file.
    ///
    /// This method writes the current hive data (including any applied transaction logs)
    /// to a new file. The checksum is recalculated before saving.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path where the cleaned hive should be saved.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be created
    /// - Write operation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reg_parser::Hive;
    ///
    /// let hive = Hive::open_with_logs(
    ///     "SYSTEM",
    ///     Some("SYSTEM.LOG1"),
    ///     Some("SYSTEM.LOG2")
    /// ).unwrap();
    ///
    /// hive.save("SYSTEM_cleaned").unwrap();
    /// ```
    pub fn save<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let mut hive_data = self.data.to_vec();

        // Recalculate and update checksum
        Self::update_checksum(&mut hive_data)?;

        // Write to file
        let mut file = File::create(output_path)?;
        file.write_all(&hive_data)?;
        file.flush()?;

        Ok(())
    }

    /// Updates the checksum in the base block.
    ///
    /// # Arguments
    ///
    /// * `data` - Mutable reference to hive data.
    fn update_checksum(data: &mut [u8]) -> Result<()> {
        if data.len() < BASE_BLOCK_SIZE {
            return Err(RegistryError::HiveTooSmall {
                size: data.len(),
                minimum: BASE_BLOCK_SIZE,
            });
        }

        // Calculate new checksum
        let checksum = calculate_checksum(data);

        // Write checksum at offset 0x1FC
        data[0x1FC..0x200].copy_from_slice(&checksum.to_le_bytes());

        Ok(())
    }

    /// Exports the hive data as a Vec<u8>.
    ///
    /// This is useful for further processing or analysis.
    ///
    /// # Returns
    ///
    /// Returns a copy of the hive data.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.to_vec()
    }
}

/// Iterator over hbins in a hive.
pub struct HbinIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for HbinIterator<'a> {
    type Item = Result<HbinHeader>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let hbin_data = &self.data[self.offset..];
        let result = HbinHeader::parse(hbin_data, self.offset as u32);

        match result {
            Ok(ref header) => {
                self.offset += header.size as usize;
                Some(Ok(header.clone()))
            }
            Err(RegistryError::InvalidSignature { .. }) => {
                // Stop iteration when we hit invalid signatures (padding/EOF)
                None
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// A registry key with access to its hive.
pub struct RegistryKey<'a> {
    hive: &'a Hive,
    /// Cell offset of this key (relative to first hbin).
    pub offset: u32,
    key_node: KeyNode,
}

impl<'a> RegistryKey<'a> {
    /// Returns a reference to the key node data.
    fn key_node(&self) -> &KeyNode {
        &self.key_node
    }

    /// Debug method: Returns the key node data for debugging.
    #[doc(hidden)]
    pub fn debug_key_node(&self) -> &KeyNode {
        self.key_node()
    }

    /// Debug method: Returns the hive reference for debugging.
    #[doc(hidden)]
    pub fn debug_hive(&self) -> &Hive {
        self.hive
    }

    /// Returns the key name.
    pub fn name(&self) -> Result<String> {
        Ok(self.key_node().name.clone())
    }

    /// Returns the number of subkeys.
    pub fn subkey_count(&self) -> Result<u32> {
        Ok(self.key_node().subkey_count)
    }

    /// Returns the number of values.
    pub fn value_count(&self) -> Result<u32> {
        Ok(self.key_node().value_count)
    }

    /// Returns an iterator over subkeys.
    pub fn subkeys(&self) -> Result<Vec<RegistryKey>> {
        let key_node = self.key_node();
        
        if !key_node.has_subkeys() {
            return Ok(Vec::new());
        }

        let mut subkey_offsets = Vec::new();
        self.collect_subkey_offsets(key_node.subkey_list_offset, &mut subkey_offsets)?;

        let mut subkeys = Vec::new();
        for offset in subkey_offsets {
            subkeys.push(self.hive.get_key(offset)?);
        }

        Ok(subkeys)
    }

    /// Recursively collects subkey offsets from subkey lists.
    fn collect_subkey_offsets(&self, list_offset: u32, offsets: &mut Vec<u32>) -> Result<()> {
        if list_offset == 0xFFFFFFFF || list_offset == 0 {
            return Ok(());
        }

        let subkey_list = self.hive.parse_subkey_list(list_offset)?;

        if subkey_list.is_index_root() {
            // Index root contains offsets to other subkey lists
            // Use iterator to avoid cloning
            for offset in subkey_list.key_offsets_iter() {
                self.collect_subkey_offsets(offset, offsets)?;
            }
        } else {
            // Direct key offsets - use iterator to avoid cloning
            offsets.extend(subkey_list.key_offsets_iter());
        }

        Ok(())
    }

    /// Returns an iterator over values.
    pub fn values(&self) -> Result<Vec<RegistryValue>> {
        let key_node = self.key_node();
        
        if !key_node.has_values() {
            return Ok(Vec::new());
        }

        if key_node.value_list_offset == 0xFFFFFFFF || key_node.value_list_offset == 0 {
            return Ok(Vec::new());
        }

        // Value list is an array of offsets
        let list_data = self.hive.read_cell(key_node.value_list_offset)?;
        let value_count = key_node.value_count as usize;
        
        if list_data.len() < value_count * 4 {
            return Err(RegistryError::TruncatedData {
                offset: key_node.value_list_offset,
                expected: value_count * 4,
                actual: list_data.len(),
            });
        }

        let mut values = Vec::new();
        for i in 0..value_count {
            let offset_pos = i * 4;
            let offset = u32::from_le_bytes([
                list_data[offset_pos],
                list_data[offset_pos + 1],
                list_data[offset_pos + 2],
                list_data[offset_pos + 3],
            ]);

            let value_key = self.hive.parse_value_key(offset)?;
            values.push(RegistryValue {
                hive: self.hive,
                value_key,
            });
        }

        Ok(values)
    }

    /// Gets a specific value by name.
    pub fn value(&self, name: &str) -> Result<RegistryValue> {
        let values = self.values()?;
        
        for value in values {
            if value.value_key.name.eq_ignore_ascii_case(name) {
                return Ok(value);
            }
        }

        Err(RegistryError::NotFound(format!("Value '{}'", name)))
    }
}

/// A registry value.
pub struct RegistryValue<'a> {
    hive: &'a Hive,
    value_key: ValueKey,
}

impl<'a> RegistryValue<'a> {
    /// Returns the value name.
    pub fn name(&self) -> &str {
        &self.value_key.name
    }

    /// Returns the value data type.
    pub fn data_type(&self) -> crate::cell::ValueType {
        self.value_key.data_type
    }

    /// Returns the parsed value data.
    pub fn data(&self) -> Result<ValueData> {
        let raw_data = if self.value_key.is_inline_data() {
            self.value_key.inline_data()
        } else if self.value_key.data_offset == 0xFFFFFFFF || self.value_key.data_offset == 0 {
            Vec::new()
        } else {
            self.hive
                .read_value_data(self.value_key.data_offset, self.value_key.data_length)?
        };

        ValueData::parse(&raw_data, self.value_key.data_type, self.value_key.data_offset)
    }

    /// Returns the raw value data as bytes.
    pub fn raw_data(&self) -> Result<Vec<u8>> {
        if self.value_key.is_inline_data() {
            Ok(self.value_key.inline_data())
        } else if self.value_key.data_offset == 0xFFFFFFFF || self.value_key.data_offset == 0 {
            Ok(Vec::new())
        } else {
            self.hive
                .read_value_data(self.value_key.data_offset, self.value_key.data_length)
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests are in tests/ directory using real hive files
}
