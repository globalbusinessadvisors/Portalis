#!/usr/bin/env node

/**
 * Post-install script for Portalis npm package
 * Downloads the appropriate pre-built binary for the current platform
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const PACKAGE_VERSION = require('../package.json').version;
const REPO = 'portalis/portalis';
const BINARY_NAME = 'portalis';

// Platform mapping
const PLATFORM_MAP = {
  'darwin-x64': 'macos-x86_64',
  'darwin-arm64': 'macos-aarch64',
  'linux-x64': 'linux-x86_64',
  'linux-arm64': 'linux-aarch64',
  'win32-x64': 'windows-x86_64',
};

function getPlatform() {
  const type = process.platform;
  const arch = process.arch;
  const key = `${type}-${arch}`;

  const platform = PLATFORM_MAP[key];
  if (!platform) {
    console.error(`⚠️  Unsupported platform: ${type}-${arch}`);
    console.error('Supported platforms:');
    Object.keys(PLATFORM_MAP).forEach(k => console.error(`  - ${k}`));
    console.error('\nYou can build from source:');
    console.error('  git clone https://github.com/portalis/portalis.git');
    console.error('  cd portalis');
    console.error('  cargo build --release --bin portalis');
    process.exit(1);
  }

  return platform;
}

function getBinaryExtension() {
  return process.platform === 'win32' ? '.exe' : '';
}

function downloadBinary(url, dest) {
  return new Promise((resolve, reject) => {
    console.log(`📥 Downloading from ${url}...`);

    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        return downloadBinary(response.headers.location, dest)
          .then(resolve)
          .catch(reject);
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
        return;
      }

      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

async function install() {
  console.log('🚀 Installing Portalis...\n');

  const platform = getPlatform();
  const binDir = path.join(__dirname, '..', 'bin');
  const ext = getBinaryExtension();
  const binaryName = BINARY_NAME + ext;
  const binaryPath = path.join(binDir, binaryName);

  // Create bin directory if it doesn't exist
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  // Try to download from GitHub Releases
  const downloadUrl = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${BINARY_NAME}-${platform}${ext}`;

  try {
    await downloadBinary(downloadUrl, binaryPath);

    // Make binary executable on Unix systems
    if (process.platform !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log(`✅ Successfully installed Portalis ${PACKAGE_VERSION}`);
    console.log(`📍 Binary location: ${binaryPath}\n`);
    console.log('Get started with:');
    console.log('  portalis --help');
    console.log('  portalis convert your_script.py');

  } catch (error) {
    console.error(`\n❌ Failed to download binary: ${error.message}\n`);
    console.error('Fallback options:');
    console.error('1. Install via cargo (requires Rust):');
    console.error('   cargo install portalis\n');
    console.error('2. Build from source:');
    console.error('   git clone https://github.com/portalis/portalis.git');
    console.error('   cd portalis');
    console.error('   cargo build --release --bin portalis\n');
    console.error('3. Report this issue:');
    console.error(`   https://github.com/${REPO}/issues\n`);

    // Don't fail the install, just warn
    process.exit(0);
  }
}

// Run installation
install().catch((err) => {
  console.error('Installation error:', err);
  process.exit(0); // Don't fail npm install
});
