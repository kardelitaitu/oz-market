#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

// 1. Determine the binary name based on the OS
const platform = os.platform();
let binaryName = 'oz-market-mcp';
if (platform === 'win32') {
  binaryName += '.exe';
}

// 2. Resolve the path to the binary
// Check if running in development (local workspace) or production (installed npm package)
let binaryPath = path.join(__dirname, '../binaries', platform, binaryName);

if (!fs.existsSync(binaryPath)) {
  // Fallback to local cargo release or debug builds for easy development testing
  const targetDir = path.join(__dirname, '../../../target');
  const releasePath = path.join(targetDir, 'release', binaryName);
  const debugPath = path.join(targetDir, 'debug', binaryName);
  
  if (fs.existsSync(releasePath)) {
    binaryPath = releasePath;
  } else if (fs.existsSync(debugPath)) {
    binaryPath = debugPath;
  } else {
    console.error(`Error: Could not find oz-market-mcp binary at ${binaryPath} or in target folder.`);
    process.exit(1);
  }
}

// 3. Spawn the Rust process, inheriting environment variables and stdio streams
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env
});

child.on('close', (code) => {
  process.exit(code || 0);
});

child.on('error', (err) => {
  console.error('Failed to start the MCP server process:', err);
  process.exit(1);
});
