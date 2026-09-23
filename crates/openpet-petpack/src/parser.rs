use crate::manifest::PetPackManifest;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path};
use thiserror::Error;
use zip::ZipArchive;

pub const MAX_PACKAGE_SIZE: u64 = 256 * 1024 * 1024; // 256 MB
pub const MAX_UNPACKED_SIZE: u64 = 512 * 1024 * 1024; // 512 MB
pub const MAX_FILE_COUNT: usize = 2000;
pub const MAX_SINGLE_ASSET_SIZE: u64 = 128 * 1024 * 1024; // 128 MB

#[derive(Error, Debug)]
pub enum PetPackError {
    #[error("Package archive exceeds maximum allowable size of {0} bytes")]
    PackageTooLarge(u64),

    #[error("Package exceeds maximum unpacked size of {0} bytes (decompression bomb protection)")]
    UnpackedSizeExceeded(u64),

    #[error("Package exceeds maximum file count limit of {0}")]
    FileCountExceeded(usize),

    #[error("File '{0}' exceeds single asset limit of {1} bytes")]
    SingleAssetTooLarge(String, u64),

    #[error("Security violation: path traversal or malicious path detected in '{0}'")]
    PathTraversal(String),

    #[error("Checksum mismatch for '{0}'. Expected {1}, computed {2}")]
    ChecksumMismatch(String, String, String),

    #[error("Missing mandatory manifest.json in archive")]
    MissingManifest,

    #[error("Manifest validation error: {0}")]
    ManifestInvalid(String),

    #[error("Archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Checks if a relative zip entry path is strictly safe against traversal and stream attacks.
pub fn is_safe_zip_entry_path(name: &str) -> bool {
    let path = Path::new(name);

    // Reject NTFS alternate data streams
    if name.contains(':') {
        return false;
    }

    // Must be relative
    if path.is_absolute() {
        return false;
    }

    // Must not contain ParentDir (..) or Prefix/RootDir components
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return false,
        }
    }

    true
}

/// Computes SHA256 hex digest of file contents.
pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Validates and securely unpacks a .openpet package into a destination directory.
pub fn unpack_petpack_securely(
    package_path: &Path,
    destination: &Path,
) -> Result<PetPackManifest, PetPackError> {
    let file = File::open(package_path)?;
    let metadata = file.metadata()?;

    if metadata.len() > MAX_PACKAGE_SIZE {
        return Err(PetPackError::PackageTooLarge(metadata.len()));
    }

    let mut archive = ZipArchive::new(file)?;
    if archive.len() > MAX_FILE_COUNT {
        return Err(PetPackError::FileCountExceeded(archive.len()));
    }

    let mut total_unpacked_bytes: u64 = 0;
    let mut manifest_bytes: Option<Vec<u8>> = None;
    let mut extracted_file_hashes: Vec<(String, String)> = Vec::new();

    // Pass 1: Validate paths, calculate cumulative sizes, extract manifest
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        if !is_safe_zip_entry_path(&name) {
            return Err(PetPackError::PathTraversal(name));
        }

        let size = entry.size();
        if size > MAX_SINGLE_ASSET_SIZE {
            return Err(PetPackError::SingleAssetTooLarge(name, size));
        }

        total_unpacked_bytes += size;
        if total_unpacked_bytes > MAX_UNPACKED_SIZE {
            return Err(PetPackError::UnpackedSizeExceeded(total_unpacked_bytes));
        }

        if name == "manifest.json" {
            let mut buf = Vec::new();
            let mut reader = entry;
            reader.read_to_end(&mut buf)?;
            manifest_bytes = Some(buf);
        }
    }

    let raw_manifest = manifest_bytes.ok_or(PetPackError::MissingManifest)?;
    let manifest: PetPackManifest = serde_json::from_slice(&raw_manifest)?;
    manifest.validate().map_err(PetPackError::ManifestInvalid)?;

    // Pass 2: Extract files safely into destination
    fs::create_dir_all(destination)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        if entry.is_dir() {
            let dir_path = destination.join(&name);
            fs::create_dir_all(&dir_path)?;
            continue;
        }

        let out_path = destination.join(&name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;

        let hash = compute_sha256(&content);
        extracted_file_hashes.push((name.clone(), hash));

        fs::write(&out_path, &content)?;
    }

    // Pass 3: Validate file hashes if listed in manifest
    for (filename, hash) in extracted_file_hashes {
        if let Some(expected_hash) = manifest.hashes.get(&filename) {
            if !expected_hash.eq_ignore_ascii_case(&hash) {
                return Err(PetPackError::ChecksumMismatch(
                    filename,
                    expected_hash.clone(),
                    hash,
                ));
            }
        }
    }

    Ok(manifest)
}
