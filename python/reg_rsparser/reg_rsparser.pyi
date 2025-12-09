"""
Type stubs for reg_rsparser Python bindings.

This module provides Python bindings for the high-performance Windows Registry
hive parser written in Rust.
"""

from typing import Optional, List

__version__: str

class BaseBlock:
    """
    Represents the base block (header) of a registry hive.
    
    The base block contains metadata about the hive including signatures,
    sequence numbers, and the root key offset.
    """
    
    @property
    def signature(self) -> str:
        """Get the signature (should be "regf")."""
        ...
    
    @property
    def primary_sequence(self) -> int:
        """Get the primary sequence number."""
        ...
    
    @property
    def secondary_sequence(self) -> int:
        """Get the secondary sequence number."""
        ...
    
    @property
    def root_cell_offset(self) -> int:
        """Get the root cell offset."""
        ...
    
    @property
    def hive_bins_data_size(self) -> int:
        """Get the hive bins data size."""
        ...
    
    @property
    def file_name(self) -> Optional[str]:
        """Get the file name (if present)."""
        ...
    
    def __repr__(self) -> str: ...

class HbinHeader:
    """
    Represents a hive bin (hbin) header.
    
    Hive bins are 4KB-aligned blocks that contain cells.
    """
    
    @property
    def signature(self) -> str:
        """Get the signature (should be "hbin")."""
        ...
    
    @property
    def offset(self) -> int:
        """Get the offset from the start of the hive bins data."""
        ...
    
    @property
    def size(self) -> int:
        """Get the size of this hbin."""
        ...
    
    def __repr__(self) -> str: ...

