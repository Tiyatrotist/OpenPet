use crate::manifest::PetPackManifest;
use crate::parser::{compute_sha256, PetPackError};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Packages a directory structure containing manifest.json and assets into a signed/hashed .openpet archive.
pub fn build_petpack(source_dir: &Path, output_path: &Path) -> Result<(), PetPackError> {
    let manifest_path = source_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Err(PetPackError::MissingManifest);
    }

    let manifest_bytes = fs::read(&manifest_path)?;
    let mut manifest: PetPackManifest = serde_json::from_slice(&manifest_bytes)?;

    // Scan source dir and compute hashes for all included files
    let mut file_entries = Vec::new();
    collect_files_recursive(source_dir, source_dir, &mut file_entries)?;

    for (rel_path, abs_path) in &file_entries {
        if rel_path == "manifest.json" {
            continue;
        }
        let content = fs::read(abs_path)?;
        let hash = compute_sha256(&content);
        manifest.hashes.insert(rel_path.clone(), hash);
    }

    manifest.validate().map_err(PetPackError::ManifestInvalid)?;

    // Write out the archive
    let out_file = File::create(output_path)?;
    let mut zip = ZipWriter::new(out_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // Write updated manifest
    zip.start_file("manifest.json", options)?;
    let updated_manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    zip.write_all(&updated_manifest_bytes)?;

    // Write other files
    for (rel_path, abs_path) in file_entries {
        if rel_path == "manifest.json" {
            continue;
        }
        zip.start_file(&rel_path, options)?;
        let mut f = File::open(&abs_path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
    }

    zip.finish()?;
    Ok(())
}

fn collect_files_recursive(
    root: &Path,
    current: &Path,
    acc: &mut Vec<(String, std::path::PathBuf)>,
) -> Result<(), PetPackError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(root, &path, acc)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| PetPackError::PathTraversal(e.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            acc.push((rel, path));
        }
    }
    Ok(())
}
