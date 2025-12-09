//! Python bindings for the Windows Registry parser using PyO3.
//!
//! This module provides Python-friendly wrappers around the core Rust types.

use pyo3::prelude::*;
use pyo3::exceptions::{PyIOError, PyValueError, PyRuntimeError};
use pyo3::types::PyBytes;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use crate::{Hive as RustHive};
use crate::{ValueData as RustValueData, ValueType as RustValueType};
use crate::{BaseBlock as RustBaseBlock, HbinHeader as RustHbinHeader};
use crate::RegistryError;

/// Convert Rust RegistryError to Python exception
fn registry_error_to_py(err: RegistryError) -> PyErr {
    match err {
        RegistryError::Io(e) => PyIOError::new_err(e.to_string()),
        
        RegistryError::InvalidSignature { expected, found } => {
            PyValueError::new_err(format!(
                "Invalid signature: expected {:?}, found {:?}",
                String::from_utf8_lossy(&expected),
                String::from_utf8_lossy(&found)
            ))
        }
        
        RegistryError::ChecksumMismatch { expected, calculated } => {
            PyValueError::new_err(format!(
                "Checksum mismatch: expected {:#x}, calculated {:#x}",
                expected, calculated
            ))
        }
        
        RegistryError::InvalidOffset { offset, hive_size } => {
            PyValueError::new_err(format!(
                "Invalid offset {:#x} (hive size: {:#x})",
                offset, hive_size
            ))
        }
        
        RegistryError::TruncatedData { offset, expected, actual } => {
            PyValueError::new_err(format!(
                "Truncated data at offset {:#x}: expected {} bytes, got {} bytes",
                offset, expected, actual
            ))
        }
        
        RegistryError::UnsupportedVersion { major, minor } => {
            PyValueError::new_err(format!(
                "Unsupported hive version: {}.{}",
                major, minor
            ))
        }
        
        RegistryError::InvalidValueType(type_id) => {
            PyValueError::new_err(format!("Invalid value type: {}", type_id))
        }
        
        RegistryError::InvalidUtf16 { offset } => {
            PyValueError::new_err(format!(
                "Invalid UTF-16 string at offset {:#x}",
                offset
            ))
        }
        
        RegistryError::NotFound(msg) => {
            PyValueError::new_err(format!("Not found: {}", msg))
        }
        
        RegistryError::InvalidFormat(msg) => {
            PyValueError::new_err(format!("Invalid format: {}", msg))
        }
        
        RegistryError::InvalidCellSize { size, offset } => {
            PyValueError::new_err(format!(
                "Invalid cell size {} at offset {:#x}",
                size, offset
            ))
        }
        
        RegistryError::UnknownCellType { cell_type, offset } => {
            PyValueError::new_err(format!(
                "Unknown cell type {:?} at offset {:#x}",
                cell_type, offset
            ))
        }
        
        RegistryError::HiveTooSmall { size, minimum } => {
            PyValueError::new_err(format!(
                "Hive too small: {} bytes (minimum: {} bytes)",
                size, minimum
            ))
        }
        
        RegistryError::InvalidSubkeyList { list_type } => {
            PyValueError::new_err(format!(
                "Invalid subkey list type: {:?}",
                list_type
            ))
        }
        
        RegistryError::BigDataNotSupported { size, max_supported } => {
            PyValueError::new_err(format!(
                "Big data blocks not supported: value size is {} bytes (max supported: {} bytes)",
                size, max_supported
            ))
        }
    }
}

/// Python wrapper for BaseBlock
#[pyclass(name = "BaseBlock")]
#[derive(Clone)]
pub struct PyBaseBlock {
    inner: RustBaseBlock,
}

#[pymethods]
impl PyBaseBlock {
    /// Get the signature (should be "regf")
    #[getter]
    fn signature(&self) -> String {
        String::from_utf8_lossy(&self.inner.signature).to_string()
    }

    /// Get the primary sequence number
    #[getter]
    fn primary_sequence(&self) -> u32 {
        self.inner.primary_sequence
    }

    /// Get the secondary sequence number
    #[getter]
    fn secondary_sequence(&self) -> u32 {
        self.inner.secondary_sequence
    }

