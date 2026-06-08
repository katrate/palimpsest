use anyhow::{anyhow, Context};
use std::io::{Read, Write};
use std::path::PathBuf;

/// Get the target triple for the current platform (matches release asset names).
fn target_triple() -> &'static str {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        _ => panic!("Unsupported platform: {}-{}", os, arch),
    }
}

/// Extension for the archive format.
fn archive_ext() -> &'static str {
    if cfg!(target_os = "windows") { "zip" } else { "tar.gz" }
}

/// Run the auto-update.
pub fn execute() -> anyhow::Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("✦ Current version: v{}", current_version);

    // ── Fetch latest release info from GitHub API ────────────
    println!("✦ Checking for updates...");
    let resp = ureq::get("https://api.github.com/repos/katrate/palimpsest/releases/latest")
        .set("User-Agent", "palin-updater/1.0")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| anyhow!("Failed to fetch release info: {}", e))?;

    let mut body = String::new();
    resp.into_reader().read_to_string(&mut body)?;
    let release: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse GitHub API response")?;

    let latest_tag = release["tag_name"].as_str().unwrap_or("unknown");
    println!("✦ Latest version: {}", latest_tag);

    // Strip leading 'v' for comparison
    let latest_ver = latest_tag.trim_start_matches('v');
    if latest_ver == current_version {
        println!("✦ You're already up to date! (v{})", current_version);
        return Ok(());
    }

    // ── Find the asset matching our platform ─────────────────
    let target = target_triple();
    let ext = archive_ext();
    let asset_name = format!("palimpsest-{}.{}", target, ext);

    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| anyhow!("No assets found in release"))?;

    let asset = assets
        .iter()
        .find(|a| {
            a["name"]
                .as_str()
                .map(|n| n == asset_name)
                .unwrap_or(false)
        })
        .ok_or_else(|| anyhow!("No matching asset found for {}", target))?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| anyhow!("No download URL for asset"))?;

    let asset_size = asset["size"].as_u64().unwrap_or(0);

    println!(
        "✦ Downloading {} ({})...",
        asset_name,
        if asset_size > 1_000_000 {
            format!("{:.1} MB", asset_size as f64 / 1_000_000.0)
        } else {
            format!("{} bytes", asset_size)
        }
    );

    // ── Download the archive ─────────────────────────────────
    let resp = ureq::get(download_url)
        .set("User-Agent", "palin-updater/1.0")
        .call()
        .map_err(|e| anyhow!("Download failed: {}", e))?;

    let temp_dir = std::env::temp_dir().join("palin_update");
    std::fs::create_dir_all(&temp_dir)?;

    let archive_path = temp_dir.join(&asset_name);
    {
        let mut file = std::fs::File::create(&archive_path)?;
        let mut reader = resp.into_reader();
        std::io::copy(&mut reader, &mut file)?;
    }

    // ── Extract the binary from the archive ──────────────────
    let binary_name = if cfg!(target_os = "windows") {
        "palin.exe"
    } else {
        "palin"
    };
    let extracted_path = temp_dir.join(binary_name);

    if cfg!(target_os = "windows") {
        extract_zip(&archive_path, &extracted_path)?;
    } else {
        extract_tar_gz(&archive_path, &extracted_path)?;
    }

    // Quick sanity check: read a few bytes
    let meta = std::fs::metadata(&extracted_path)?;
    if meta.len() < 1024 {
        anyhow::bail!("Downloaded binary looks too small ({})", meta.len());
    }

    // ── Replace the current executable ───────────────────────
    let current_exe = std::env::current_exe()?;
    println!("✦ Installing update...");

    if cfg!(target_os = "windows") {
        // On Windows, we can't overwrite a running EXE.
        // Create a batch script that waits, copies, cleans up, and starts the new version.
        let batch_path = temp_dir.join("update.bat");
        let mut bat = std::fs::File::create(&batch_path)?;
        write!(
            bat,
            "@echo off\n\
             ping 127.0.0.1 -n 2 > nul\n\
             copy /Y \"{}\" \"{}\" > nul\n\
             if exist \"{}\" del \"{}\"\n\
             rmdir /Q /S \"{}\" 2>nul\n\
             start \"\" /B \"{}\"\n\
             del \"%~f0\"\n",
            extracted_path.display(),
            current_exe.display(),
            archive_path.display(),
            archive_path.display(),
            temp_dir.display(),
            current_exe.display(),
        )?;
        bat.flush()?;

        // Spawn the batch script detached and exit
        let _ = std::process::Command::new("cmd")
            .args(&["/C", "start", "/MIN", batch_path.to_str().unwrap_or("")])
            .spawn();

        println!("✦ Update applied! Restarting...");
    } else {
        // On Unix, we can overwrite the running binary (old inode stays alive)
        std::fs::rename(&extracted_path, &current_exe)?;
        // Clean up temp
        let _ = std::fs::remove_file(&archive_path);
        let _ = std::fs::remove_dir(&temp_dir);

        println!("✦ Update applied! Restarting...");

        // Re-launch self
        let _ = std::process::Command::new(&current_exe)
            .args(&std::env::args().skip(1).collect::<Vec<_>>())
            .spawn();
    }

    std::process::exit(0);
}

/// Extract a single file (the binary) from a zip archive.
fn extract_zip(archive: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    use std::io::BufReader;

    let file = std::fs::File::open(archive)?;
    let mut reader = BufReader::new(file);
    let mut zip_reader = zip::ZipArchive::new(&mut reader)?;

    let binary_name = if cfg!(target_os = "windows") {
        "palin.exe"
    } else {
        "palin"
    };

    // Find the binary in the archive (it's inside a folder like palimpsest-x86_64-pc-windows-msvc/)
    for i in 0..zip_reader.len() {
        let mut entry = zip_reader.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        if name.ends_with(binary_name) {
            let mut out = std::fs::File::create(output)?;
            std::io::copy(&mut entry, &mut out)?;
            return Ok(());
        }
    }

    anyhow::bail!("Binary '{}' not found in archive", binary_name);
}

/// Extract a single file (the binary) from a tar.gz archive.
fn extract_tar_gz(archive: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar_reader = tar::Archive::new(decoder);

    let binary_name = if cfg!(target_os = "windows") {
        "palin.exe"
    } else {
        "palin"
    };

    for entry in tar_reader.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let name = path.to_string_lossy().replace('\\', "/");
        if name.ends_with(binary_name) {
            let mut out = std::fs::File::create(output)?;
            std::io::copy(&mut entry, &mut out)?;
            return Ok(());
        }
    }

    anyhow::bail!("Binary '{}' not found in archive", binary_name);
}
