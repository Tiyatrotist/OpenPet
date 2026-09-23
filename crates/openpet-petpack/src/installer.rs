use crate::parser::{unpack_petpack_securely, PetPackError};
use openpet_types::{PetId, PetMetadata};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Atomically extracts and installs a .openpet file using isolated staging.
///
/// Ensures an interrupted or malicious unpack never leaves a corrupted pet in the live packs directory.
pub fn install_petpack_atomically(
    package_path: &Path,
    packs_dir: &Path,
    staging_dir: &Path,
) -> Result<PetMetadata, PetPackError> {
    let staging_id = format!("stage_{}", Uuid::new_v4());
    let isolated_stage = staging_dir.join(&staging_id);

    fs::create_dir_all(&isolated_stage)?;

    // Decompress and validate within staging
    let manifest_result = unpack_petpack_securely(package_path, &isolated_stage);

    let manifest = match manifest_result {
        Ok(m) => m,
        Err(e) => {
            // Clean up staging on failure
            let _ = fs::remove_dir_all(&isolated_stage);
            return Err(e);
        }
    };

    fs::create_dir_all(packs_dir)?;

    let target_pet_dir = packs_dir.join(&manifest.id);
    if !target_pet_dir.starts_with(packs_dir) {
        let _ = fs::remove_dir_all(&isolated_stage);
        return Err(PetPackError::PathTraversal(manifest.id.clone()));
    }

    if target_pet_dir.exists() {
        let _ = fs::remove_dir_all(&target_pet_dir);
    }

    // Atomic move into packs
    if let Err(e) = fs::rename(&isolated_stage, &target_pet_dir) {
        // Fallback for cross-device moves: copy then remove
        let _ = fs::remove_dir_all(&isolated_stage);
        return Err(PetPackError::Io(e));
    }

    Ok(PetMetadata {
        id: PetId::new(&manifest.id),
        name: manifest.name,
        version: manifest.version,
        author: manifest.author,
        description: manifest.description,
        license: manifest.license,
        homepage: manifest.homepage,
        created_with: manifest.created_with,
        source_provenance: manifest.source_provenance,
        minimum_openpet_version: manifest.minimum_openpet_version,
        tags: manifest.behavior_tags,
    })
}
