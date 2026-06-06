#!/usr/bin/env node

/**
 * Install script for palimpsest npm package.
 *
 * Detects the current platform/architecture and downloads the pre-built
 * Rust binary from the latest GitHub release.
 *
 * This avoids needing Rust installed just to use the CLI.
 */

const https = require("https");
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");
const { createWriteStream, existsSync, mkdirSync, chmodSync } = fs;

// ─── Config ──────────────────────────────────────────────────────────────

const PACKAGE_NAME = "palimpsest";
const BINARY_NAME = "palin";
// TODO: Replace 'your-username' with your actual GitHub username before publishing
const REPO = "katrate/palimpsest";

// Map Node.js platform/arch to our binary naming convention
const PLATFORM_MAP = {
  win32: { os: "pc-windows-msvc", ext: ".exe" },
  darwin: { os: "apple-darwin", ext: "" },
  linux: { os: "unknown-linux-gnu", ext: "" },
};

const ARCH_MAP = {
  x64: "x86_64",
  arm64: "aarch64",
};

// ─── Helpers ─────────────────────────────────────────────────────────────

function getTarget() {
  const platform = process.platform;
  const arch = process.arch;

  const plat = PLATFORM_MAP[platform];
  const cpu = ARCH_MAP[arch];

  if (!plat) {
    throw new Error(
      `Unsupported platform: ${platform}. ` +
        `Palimpsest supports Windows, macOS, and Linux (x64 & arm64).`
    );
  }
  if (!cpu) {
    throw new Error(
      `Unsupported architecture: ${arch}. ` +
        `Palimpsest supports x86_64 and arm64.`
    );
  }

  return { target: `${cpu}-${plat.os}`, ext: plat.ext };
}

function getInstallDir() {
  const dir = path.join(__dirname, "bin");
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  return dir;
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = createWriteStream(dest);
    const request = https.get(url, (response) => {
      // Handle redirects
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        file.close();
        fs.unlinkSync(dest);
        return download(response.headers.location, dest).then(resolve).catch(reject);
      }

      if (response.statusCode !== 200) {
        file.close();
        fs.unlinkSync(dest);
        let body = "";
        response.on("data", (chunk) => (body += chunk.toString()));
        response.on("end", () => {
          reject(
            new Error(
              `Download failed (HTTP ${response.statusCode}): ${body}`
            )
          );
        });
        return;
      }

      response.pipe(file);
      file.on("finish", () => {
        file.close();
        resolve();
      });
    });

    request.on("error", (err) => {
      file.close();
      if (existsSync(dest)) fs.unlinkSync(dest);
      reject(err);
    });

    // Timeout after 60 seconds
    request.setTimeout(60000, () => {
      request.destroy();
      file.close();
      if (existsSync(dest)) fs.unlinkSync(dest);
      reject(new Error("Download timed out after 60 seconds"));
    });
  });
}

function getLatestReleaseAssetUrl(target, ext) {
  // GitHub API to get the latest release
  const apiUrl = `https://api.github.com/repos/${REPO}/releases/latest`;

  return new Promise((resolve, reject) => {
    const request = https.get(
      apiUrl,
      { headers: { "User-Agent": "palimpsest-installer", Accept: "application/json" } },
      (response) => {
        let body = "";
        response.on("data", (chunk) => (body += chunk.toString()));
        response.on("end", () => {
          if (response.statusCode !== 200) {
            // Fallback: use the package version from npm (set during npm install)
            const version = process.env.npm_package_version || "0.1.0";
            const tag = version.startsWith("v") ? version : `v${version}`;
            const archiveName = `${BINARY_NAME}-${target}${ext === ".exe" ? ".zip" : ".tar.gz"}`;
            resolve(
              `https://github.com/${REPO}/releases/download/${tag}/${archiveName}`
            );
            return;
          }

          try {
            const release = JSON.parse(body);
            const tagName = release.tag_name;

            // Find the right asset
            const archiveName = `${BINARY_NAME}-${target}${ext === ".exe" ? ".zip" : ".tar.gz"}`;
            const asset = release.assets.find((a) => a.name === archiveName);

            if (asset) {
              resolve(asset.browser_download_url);
            } else {
              // Fallback to constructed URL
              resolve(
                `https://github.com/${REPO}/releases/download/${tagName}/${archiveName}`
              );
            }
          } catch {
            reject(new Error("Failed to parse GitHub release info"));
          }
        });
      }
    );

    request.on("error", reject);
    request.setTimeout(30000, () => {
      request.destroy();
      reject(new Error("Timed out fetching release info"));
    });
  });
}

function extractArchive(archivePath, destDir, ext) {
  return new Promise((resolve, reject) => {
    const binaryName = BINARY_NAME + ext;

    if (ext === ".exe") {
      // .zip file
      const { execSync } = require("child_process");
      try {
        // Use PowerShell to extract on Windows
        execSync(
          `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`,
          { stdio: "pipe" }
        );
        resolve(path.join(destDir, binaryName));
      } catch {
        // Fallback: try using the 'tar' command or just copy if the zip contains the binary directly
        const fs = require("fs");
        const outputPath = path.join(destDir, binaryName);
        fs.copyFileSync(archivePath, outputPath);
        resolve(outputPath);
      }
    } else {
      // .tar.gz file
      const { execSync } = require("child_process");
      try {
        execSync(
          `tar -xzf "${archivePath}" -C "${destDir}"`,
          { stdio: "pipe" }
        );
        resolve(path.join(destDir, binaryName));
      } catch {
        reject(new Error("Failed to extract tar.gz archive. Make sure tar is available."));
      }
    }
  });
}

// ─── Main ────────────────────────────────────────────────────────────────

async function main() {
  const { target, ext } = getTarget();
  const installDir = getInstallDir();
  const binaryPath = path.join(installDir, `${BINARY_NAME}${ext}`);

  // Check if binary already exists
  if (existsSync(binaryPath)) {
    console.log(`✓ palin already installed at ${binaryPath}`);
    return;
  }

  console.log(`✦ Downloading palin for ${target}...`);

  // Get the download URL
  const downloadUrl = await getLatestReleaseAssetUrl(target, ext);
  const archiveName = `${BINARY_NAME}-${target}${ext === ".exe" ? ".zip" : ".tar.gz"}`;
  const archivePath = path.join(installDir, archiveName);

  // Download
  try {
    await download(downloadUrl, archivePath);
  } catch (err) {
    console.error(`✗ Failed to download: ${err.message}`);
    console.error(`  URL: ${downloadUrl}`);
    console.error(`  You can manually download the binary from GitHub Releases.`);
    process.exit(1);
  }

  // Extract
  try {
    const extracted = await extractArchive(archivePath, installDir, ext);
    // Rename if needed
    if (extracted !== binaryPath && existsSync(extracted)) {
      fs.renameSync(extracted, binaryPath);
    }
  } catch (err) {
    console.error(`✗ Failed to extract archive: ${err.message}`);
    process.exit(1);
  }

  // Clean up archive
  try {
    fs.unlinkSync(archivePath);
  } catch {
    // ignore
  }

  // Make executable on non-Windows
  if (ext !== ".exe") {
    try {
      chmodSync(binaryPath, 0o755);
    } catch {
      // ignore
    }
  }

  if (existsSync(binaryPath)) {
    console.log(`✓ palin installed successfully at ${binaryPath}`);
    console.log(`  Run \`palin --help\` to get started`);
  } else {
    console.error(`✗ Binary not found after extraction`);
    process.exit(1);
  }
}

main().catch((err) => {
  console.error(`✗ Installation failed: ${err.message}`);
  process.exit(1);
});
