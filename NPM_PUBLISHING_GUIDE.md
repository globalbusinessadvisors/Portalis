# NPM Publishing Guide for Portalis

This guide explains the complete npm publishing infrastructure that has been set up for Portalis.

## 📦 What's Been Set Up

### 1. NPM Package Structure

```
portalis/
├── package.json           # NPM package configuration
├── index.js              # Node.js entry point
├── .npmignore            # Files to exclude from npm package
├── bin/
│   ├── portalis          # Unix wrapper script
│   └── portalis.cmd      # Windows wrapper script
└── scripts/
    └── install.js        # Post-install binary download script
```

### 2. GitHub Actions Workflow

**Location:** `.github/workflows/release.yml`

Automatically builds cross-platform binaries and publishes to both npm and crates.io when you push a version tag.

**Supported Platforms:**
- Linux x86_64
- Linux ARM64 (aarch64)
- macOS x86_64 (Intel)
- macOS ARM64 (Apple Silicon)
- Windows x86_64

### 3. Publishing Script

**File:** `publish.sh`

Enhanced to support npm publishing alongside Rust crates and Python packages.

## 🚀 How to Publish

### Prerequisites

1. **GitHub Secrets** (for automated releases):
   - `CARGO_REGISTRY_TOKEN` - Your crates.io token
   - `NPM_TOKEN` - Your npm token
   - `GITHUB_TOKEN` - Automatically provided

2. **Local `.env` file** (for manual publishing):
   ```bash
   CARGO_REGISTRY_TOKEN=crates-io-xxx
   NPM_TOKEN=npm_xxx
   TWINE_USERNAME=__token__
   TWINE_PASSWORD=pypi-xxx
   ```

### Publishing Process

#### Option 1: Automated (Recommended)

```bash
# 1. Update version in Cargo.toml and package.json
# 2. Commit changes
git add -A
git commit -m "chore: Release v0.1.0"

# 3. Create and push tag
git tag v0.1.0
git push origin v0.1.0

# GitHub Actions will automatically:
# - Build binaries for all platforms
# - Create GitHub Release with binaries
# - Publish to npm
# - Publish to crates.io
```

#### Option 2: Manual Publishing

```bash
# 1. Create .env with your tokens
cat > .env << EOF
CARGO_REGISTRY_TOKEN=your_crates_token
NPM_TOKEN=your_npm_token
EOF

# 2. Test with dry-run
./publish.sh --dry-run

# 3. Publish to all registries
./publish.sh

# Or publish only to npm
./publish.sh --skip-rust --skip-python
```

## 📥 How Users Install

### Via npm (Recommended for end-users)

```bash
# Global installation
npm install -g portalis

# Verify
portalis --version
```

**What happens during installation:**
1. npm installs the package
2. `scripts/install.js` runs automatically (postinstall hook)
3. Script detects platform (OS + architecture)
4. Downloads appropriate binary from GitHub Releases
5. Places binary in `bin/` directory
6. Makes it executable (Unix systems)

### Via cargo (For Rust developers)

```bash
cargo install portalis
```

### Via source (For contributors)

```bash
git clone https://github.com/portalis/portalis.git
cd portalis
cargo build --release
```

## 🔧 Configuration

### package.json

```json
{
  "name": "portalis",
  "version": "0.1.0",
  "description": "GPU-accelerated Python to Rust/WASM transpiler",
  "bin": {
    "portalis": "./bin/portalis"
  },
  "scripts": {
    "postinstall": "node scripts/install.js"
  }
}
```

### Platform Detection

The install script automatically detects:
- **Darwin (macOS)**: x64 (Intel) or arm64 (Apple Silicon)
- **Linux**: x64 or arm64
- **Windows**: x64

Unsupported platforms show instructions for building from source.

## 🐛 Troubleshooting

### Binary Download Fails

**Problem:** User reports binary download failure during `npm install`

**Solution:**
1. Ensure GitHub Release exists with version tag (e.g., `v0.1.0`)
2. Verify binaries are uploaded to the release
3. Check binary naming matches pattern:
   - `portalis-linux-x86_64`
   - `portalis-macos-x86_64`
   - `portalis-macos-aarch64`
   - `portalis-windows-x86_64.exe`

### Platform Not Supported

**Problem:** Installation fails on unsupported platform

**Solution:** The install script gracefully handles this and suggests:
```bash
# Build from source
git clone https://github.com/portalis/portalis.git
cd portalis
cargo build --release
```

### Permission Denied (Unix)

**Problem:** Binary not executable

**Solution:**
```bash
chmod +x $(npm root -g)/portalis/bin/portalis
```

## 📊 Publishing Checklist

Before publishing a new version:

- [ ] Update version in `Cargo.toml` (workspace level)
- [ ] Update version in `package.json`
- [ ] Update `CHANGELOG.md`
- [ ] Run tests: `cargo test --workspace`
- [ ] Test npm package locally: `npm pack && npm install -g ./portalis-0.1.0.tgz`
- [ ] Commit changes
- [ ] Create git tag: `git tag v0.1.0`
- [ ] Push tag: `git push origin v0.1.0`
- [ ] Monitor GitHub Actions workflow
- [ ] Verify npm package: `npm view portalis`
- [ ] Verify crates.io: `cargo search portalis`

## 🔐 Security

### Token Management

**Never commit tokens to git!**

- `.env` is in `.gitignore`
- Use GitHub Secrets for CI/CD
- Rotate tokens periodically
- Use scoped tokens with minimal permissions:
  - npm: Automation token
  - crates.io: Publish-only token

### Binary Integrity

Binaries are:
- Built in isolated GitHub Actions runners
- Checksummed automatically by GitHub Releases
- Served over HTTPS from GitHub CDN
- Verified during download (HTTP status checks)

## 📈 Monitoring

### After Publishing

Check these metrics:

1. **npm Downloads:**
   ```bash
   npm view portalis
   ```

2. **GitHub Release:**
   - Verify all 5 platform binaries are attached
   - Check download counts

3. **crates.io Stats:**
   ```bash
   cargo search portalis --limit 1
   ```

4. **Installation Tests:**
   ```bash
   # Test on different platforms
   docker run --rm -it node:18 bash -c "npm install -g portalis && portalis --version"
   ```

## 🎯 Next Steps

1. **Version 0.1.0 (First Release):**
   - Tag and push
   - Monitor automated build
   - Test installation on multiple platforms

2. **Future Improvements:**
   - Add checksums verification
   - Implement auto-update mechanism
   - Create separate platform packages (@portalis/linux-x64, etc.)
   - Add telemetry for usage analytics

## 📚 Resources

- [npm Publishing Docs](https://docs.npmjs.com/packages-and-modules/contributing-packages-to-the-registry)
- [GitHub Actions for Rust](https://github.com/actions-rust-lang/setup-rust-toolchain)
- [Cargo Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)

---

## Summary

✅ **Complete npm infrastructure is ready!**

**To publish:**
```bash
git tag v0.1.0
git push origin v0.1.0
```

**Users install with:**
```bash
npm install -g portalis
```

That's it! 🚀