class ValueType:
    """
    Represents a registry value type.
    
    Common types include REG_SZ, REG_DWORD, REG_BINARY, etc.
    """
    
    def type_id(self) -> int:
        """Get the numeric type ID."""
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class ValueData:
    """
    Represents parsed registry value data.
    
    This class provides type-safe access to registry values with
    appropriate conversion methods.
    """
    
    def is_none(self) -> bool:
        """Check if this is a None value."""
        ...
    
    def is_string(self) -> bool:
        """Check if this is a String value."""
        ...
    
    def is_binary(self) -> bool:
        """Check if this is a Binary value."""
        ...
    
    def is_dword(self) -> bool:
        """Check if this is a DWORD value."""
        ...
    
    def is_qword(self) -> bool:
        """Check if this is a QWORD value."""
        ...
    
    def is_multi_string(self) -> bool:
        """Check if this is a MultiString value."""
        ...
    
    def as_string(self) -> str:
        """
        Get as string (if applicable).
        
        Raises:
            ValueError: If the value is not a string type.
        """
        ...
    
    def as_binary(self) -> bytes:
        """
        Get as binary data (if applicable).
        
        Raises:
            ValueError: If the value is not a binary type.
        """
        ...
    
    def as_dword(self) -> int:
        """
        Get as DWORD (if applicable).
        
        Raises:
            ValueError: If the value is not a DWORD type.
        """
        ...
    
    def as_qword(self) -> int:
        """
        Get as QWORD (if applicable).
        
        Raises:
            ValueError: If the value is not a QWORD type.
        """
        ...
    
    def as_multi_string(self) -> List[str]:
        """
        Get as multi-string (if applicable).
        
        Raises:
            ValueError: If the value is not a multi-string type.
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class RegistryValue:
    """
    Represents a registry value.
    
    A value has a name, type, and data. Values are contained within registry keys.
    """
    
    def name(self) -> str:
        """Get the value name."""
        ...
    
    def value_type(self) -> ValueType:
        """Get the value type."""
        ...
    
    def data(self) -> ValueData:
        """
        Get the parsed value data.
        
        Raises:
            IOError: If there's an I/O error reading the data.
            ValueError: If the data is invalid or corrupted.
        """
        ...
    
    def raw_data(self) -> bytes:
        """
        Get the raw value data as bytes.
        
        Raises:
            IOError: If there's an I/O error reading the data.
            ValueError: If the data is invalid or corrupted.
        """
        ...
    
    def data_size(self) -> int:
        """Get the data size in bytes."""
        ...
    
    def __repr__(self) -> str: ...

class RegistryKey:
    """
    Represents a registry key.
    
    A key can contain subkeys and values. Keys form a hierarchical tree structure.
    """
    
    def name(self) -> str:
        """
        Get the key name.
        
        Raises:
            IOError: If there's an I/O error reading the name.
            ValueError: If the name is invalid or corrupted.
        """
        ...
    
    def subkey_count(self) -> int:
        """Get the number of subkeys."""
        ...
    
    def value_count(self) -> int:
        """Get the number of values."""
        ...
    
    def subkeys(self) -> List[RegistryKey]:
        """
        Get all subkeys.
        
        Returns:
            A list of RegistryKey objects representing the subkeys.
        
        Raises:
            IOError: If there's an I/O error reading the subkeys.
            ValueError: If the subkey data is invalid or corrupted.
        """
        ...
    
    def values(self) -> List[RegistryValue]:
        """
        Get all values.
        
        Returns:
            A list of RegistryValue objects representing the values.
        
        Raises:
            IOError: If there's an I/O error reading the values.
            ValueError: If the value data is invalid or corrupted.
        """
        ...
    
    def value(self, name: str) -> RegistryValue:
        """
        Get a specific value by name.
        
        Args:
            name: The name of the value to retrieve.
        
        Returns:
            The RegistryValue with the specified name.
        
        Raises:
            IOError: If there's an I/O error reading the value.
            ValueError: If the value is not found or data is corrupted.
        """
        ...
    
    def last_written_timestamp(self) -> Optional[int]:
        """
        Get the last write timestamp as Unix timestamp (seconds since epoch).
        
        Returns:
            Unix timestamp in seconds, or None if not available.
        """
        ...
    
    def __repr__(self) -> str: ...

class Hive:
    """
    Represents an open registry hive file.
    
    This is the main entry point for parsing Windows Registry hive files.
    The hive uses memory-mapped I/O for efficient zero-copy access.
    """
    
    @staticmethod
    def open(path: str) -> Hive:
        """
        Open a registry hive file.
        
        Args:
            path: Path to the registry hive file.
        
        Returns:
            A Hive object representing the opened hive.
        
        Raises:
            IOError: If the file cannot be opened or read.
            ValueError: If the file is not a valid registry hive.
        """
        ...
    
    @staticmethod
    def open_with_logs(
        hive_path: str,
        log1_path: Optional[str] = None,
        log2_path: Optional[str] = None
    ) -> Hive:
        """
        Open a registry hive with transaction logs applied.
        
        This method applies transaction logs (.LOG1, .LOG2) to recover
        uncommitted changes and produce a clean hive.
        
        Args:
            hive_path: Path to the main registry hive file.
            log1_path: Optional path to the .LOG1 file.
            log2_path: Optional path to the .LOG2 file.
        
        Returns:
            A Hive object with transaction logs applied.
        
        Raises:
            IOError: If any file cannot be opened or read.
            ValueError: If any file is not valid or corrupted.
        """
        ...
    
    def base_block(self) -> BaseBlock:
        """
        Get the base block (header) information.
        
        Returns:
            A BaseBlock object containing hive metadata.
        """
        ...
    
    def root_key(self) -> RegistryKey:
        """
        Get the root key of the hive.
        
        Returns:
            A RegistryKey object representing the root key.
        
        Raises:
            IOError: If there's an I/O error reading the root key.
            ValueError: If the root key data is invalid or corrupted.
        """
        ...
    
    def hbins(self) -> List[HbinHeader]:
        """
        Get all hbin headers.
        
        Returns:
            A list of HbinHeader objects representing all hive bins.
        
        Raises:
            IOError: If there's an I/O error reading the hbins.
            ValueError: If the hbin data is invalid or corrupted.
        """
        ...
    
    def save(self, path: str) -> None:
        """
        Save the hive to a file.
        
        Args:
            path: Path where the hive should be saved.
        
        Raises:
            IOError: If the file cannot be written.
        """
        ...
    
    def __repr__(self) -> str: ...
