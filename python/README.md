# Windows Registry Parser - Python Bindings

High-performance Python bindings for the Windows Registry hive parser, built on Rust for speed and safety.

## Features

- **🚀 High Performance**: Zero-copy parsing using memory-mapped files
- **🔒 Type-Safe**: Strong typing with comprehensive type hints
- **📦 Complete Support**: Handles all common Windows registry hive formats
  - SYSTEM, SOFTWARE, SAM, SECURITY
  - NTUSER.DAT, UsrClass.dat
  - Amcache.hve, and more
- **⚡ Lazy Evaluation**: Parses structures only when accessed
- **🛡️ Robust Error Handling**: Comprehensive error types for debugging
- **🧪 Well-Tested**: Extensive test suite using real registry hives
- **📚 Well-Documented**: Complete API documentation with examples

## Installation

### From PyPI (when published)

```bash
pip install reg-rsparser
```

### From Source

Requirements:
- Python 3.7 or later
- Rust toolchain (for building)
- maturin

```bash
# Install maturin
pip install maturin

# Clone the repository
git clone https://github.com/ac-rn/reg-rs.git
cd reg-rs

# Build and install
maturin develop --release --features python
```

## Quick Start

```python
import reg_parser

# Open a registry hive
hive = reg_parser.Hive.open("SYSTEM")

# Get the root key
root = hive.root_key()
print(f"Root key: {root.name()}")

# Enumerate subkeys
for subkey in root.subkeys():
    print(f"  Subkey: {subkey.name()}")

# Enumerate values
for value in root.values():
    data = value.data()
    print(f"  Value: {value.name()} = {data}")
```

## Examples

### Basic Usage

```python
import reg_parser

# Open a registry hive
hive = reg_parser.Hive.open("test_data/SYSTEM")

# Get base block information
base_block = hive.base_block()
print(f"Signature: {base_block.signature}")
print(f"Root offset: {base_block.root_cell_offset:#x}")

# Get the root key
root = hive.root_key()
print(f"Root: {root.name()}")
print(f"Subkeys: {root.subkey_count()}")
print(f"Values: {root.value_count()}")
```

### Accessing Specific Values

```python
import reg_parser

hive = reg_parser.Hive.open("SOFTWARE")
root = hive.root_key()

# Navigate to a specific key
for subkey in root.subkeys():
    if subkey.name() == "Microsoft":
        # Get a specific value
        try:
            value = subkey.value("SomeValue")
            data = value.data()
            
            if data.is_string():
                print(f"String value: {data.as_string()}")
            elif data.is_dword():
                print(f"DWORD value: {data.as_dword():#x}")
            elif data.is_binary():
                print(f"Binary value: {len(data.as_binary())} bytes")
        except ValueError as e:
            print(f"Value not found: {e}")
```

### Deep Traversal

```python
import reg_parser

def traverse_keys(key, depth=0):
    """Recursively traverse registry keys."""
    indent = "  " * depth
    print(f"{indent}{key.name()}")
    
    # Print values
    for value in key.values():
        try:
            data = value.data()
            print(f"{indent}  {value.name()} = {data}")
        except Exception as e:
            print(f"{indent}  {value.name()} = <error: {e}>")
    
    # Recurse into subkeys
    for subkey in key.subkeys():
        traverse_keys(subkey, depth + 1)

hive = reg_parser.Hive.open("SYSTEM")
root = hive.root_key()
traverse_keys(root)
```

### Working with Different Value Types

```python
import reg_parser

hive = reg_parser.Hive.open("SOFTWARE")
root = hive.root_key()

for value in root.values():
    data = value.data()
    
    if data.is_string():
        print(f"String: {value.name()} = {data.as_string()}")
    
    elif data.is_dword():
        print(f"DWORD: {value.name()} = {data.as_dword():#x}")
    
    elif data.is_qword():
        print(f"QWORD: {value.name()} = {data.as_qword():#x}")
    
    elif data.is_binary():
        binary = data.as_binary()
        print(f"Binary: {value.name()} = {len(binary)} bytes")
    
    elif data.is_multi_string():
        strings = data.as_multi_string()
        print(f"MultiString: {value.name()} = {strings}")
    
    else:
        print(f"Other: {value.name()} = {data}")
```

### Applying Transaction Logs

```python
import reg_parser

# Open hive with transaction logs applied
hive = reg_parser.Hive.open_with_logs(
    "SYSTEM",
    log1_path="SYSTEM.LOG1",
    log2_path="SYSTEM.LOG2"
)

# Save the cleaned hive
hive.save("SYSTEM_cleaned")
```

### Forensic Analysis

```python
import reg_parser
from datetime import datetime

def collect_strings(key, strings):
    """Collect all string values from a key and its subkeys."""
    for value in key.values():
        try:
            data = value.data()
            if data.is_string():
                strings.append((key.name(), value.name(), data.as_string()))
        except Exception:
            pass
    
    for subkey in key.subkeys():
        collect_strings(subkey, strings)

hive = reg_parser.Hive.open("SYSTEM")
root = hive.root_key()

strings = []
collect_strings(root, strings)
print(f"Found {len(strings)} string values")

# Print first 10
for key_name, value_name, value_data in strings[:10]:
    print(f"{key_name}\\{value_name} = {value_data}")
```

### Enumerating Hive Bins

```python
import reg_parser

hive = reg_parser.Hive.open("SYSTEM")

# Get all hbin headers
hbins = hive.hbins()
print(f"Total hbins: {len(hbins)}")

for hbin in hbins:
    print(f"Hbin at offset {hbin.offset:#x}, size {hbin.size:#x}")
```

