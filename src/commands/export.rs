use crate::storage;
use crate::types::ExportFormat;
use std::path::Path;

/// Execute `palin export <name> <epoch> --format zip|tar`
pub fn execute(
    name: Option<&str>,
    epoch_id: &str,
    format: ExportFormat,
    output_dir: Option<&Path>,
) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epoch = storage::resolve_epoch(&conn, epoch_id)?;
    let entries = storage::get_file_entries(&conn, epoch.id, "epoch")?;

    let out_dir = match output_dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
    };

    std::fs::create_dir_all(&out_dir)?;

    let epoch_name = epoch.display_name();
    let filename = format!("{}-{}.{}", resolved.name, epoch_name, format.extension());
    let output_path = out_dir.join(&filename);

    match format {
        ExportFormat::Zip => export_zip(&output_path, &resolved.name, &entries)?,
        ExportFormat::Tar => export_tar(&output_path, &resolved.name, &entries)?,
    }

    println!(
        "✦ Exported {} as '{}' ({})",
        epoch_name,
        output_path.display(),
        format.extension()
    );

    Ok(())
}

fn export_zip(
    output_path: &Path,
    palin_name: &str,
    entries: &[crate::types::FileEntry],
) -> anyhow::Result<()> {
    use std::io::Write;

    let file = std::fs::File::create(output_path)?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in entries {
        if let Some(ref hash) = entry.ink_hash {
            if let Ok(content) = crate::storage::ink::read_ink(palin_name, hash) {
                let path = entry.file_path.replace('\\', "/");
                zip_writer.start_file(&path, options)?;
                zip_writer.write_all(&content)?;
            }
        }
    }

    zip_writer.finish()?;
    Ok(())
}

fn export_tar(
    output_path: &Path,
    palin_name: &str,
    entries: &[crate::types::FileEntry],
) -> anyhow::Result<()> {
    let file = std::fs::File::create(output_path)?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar_builder = tar::Builder::new(enc);

    for entry in entries {
        if let Some(ref hash) = entry.ink_hash {
            if let Ok(content) = crate::storage::ink::read_ink(palin_name, hash) {
                let path = entry.file_path.replace('\\', "/");
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o644);
                tar_builder.append_data(&mut header, &path, &content[..])?;
            }
        }
    }

    tar_builder.finish()?;
    Ok(())
}
