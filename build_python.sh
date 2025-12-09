#!/bin/bash
# Script to build Python bindings for reg-parser
# Usage: ./build_python.sh [dev|release]

set -e

BUILD_TYPE="${1:-dev}"

echo "Building Python bindings for reg-parser..."
echo ""

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 not found in PATH"
    echo "Please install Python 3.7 or later"
    exit 1
fi

# Check Python version
PYTHON_VERSION=$(python3 --version)
echo "Found: $PYTHON_VERSION"

# Check if Rust is available
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found in PATH"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check Rust version
RUST_VERSION=$(cargo --version)
echo "Found: $RUST_VERSION"
echo ""

# Check if maturin is installed
if ! command -v maturin &> /dev/null; then
    echo "Maturin not found. Installing..."
    python3 -m pip install maturin
fi

# Build based on type
if [ "$BUILD_TYPE" = "dev" ]; then
    echo "Building in development mode..."
    maturin develop --features python
elif [ "$BUILD_TYPE" = "release" ]; then
    echo "Building in release mode..."
    maturin develop --release --features python
else
    echo "Error: Invalid build type. Use 'dev' or 'release'"
    exit 1
fi

echo ""
echo "Build successful!"
echo ""
echo "You can now use the Python bindings:"
echo "  python3 -c \"import reg_parser; print(reg_parser.__version__)\""
echo ""
echo "Run examples:"
echo "  python3 python/examples/basic_usage.py test_data/SYSTEM"
echo ""