    /// Get the root cell offset
    #[getter]
    fn root_cell_offset(&self) -> u32 {
        self.inner.root_cell_offset
    }

    /// Get the hive length (size of hive data in bytes)
    #[getter]
    fn hive_length(&self) -> u32 {
        self.inner.hive_length
    }

    /// Get the file name
    #[getter]
    fn file_name(&self) -> String {
        self.inner.file_name.clone()
    }

    /// Get the hive bins data size (same as hive_length)
    #[getter]
    fn hive_bins_data_size(&self) -> u32 {
        self.inner.hive_length
    }

    fn __repr__(&self) -> String {
        format!(
            "BaseBlock(signature={}, root_offset={:#x}, hive_length={:#x})",
            self.signature(),
            self.root_cell_offset(),
            self.hive_length()
        )
    }
}

/// Python wrapper for HbinHeader
#[pyclass(name = "HbinHeader")]
#[derive(Clone)]
pub struct PyHbinHeader {
    inner: RustHbinHeader,
}

#[pymethods]
impl PyHbinHeader {
    /// Get the signature (should be "hbin")
    #[getter]
    fn signature(&self) -> String {
        String::from_utf8_lossy(&self.inner.signature).to_string()
    }

    /// Get the offset from the start of the hive bins data
    #[getter]
    fn offset(&self) -> u32 {
        self.inner.offset
    }

    /// Get the size of this hbin
    #[getter]
    fn size(&self) -> u32 {
        self.inner.size
    }

    fn __repr__(&self) -> String {
        format!(
            "HbinHeader(offset={:#x}, size={:#x})",
            self.offset(),
            self.size()
        )
    }
}

/// Python wrapper for ValueType
#[pyclass(name = "ValueType")]
#[derive(Clone)]
pub struct PyValueType {
    inner: RustValueType,
}

#[pymethods]
impl PyValueType {
    /// Get the type name as string (e.g., "REG_SZ", "REG_DWORD")
    fn type_name(&self) -> String {
        self.inner.name()
    }

    /// Get the numeric type ID
    fn type_id(&self) -> u32 {
        match self.inner {
            RustValueType::None => 0,
            RustValueType::String => 1,
            RustValueType::ExpandString => 2,
            RustValueType::Binary => 3,
            RustValueType::Dword => 4,
            RustValueType::DwordBigEndian => 5,
            RustValueType::Link => 6,
            RustValueType::MultiString => 7,
            RustValueType::ResourceList => 8,
            RustValueType::FullResourceDescriptor => 9,
            RustValueType::ResourceRequirementsList => 10,
            RustValueType::Qword => 11,
            RustValueType::Unknown(id) => id,
        }
    }

    fn __repr__(&self) -> String {
        format!("ValueType({})", self.inner.name())
    }

    fn __str__(&self) -> String {
        self.inner.name()
    }
}

/// Python wrapper for ValueData
#[pyclass(name = "ValueData")]
#[derive(Clone)]
pub struct PyValueData {
    inner: RustValueData,
}

#[pymethods]
impl PyValueData {
    /// Check if this is a None value
    fn is_none(&self) -> bool {
        matches!(self.inner, RustValueData::None)
    }

    /// Check if this is a String value
    fn is_string(&self) -> bool {
        matches!(self.inner, RustValueData::String(_) | RustValueData::ExpandString(_))
    }

    /// Check if this is a Binary value
    fn is_binary(&self) -> bool {
        matches!(self.inner, RustValueData::Binary(_))
    }

    /// Check if this is a DWORD value
    fn is_dword(&self) -> bool {
        matches!(self.inner, RustValueData::Dword(_) | RustValueData::DwordBigEndian(_))
    }

    /// Check if this is a QWORD value
    fn is_qword(&self) -> bool {
        matches!(self.inner, RustValueData::Qword(_))
    }

    /// Check if this is a MultiString value
    fn is_multi_string(&self) -> bool {
        matches!(self.inner, RustValueData::MultiString(_))
    }

