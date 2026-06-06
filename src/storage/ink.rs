use sha2::{Digest, Sha256};

/// Compute the SHA-256 hash of file content
pub fn hash_content(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Store file content in the ink store, returning the hash
pub fn store_ink(name: &str, content: &[u8]) -> anyhow::Result<String> {
    let hash = hash_content(content);
    let ink_dir = crate::types::palimpsest_dir()?.join(name).join("inks");
    std::fs::create_dir_all(&ink_dir)?;
    let ink_path = ink_dir.join(&hash);

    if !ink_path.exists() {
        std::fs::write(&ink_path, content)?;
    }

    Ok(hash)
}

/// Read ink content by hash
pub fn read_ink(name: &str, hash: &str) -> anyhow::Result<Vec<u8>> {
    let ink_path = crate::types::palimpsest_dir()?
        .join(name)
        .join("inks")
        .join(hash);

    if !ink_path.exists() {
        anyhow::bail!("Ink '{}' not found for palin '{}'", hash, name);
    }

    Ok(std::fs::read(&ink_path)?)
}

/// Delete an ink file
pub fn delete_ink_file(name: &str, hash: &str) -> anyhow::Result<()> {
    let ink_path = crate::types::palimpsest_dir()?
        .join(name)
        .join("inks")
        .join(hash);

    if ink_path.exists() {
        std::fs::remove_file(&ink_path)?;
    }

    Ok(())
}
