# GitHub Actions Workflows

This directory contains GitHub Actions workflows for continuous integration and deployment.

## Workflows

### 1. CI (`ci.yml`)

**Triggers:**
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop` branches

**What it does:**
- Tests the Rust code on Linux, Windows, and macOS
- Runs tests with stable and beta Rust toolchains
- Checks code formatting with `rustfmt`
- Runs `clippy` linter
- Builds and tests with the Python feature flag
- Runs benchmarks (compilation check)

### 2. Python Package Publishing (`python-publish.yml`)

**Triggers:**
- Push of version tags (e.g., `v0.1.0`)
- Manual workflow dispatch (for testing)

**What it does:**
1. **Build wheels** for multiple platforms:
   - Linux: x86_64, aarch64
   - Windows: x64, x86
   - macOS: x86_64 (Intel), aarch64 (Apple Silicon)

2. **Build source distribution** (sdist)

3. **Test wheels** on all platforms with Python 3.8-3.12

4. **Publish to PyPI** using trusted publishing (OIDC)

## Setup Instructions

### 1. PyPI Trusted Publishing

To enable automatic publishing to PyPI, you need to configure trusted publishing:

1. Go to [PyPI](https://pypi.org/) and log in
2. Navigate to your account settings
3. Go to "Publishing" → "Add a new publisher"
4. Fill in the details:
   - **PyPI Project Name**: `reg-parser`
   - **Owner**: Your GitHub username or organization
   - **Repository name**: `reg-parser`
   - **Workflow name**: `python-publish.yml`
   - **Environment name**: `pypi`

This allows GitHub Actions to publish to PyPI without storing credentials as secrets.

### 2. GitHub Environment (Optional but Recommended)

Create a GitHub environment for additional protection:

1. Go to your repository settings
2. Navigate to "Environments"
3. Create a new environment named `pypi`
4. Add protection rules (optional):
   - Required reviewers
   - Wait timer
   - Deployment branches (only tags matching `v*`)

### 3. Test PyPI (Optional)

To test the publishing workflow without affecting the real PyPI:

1. Set up trusted publishing on [Test PyPI](https://test.pypi.org/)
2. Manually trigger the workflow:
   - Go to Actions → "Build and Publish Python Package"
   - Click "Run workflow"
   - Check "Publish to Test PyPI instead of PyPI"
   - Click "Run workflow"

## Publishing a New Version

To publish a new version to PyPI:

1. **Update version numbers:**
   ```bash
   # Update version in Cargo.toml
   # Update version in pyproject.toml
   ```

2. **Commit and push changes:**
   ```bash
   git add Cargo.toml pyproject.toml
   git commit -m "Bump version to 0.1.1"
   git push
   ```

3. **Create and push a tag:**
   ```bash
   git tag v0.1.1
   git push origin v0.1.1
   ```

4. **Monitor the workflow:**
   - Go to the Actions tab in your GitHub repository
   - Watch the "Build and Publish Python Package" workflow
   - Once complete, your package will be available on PyPI

## Manual Testing

To manually test the wheels before publishing:

1. Download artifacts from a workflow run:
   - Go to Actions → Select a workflow run
   - Scroll to "Artifacts"
   - Download the wheels

2. Install and test locally:
   ```bash
   pip install path/to/downloaded/wheel.whl
   python -c "import reg_parser; print(reg_parser.__version__)"
   ```

## Troubleshooting

### Build Failures

**Linux ARM64 builds fail:**
- QEMU emulation can be slow and may timeout
- Consider using native ARM64 runners if available

**Windows builds fail:**
- Check that Rust targets are correctly specified
- Ensure Python architecture matches Rust target

**macOS builds fail:**
- Universal2 wheels require building for both architectures
- Ensure Rust toolchain supports the target

### Publishing Failures

**"Trusted publishing not configured":**
- Verify PyPI trusted publishing is set up correctly
- Check that the workflow name and environment match

**"File already exists":**
- The version already exists on PyPI
- Bump the version number and create a new tag

**"Invalid distribution":**
- Check that wheel filenames follow PEP conventions
- Verify that the package builds correctly locally

### Testing Failures

**Import errors:**
- Check that the wheel is compatible with the Python version
- Verify that all dependencies are included

**Test failures:**
- Review test output in the workflow logs
- Run tests locally to reproduce the issue

## Caching

The workflows use GitHub Actions caching to speed up builds:

- **Cargo registry and index**: Cached per OS
- **Cargo build artifacts**: Cached per OS and Cargo.lock hash
- **Rust toolchains**: Cached by `rust-toolchain` action

Caches are automatically invalidated when dependencies change.

## Security

- **No secrets required**: Uses OIDC trusted publishing
- **Environment protection**: Optional approval gates
- **Read-only permissions**: Workflows have minimal permissions
- **Artifact retention**: Artifacts are kept for 90 days by default

## Cost Considerations

- **Linux builds**: Free on public repositories
- **Windows builds**: Free on public repositories
- **macOS builds**: Free on public repositories
- **ARM64 builds**: May use more minutes due to QEMU emulation

For private repositories, check GitHub Actions pricing.

## Future Improvements

Potential enhancements:

- [ ] Add code coverage reporting
- [ ] Generate and publish documentation
- [ ] Create GitHub releases automatically
- [ ] Add changelog generation
- [ ] Implement semantic versioning automation
- [ ] Add performance regression testing
- [ ] Build universal2 wheels for macOS
- [ ] Add support for musl Linux builds