    /// Get as string (if applicable)
    fn as_string(&self) -> PyResult<String> {
        match &self.inner {
            RustValueData::String(s) | RustValueData::ExpandString(s) => Ok(s.clone()),
            _ => Err(PyValueError::new_err("Not a string value")),
        }
    }

    /// Get as binary data (if applicable)
    fn as_binary<'py>(&self, py: Python<'py>) -> PyResult<&'py PyBytes> {
        match &self.inner {
            RustValueData::Binary(b) => Ok(PyBytes::new(py, b)),
            _ => Err(PyValueError::new_err("Not a binary value")),
        }
    }

    /// Get as DWORD (if applicable)
    fn as_dword(&self) -> PyResult<u32> {
        match &self.inner {
            RustValueData::Dword(d) => Ok(*d),
            RustValueData::DwordBigEndian(d) => Ok(*d),
            _ => Err(PyValueError::new_err("Not a DWORD value")),
        }
    }

    /// Get as QWORD (if applicable)
    fn as_qword(&self) -> PyResult<u64> {
        match &self.inner {
            RustValueData::Qword(q) => Ok(*q),
            _ => Err(PyValueError::new_err("Not a QWORD value")),
        }
    }

    /// Get as multi-string (if applicable)
    fn as_multi_string(&self) -> PyResult<Vec<String>> {
        match &self.inner {
            RustValueData::MultiString(strings) => Ok(strings.clone()),
            _ => Err(PyValueError::new_err("Not a multi-string value")),
        }
    }

    fn __repr__(&self) -> String {
        self.inner.to_string()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Python wrapper for RegistryValue
/// 
/// Stores owned data to avoid lifetime issues with PyO3.
#[pyclass(name = "RegistryValue")]
pub struct PyRegistryValue {
    name: String,
    data_type: RustValueType,
    raw_data: Vec<u8>,
}

#[pymethods]
impl PyRegistryValue {
    /// Get the value name
    fn name(&self) -> &str {
        &self.name
    }

    /// Get the value type
    fn value_type(&self) -> PyValueType {
        PyValueType {
            inner: self.data_type,
        }
    }

    /// Get the parsed value data
    fn data(&self) -> PyResult<PyValueData> {
        RustValueData::parse(&self.raw_data, self.data_type, 0)
            .map(|d| PyValueData { inner: d })
            .map_err(registry_error_to_py)
    }

    /// Get the raw value data as bytes
    fn raw_data<'py>(&self, py: Python<'py>) -> &'py PyBytes {
        PyBytes::new(py, &self.raw_data)
    }

    /// Get the data size in bytes
    fn data_size(&self) -> usize {
        self.raw_data.len()
    }

    fn __repr__(&self) -> String {
        format!("RegistryValue(name='{}')", self.name)
    }
}

/// Python wrapper for RegistryKey
/// 
/// Stores Arc<Hive> and offset to avoid lifetime issues with PyO3.
#[pyclass(name = "RegistryKey")]
pub struct PyRegistryKey {
    hive: Arc<RustHive>,
    offset: u32,
    name: String,
    subkey_count: u32,
    value_count: u32,
}

#[pymethods]
impl PyRegistryKey {
    /// Get the key name
    fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of subkeys
    fn subkey_count(&self) -> u32 {
        self.subkey_count
    }

    /// Get the number of values
    fn value_count(&self) -> u32 {
        self.value_count
    }

    /// Get all subkeys
    fn subkeys(&self, py: Python) -> PyResult<Vec<PyRegistryKey>> {
        let hive = Arc::clone(&self.hive);
        let offset = self.offset;
        
        // Release GIL during Rust operations with panic protection
        let subkeys_data = py.allow_threads(move || {
            // Catch any panics and convert to Python exceptions
            catch_unwind(AssertUnwindSafe(|| {
                let key = hive.get_key(offset)?;
                let subkeys = key.subkeys()?;
                
                let mut result = Vec::new();
                for subkey in subkeys {
                    let name = subkey.name()?;
                    let subkey_count = subkey.subkey_count()?;
                    let value_count = subkey.value_count()?;
                    
                    result.push((subkey.offset, name, subkey_count, value_count));
                }
                
                Ok::<_, RegistryError>(result)
            }))
            .map_err(|panic_err| {
                // Convert panic to Python exception
                let panic_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                    format!("Rust panic: {}", s)
                } else if let Some(s) = panic_err.downcast_ref::<String>() {
                    format!("Rust panic: {}", s)
                } else {
                    "Rust panic: unknown error".to_string()
                };
                PyRuntimeError::new_err(panic_msg)
            })?
            .map_err(registry_error_to_py)
        })?;
        