## API Reference

### Hive

The main entry point for parsing registry hives.

```python
class Hive:
    @staticmethod
    def open(path: str) -> Hive:
        """Open a registry hive file."""
    
    @staticmethod
    def open_with_logs(
        hive_path: str,
        log1_path: Optional[str] = None,
        log2_path: Optional[str] = None
    ) -> Hive:
        """Open a hive with transaction logs applied."""
    
    def base_block(self) -> BaseBlock:
        """Get the base block (header) information."""
    
    def root_key(self) -> RegistryKey:
        """Get the root key of the hive."""
    
    def hbins(self) -> List[HbinHeader]:
        """Get all hbin headers."""
    
    def save(self, path: str) -> None:
        """Save the hive to a file."""
```

### RegistryKey

Represents a registry key with subkeys and values.

```python
class RegistryKey:
    def name(self) -> str:
        """Get the key name."""
    
    def subkey_count(self) -> int:
        """Get the number of subkeys."""
    
    def value_count(self) -> int:
        """Get the number of values."""
    
    def subkeys(self) -> List[RegistryKey]:
        """Get all subkeys."""
    
    def values(self) -> List[RegistryValue]:
        """Get all values."""
    
    def value(self, name: str) -> RegistryValue:
        """Get a specific value by name."""
    
    def last_written_timestamp(self) -> Optional[int]:
        """Get the last write timestamp (Unix timestamp)."""
```

### RegistryValue

Represents a registry value with typed data.

```python
class RegistryValue:
    def name(self) -> str:
        """Get the value name."""
    
    def value_type(self) -> ValueType:
        """Get the value type."""
    
    def data(self) -> ValueData:
        """Get the parsed value data."""
    
    def raw_data(self) -> bytes:
        """Get the raw value data as bytes."""
    
    def data_size(self) -> int:
        """Get the data size in bytes."""
```

### ValueData

Enum representing parsed value data with type-safe accessors.

```python
class ValueData:
    def is_none(self) -> bool: ...
    def is_string(self) -> bool: ...
    def is_binary(self) -> bool: ...
    def is_dword(self) -> bool: ...
    def is_qword(self) -> bool: ...
    def is_multi_string(self) -> bool: ...
    
    def as_string(self) -> str: ...
    def as_binary(self) -> bytes: ...
    def as_dword(self) -> int: ...
    def as_qword(self) -> int: ...
    def as_multi_string(self) -> List[str]: ...
```

## Supported Registry Value Types

- `REG_NONE` - No value type
- `REG_SZ` - Null-terminated string
- `REG_EXPAND_SZ` - String with environment variables
- `REG_BINARY` - Binary data
- `REG_DWORD` - 32-bit little-endian integer
- `REG_DWORD_BIG_ENDIAN` - 32-bit big-endian integer
- `REG_LINK` - Symbolic link
- `REG_MULTI_SZ` - Multiple null-terminated strings
- `REG_QWORD` - 64-bit little-endian integer
- Resource types (list, descriptor, requirements)

## Error Handling

The library uses Python exceptions for error handling:

```python
import reg_parser

try:
    hive = reg_parser.Hive.open("invalid.dat")
except IOError as e:
    print(f"I/O error: {e}")
except ValueError as e:
    print(f"Invalid hive: {e}")
```

Common exceptions:
- `IOError`: File access errors
- `ValueError`: Invalid signatures, corrupted data, invalid offsets
- `RuntimeError`: Other runtime errors

## Performance

The parser is designed for maximum performance:

- **Memory-mapped I/O**: Zero-copy access to hive data
- **Lazy parsing**: Structures are parsed only when accessed
- **Caching**: Parsed key nodes are cached to avoid redundant work
- **Minimal allocations**: Uses slices and references where possible

Typical performance on modern hardware:
- Open hive: ~1-5ms
- Root key access: ~10-50μs
- Enumerate subkeys: ~100-500μs
- Deep traversal (1000 keys): ~5-20ms

## Use Cases

- **Digital Forensics**: Analyze registry hives from disk images
- **System Administration**: Audit registry configurations
- **Security Research**: Investigate malware persistence mechanisms
- **Data Recovery**: Extract data from corrupted registry files
- **Compliance**: Verify system configurations against policies

## Building from Source

### Prerequisites

- Python 3.7 or later
- Rust toolchain (install from https://rustup.rs/)
- maturin (`pip install maturin`)

### Development Build

```bash
# Clone the repository
git clone https://github.com/ac-rn/reg-parser.git
cd reg-parser

# Install in development mode
maturin develop --features python

# Run tests
python -m pytest python/tests/
```

### Release Build

```bash
# Build release wheel
maturin build --release --features python

# Install the wheel
pip install target/wheels/reg_parser-*.whl
```

## Testing

```bash
# Run Python tests
python -m pytest python/tests/ -v

# Run with coverage
python -m pytest python/tests/ --cov=reg_parser --cov-report=html
```

## License

This project is dual-licensed under:

- MIT License ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

You may choose either license for your use.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Support

For questions, issues, or feature requests:
- Open an issue on [GitHub](https://github.com/ac-rn/reg-parser/issues)
- Check the [Rust documentation](https://docs.rs/reg-parser)

## Acknowledgments

- Microsoft for the Windows Registry format
- The forensics community for reverse engineering efforts
- [regf specification](https://github.com/msuhanov/regf) by Maxim Suhanov
