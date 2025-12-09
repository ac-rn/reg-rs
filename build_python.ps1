# PowerShell script to build Python bindings for reg-parser
# Usage: .\build_python.ps1 [dev|release]

param(
    [Parameter(Position=0)]
    [ValidateSet("dev", "release")]
    [string]$BuildType = "dev"
)

Write-Host "Building Python bindings for reg-parser..." -ForegroundColor Green
Write-Host ""

# Check if Python is available
if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    Write-Host "Error: Python not found in PATH" -ForegroundColor Red
    Write-Host "Please install Python 3.7 or later from https://www.python.org/" -ForegroundColor Yellow
    exit 1
}

# Check Python version
$pythonVersion = python --version 2>&1
Write-Host "Found: $pythonVersion" -ForegroundColor Cyan

# Check if Rust is available
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: Rust/Cargo not found in PATH" -ForegroundColor Red
    Write-Host "Please install Rust from https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

# Check Rust version
$rustVersion = cargo --version
Write-Host "Found: $rustVersion" -ForegroundColor Cyan
Write-Host ""

# Check if maturin is installed
if (-not (Get-Command maturin -ErrorAction SilentlyContinue)) {
    Write-Host "Maturin not found. Installing..." -ForegroundColor Yellow
    python -m pip install maturin
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Error: Failed to install maturin" -ForegroundColor Red
        exit 1
    }
}

# Build based on type
if ($BuildType -eq "dev") {
    Write-Host "Building in development mode..." -ForegroundColor Green
    maturin develop --features python
} else {
    Write-Host "Building in release mode..." -ForegroundColor Green
    maturin develop --release --features python
}

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "Build successful!" -ForegroundColor Green
    Write-Host ""
    Write-Host "You can now use the Python bindings:" -ForegroundColor Cyan
    Write-Host "  python -c `"import reg_parser; print(reg_parser.__version__)`"" -ForegroundColor White
    Write-Host ""
    Write-Host "Run examples:" -ForegroundColor Cyan
    Write-Host "  python python/examples/basic_usage.py test_data/SYSTEM" -ForegroundColor White
    Write-Host ""
} else {
    Write-Host ""
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}