        // Convert to Python objects (with GIL held)
        Ok(subkeys_data.into_iter().map(|(offset, name, subkey_count, value_count)| {
            PyRegistryKey {
                hive: Arc::clone(&self.hive),
                offset,
                name,
                subkey_count,
                value_count,
            }
        }).collect())
    }

    /// Get all values
    fn values(&self, py: Python) -> PyResult<Vec<PyRegistryValue>> {
        let hive = Arc::clone(&self.hive);
        let offset = self.offset;
        
        // Release GIL during Rust operations with panic protection
        let values_data = py.allow_threads(move || {
            // Catch any panics and convert to Python exceptions
            catch_unwind(AssertUnwindSafe(|| {
                let key = hive.get_key(offset)?;
                let values = key.values()?;
                
                let mut result = Vec::new();
                for value in values {
                    let name = value.name().to_string();
                    let data_type = value.data_type();
                    // Use unwrap_or_default to return empty vec if raw_data fails
                    // This ensures all values are returned, even if data can't be read
                    let raw_data = value.raw_data().unwrap_or_default();
                    
                    result.push((name, data_type, raw_data));
                }
                
                Ok::<_, RegistryError>(result)
            }))
            .map_err(|panic_err| {
                let panic_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                    format!("Rust panic: {}", s)
                } else if let Some(s) = panic_err.downcast_ref::<String>() {
                    format!("Rust panic: {}", s)
                } else {
                    "Rust panic: unknown error".to_string()
                };
                PyRuntimeError::new_err(panic_msg)
            })?
            .map_err(registry_error_to_py)
        })?;
        
        // Convert to Python objects (with GIL held)
        Ok(values_data.into_iter().map(|(name, data_type, raw_data)| {
            PyRegistryValue {
                name,
                data_type,
                raw_data,
            }
        }).collect())
    }

    /// Get a specific value by name
    fn value(&self, name: &str, py: Python) -> PyResult<PyRegistryValue> {
        let hive = Arc::clone(&self.hive);
        let offset = self.offset;
        let name_owned = name.to_string();
        
        // Release GIL during Rust operations with panic protection
        let (value_name, data_type, raw_data) = py.allow_threads(move || {
            catch_unwind(AssertUnwindSafe(|| {
                let key = hive.get_key(offset)?;
                let value = key.value(&name_owned)?;
                
                let value_name = value.name().to_string();
                let data_type = value.data_type();
                let raw_data = value.raw_data()?;
                
                Ok::<_, RegistryError>((value_name, data_type, raw_data))
            }))
            .map_err(|panic_err| {
                let panic_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                    format!("Rust panic: {}", s)
                } else if let Some(s) = panic_err.downcast_ref::<String>() {
                    format!("Rust panic: {}", s)
                } else {
                    "Rust panic: unknown error".to_string()
                };
                PyRuntimeError::new_err(panic_msg)
            })?
            .map_err(registry_error_to_py)
        })?;
        
        Ok(PyRegistryValue {
            name: value_name,
            data_type,
            raw_data,
        })
    }

    /// Get the last write timestamp as Unix timestamp (seconds since epoch)
    fn last_written_timestamp(&self) -> PyResult<Option<i64>> {
        // Note: Cannot use py.allow_threads() here due to RefCell in Hive not being Sync
        // This is a quick operation so GIL impact is minimal
        let key = self.hive.get_key(self.offset)
            .map_err(registry_error_to_py)?;
        let key_node = key.debug_key_node();
        
        // Convert Windows FILETIME to Unix timestamp
        // FILETIME is 100-nanosecond intervals since 1601-01-01
        // Unix epoch is 1970-01-01, difference is 11644473600 seconds
        const FILETIME_UNIX_DIFF: i64 = 11644473600;
        
        if key_node.last_written == 0 {
            return Ok(None);
        }
        
        let seconds = (key_node.last_written / 10_000_000) as i64 - FILETIME_UNIX_DIFF;
        Ok(Some(seconds))
    }

    fn __repr__(&self) -> String {
        format!(
            "RegistryKey(name='{}', subkeys={}, values={})",
            self.name,
            self.subkey_count,
            self.value_count
        )
    }
}

