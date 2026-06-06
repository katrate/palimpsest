#!/usr/bin/env node

/**
 * CLI shim for Palimpsest.
 *
 * Locates the platform-specific Rust binary and forwards
 * all arguments to it, preserving stdio.
 */

const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

function getBinaryPath() {
  const platform = process.platform;
  const binaryName = platform === "win32" ? "palin.exe" : "palin";

  // Look in the npm package's bin directory first
  const localBin = path.join(__dirname, "bin", binaryName);
  if (fs.existsSync(localBin)) {
    return localBin;
  }

  // Fallback: check if palin is on PATH
  return binaryName;
}

function main() {
  const binaryPath = getBinaryPath();
  const args = process.argv.slice(2);

  if (!fs.existsSync(binaryPath) && !binaryPath.includes(path.sep)) {
    // binaryPath is just "palin" or "palin.exe" — check PATH
    const result = spawnSync(binaryPath, args, {
      stdio: "inherit",
      windowsHide: true,
    });
    process.exit(result.status ?? 1);
  }

  if (!fs.existsSync(binaryPath)) {
    console.error("✗ palimpsest binary not found. Reinstall with: npm install -g palimpsest");
    process.exit(1);
  }

  const result = spawnSync(binaryPath, args, {
    stdio: "inherit",
    windowsHide: true,
  });

  process.exit(result.status ?? 1);
}

main();
