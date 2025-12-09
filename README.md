# Windows Registry Hive Parser

A high-performance, production-grade Windows Registry hive parser written in Rust.

[![Crates.io](https://img.shields.io/crates/v/reg-parser.svg)](https://crates.io/crates/reg-parser)
[![Documentation](https://docs.rs/reg-parser/badge.svg)](https://docs.rs/reg-parser)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- **🚀 High Performance**: Zero-copy parsing using memory-mapped files
- **📦 Complete Support**: Handles all common Windows registry hive formats
  - SYSTEM, SOFTWARE, SAM, SECURITY
  - NTUSER.DAT, UsrClass.dat
  - Amcache.hve, and more
  - **Big data blocks** (values > 16KB) fully supported
- **🔒 Type-Safe**: Strong typing for registry values and structures
- **⚡ Lazy Evaluation**: Parses structures only when accessed
- **🛡️ Robust Error Handling**: Comprehensive error types for debugging
- **🧪 Well-Tested**: Extensive test suite using real registry hives
- **📚 Well-Documented**: Complete API documentation with examples
- **🐍 Python Bindings**: High-performance Python bindings available

## Installation

### Rust

Add this to your `Cargo.toml`:

```toml
[dependencies]
reg-parser = "0.1"
```

### Python

```bash
# Install from source (requires Rust toolchain)
pip install maturin
git clone https://github.com/ac-rn/reg-parser.git
cd reg-parser
maturin develop --release --features python

# Or use the build script
./build_python.sh release  # Linux/macOS
.\build_python.ps1 release # Windows
```

See [PYTHON_BINDINGS.md](PYTHON_BINDINGS.md) and [python/README.md](python/README.md) for detailed Python documentation.

## Quick Start

### Rust

```rust
use reg_parser::Hive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open a registry hive
    let mut hive = Hive::open("SYSTEM")?;

    // Get the root key
    let mut root = hive.root_key()?;
    println!("Root key: {}", root.name()?);

    // Enumerate subkeys
    for mut subkey in root.subkeys()? {
        println!("  Subkey: {}", subkey.name()?);
    }

    // Enumerate values
    for value in root.values()? {
        println!("  Value: {} = {}", value.name(), value.data()?.to_string());
    }

    Ok(())
}
```

### Python

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

## Architecture

### Binary Layout

Registry hives follow this structure:

```
[Base Block - 4KB]
  - Signature: "regf"
  - Version, timestamps, root offset
  - Checksum

[Hive Bins - variable size, 4KB aligned]
  [Hbin Header - 32 bytes]
    - Signature: "hbin"
    - Offset, size
  
  [Cells - variable size]
    [Cell Size - 4 bytes, negative if allocated]
    [Cell Data]
      - Key nodes (nk)
      - Value keys (vk)
      - Subkey lists (lf/lh/li/ri)
      - Security descriptors (sk)
```

### Module Structure

- **`header`**: Base block (regf) parsing
- **`hbin`**: Hive bin block parsing
- **`cell`**: Cell type definitions
- **`key`**: Key node (nk) structures
- **`value`**: Value key (vk) structures and data types
- **`subkey_list`**: Subkey list parsing (lf/lh/li/ri)
- **`hive`**: Main hive parser with memory-mapped I/O
- **`error`**: Comprehensive error types
- **`utils`**: Helper functions for binary parsing

## Examples

### Accessing Specific Values

```rust
use reg_parser::{Hive, ValueData};

let mut hive = Hive::open("SOFTWARE")?;
let mut root = hive.root_key()?;

// Navigate to a specific key
let mut subkeys = root.subkeys()?;
let mut microsoft_key = subkeys.into_iter()
    .find(|k| k.name().unwrap_or("") == "Microsoft")
    .ok_or("Microsoft key not found")?;

// Get a specific value
let value = microsoft_key.value("SomeValue")?;
match value.data()? {
    ValueData::String(s) => println!("String value: {}", s),
    ValueData::Dword(d) => println!("DWORD value: {}", d),
    ValueData::Binary(b) => println!("Binary value: {} bytes", b.len()),
    _ => println!("Other type"),
}
```

### Deep Traversal

```rust
use reg_parser::Hive;

fn traverse_keys(key: &mut reg_parser::RegistryKey, depth: usize) -> Result<(), Box<dyn std::error::Error>> {
    let indent = "  ".repeat(depth);
    println!("{}{}", indent, key.name()?);

    // Print values
    for value in key.values()? {
        println!("{}  {} = {}", indent, value.name(), value.data()?.to_string());
    }

    // Recurse into subkeys
    for mut subkey in key.subkeys()? {
        traverse_keys(&mut subkey, depth + 1)?;
    }

    Ok(())
}

let mut hive = Hive::open("SYSTEM")?;
let mut root = hive.root_key()?;
traverse_keys(&mut root, 0)?;
```

### Iterating Over Hive Bins

```rust
use reg_parser::Hive;

let hive = Hive::open("SYSTEM")?;

for hbin_result in hive.hbins() {
    let hbin = hbin_result?;
    println!("Hbin at offset {:#x}, size {:#x}", hbin.offset, hbin.size);
}
```

## Performance

The parser is designed for maximum performance:

- **Memory-mapped I/O**: Zero-copy access to hive data
- **Lazy parsing**: Structures are parsed only when accessed
- **Caching**: Parsed key nodes are cached to avoid redundant work
- **Minimal allocations**: Uses slices and references where possible

### Benchmarks

Run benchmarks with:

```bash
cargo bench
```

Typical performance on modern hardware:
- Open hive: ~1-5ms
- Root key access: ~10-50μs
- Enumerate subkeys: ~100-500μs
- Deep traversal (1000 keys): ~5-20ms

## Testing

The library includes comprehensive tests using real registry hive files:

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration

# Run with output
cargo test -- --nocapture
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

## Transaction Log Support

The parser now supports applying transaction logs (.LOG1, .LOG2) to recover uncommitted changes:

```rust
use reg_parser::Hive;

// Open hive with transaction logs applied
let hive = Hive::open_with_logs(
    "SYSTEM",
    Some("SYSTEM.LOG1"),
    Some("SYSTEM.LOG2")
)?;

// Save the cleaned hive
hive.save("SYSTEM_cleaned")?;
```

### Applying Transaction Logs

```rust
// Apply a single transaction log
let base_hive = Hive::open("SYSTEM")?;
let cleaned_hive = base_hive.apply_transaction_log("SYSTEM.LOG1")?;

// Save the result
cleaned_hive.save("SYSTEM_cleaned")?;
```

## Limitations and Future Work

### Current Limitations

- **Write operations**: No support for creating/modifying keys and values (read-only library)
  - Can save existing hives after applying transaction logs
  - Cannot create new keys or modify existing values

### Planned Features

- [ ] Security descriptor parsing and interpretation
- [ ] Class name extraction
- [ ] Full write support (create/modify keys and values)
- [ ] Parallel parsing for multi-threaded workloads

## Binary Format Reference

This parser implements the Windows Registry hive format as documented in:

- [Windows Registry File Format Specification](https://github.com/msuhanov/regf/blob/master/Windows%20registry%20file%20format%20specification.md)
- Microsoft's internal documentation
- Reverse engineering and forensic analysis

### Key Structures

#### Base Block (regf)
- **Offset**: 0x0000
- **Size**: 4096 bytes (0x1000)
- **Signature**: "regf" (0x66676572)

#### Hive Bin (hbin)
- **Alignment**: 4KB (0x1000)
- **Header Size**: 32 bytes (0x20)
- **Signature**: "hbin" (0x6E696268)

#### Cell Types
- **nk** (Key Node): Registry key
- **vk** (Value Key): Registry value
- **sk** (Security): Security descriptor
- **lf** (Fast Leaf): Subkey list with name hints
- **lh** (Hash Leaf): Subkey list with name hashes
- **li** (Index Leaf): Simple subkey list
- **ri** (Index Root): List of subkey lists
- **db** (Data Block): Big data block

### Development Setup

```bash
# Clone the repository
git clone https://github.com/ac-rn/reg-parser.git
cd reg-parser

# Build the project
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open
```

## License

This project is dual-licensed under:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

You may choose either license for your use.