/// Python wrapper for Hive
#[pyclass(name = "Hive")]
pub struct PyHive {
    inner: Arc<RustHive>,
}

#[pymethods]
impl PyHive {
    /// Open a registry hive file
    #[staticmethod]
    fn open(path: &str, py: Python) -> PyResult<PyHive> {
        // Release GIL during file I/O and parsing
        let hive = py.allow_threads(|| {
            RustHive::open(path)
        }).map_err(registry_error_to_py)?;
        
        Ok(PyHive { inner: Arc::new(hive) })
    }

    /// Open a registry hive with transaction logs applied
    #[staticmethod]
    fn open_with_logs(
        hive_path: &str,
        log1_path: Option<&str>,
        log2_path: Option<&str>,
        py: Python,
    ) -> PyResult<PyHive> {
        // Release GIL during file I/O and parsing
        let hive = py.allow_threads(|| {
            RustHive::open_with_logs(hive_path, log1_path, log2_path)
        }).map_err(registry_error_to_py)?;
        
        Ok(PyHive { inner: Arc::new(hive) })
    }

    /// Get the base block (header) information
    fn base_block(&self) -> PyBaseBlock {
        PyBaseBlock {
            inner: self.inner.base_block().clone(),
        }
    }

    /// Get the root key of the hive
    fn root_key(&self, py: Python) -> PyResult<PyRegistryKey> {
        // Release GIL during Rust operations
        let (offset, name, subkey_count, value_count) = py.allow_threads(|| {
            let key = self.inner.root_key()
                .map_err(registry_error_to_py)?;
            let name = key.name()
                .map_err(registry_error_to_py)?;
            let subkey_count = key.subkey_count()
                .map_err(registry_error_to_py)?;
            let value_count = key.value_count()
                .map_err(registry_error_to_py)?;
            
            Ok::<_, PyErr>((key.offset, name, subkey_count, value_count))
        })?;
        
        Ok(PyRegistryKey {
            hive: Arc::clone(&self.inner),
            offset,
            name,
            subkey_count,
            value_count,
        })
    }

    /// Get all hbin headers
    fn hbins(&self, py: Python) -> PyResult<Vec<PyHbinHeader>> {
        // Release GIL during iteration
        let hbins = py.allow_threads(|| {
            let mut result = Vec::new();
            for hbin_result in self.inner.hbins() {
                let hbin = hbin_result.map_err(registry_error_to_py)?;
                result.push(hbin);
            }
            Ok::<_, PyErr>(result)
        })?;
        
        Ok(hbins.into_iter().map(|h| PyHbinHeader { inner: h }).collect())
    }

    /// Save the hive to a file
    fn save(&self, path: &str, py: Python) -> PyResult<()> {
        // Release GIL during file I/O
        py.allow_threads(|| {
            self.inner.save(path)
        }).map_err(registry_error_to_py)
    }

    fn __repr__(&self) -> String {
        let base_block = self.inner.base_block();
        format!(
            "Hive(root_offset={:#x}, hive_length={:#x})",
            base_block.root_cell_offset, base_block.hive_length
        )
    }
}

/// Python module definition
#[pymodule]
fn reg_parser(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyHive>()?;
    m.add_class::<PyRegistryKey>()?;
    m.add_class::<PyRegistryValue>()?;
    m.add_class::<PyValueData>()?;
    m.add_class::<PyValueType>()?;
    m.add_class::<PyBaseBlock>()?;
    m.add_class::<PyHbinHeader>()?;
    
    // Add version constant
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    Ok(())
}
