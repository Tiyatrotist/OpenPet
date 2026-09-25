//! # OpenPet PetPack Engine
//!
//! Secure `.openpet` bundle parser, builder, and atomic installer.

pub mod builder;
pub mod installer;
pub mod manifest;
pub mod parser;

pub use builder::*;
pub use installer::*;
pub use manifest::*;
pub use parser::*;

#[cfg(test)]
mod tests {
    use super::*;
    use openpet_types::BehaviorType;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn test_zip_slip_rejection() {
        assert!(!is_safe_zip_entry_path("../evil.exe"));
        assert!(!is_safe_zip_entry_path("..\\evil.exe"));
        assert!(!is_safe_zip_entry_path("/etc/passwd"));
        assert!(!is_safe_zip_entry_path("C:\\Windows\\System32\\cmd.exe"));
        assert!(!is_safe_zip_entry_path("assets:stream.txt"));
        assert!(is_safe_zip_entry_path("assets/atlas.json"));
        assert!(is_safe_zip_entry_path("manifest.json"));
    }

    #[test]
    fn test_petpack_build_and_atomic_install() {
        let temp_src = tempdir().unwrap();
        let temp_dest = tempdir().unwrap();
        let temp_staging = tempdir().unwrap();

        // Create sample manifest
        let mut animations = HashMap::new();
        for b in [
            BehaviorType::Idle.as_str(),
            BehaviorType::Walk.as_str(),
            BehaviorType::Sit.as_str(),
            BehaviorType::Sleep.as_str(),
        ] {
            animations.insert(
                b.to_string(),
                AnimationSequence {
                    frames: vec![AtlasFrame {
                        x: 0,
                        y: 0,
                        width: 64,
                        height: 64,
                        duration_ms: 100,
                    }],
                    loops: true,
                },
            );
        }

        let manifest = PetPackManifest {
            schema_version: 1,
            id: "sample-fox".to_string(),
            name: "Sample Fox".to_string(),
            version: "1.0.0".to_string(),
            author: "Tiyatrotist".to_string(),
            license: "AGPL-3.0".to_string(),
            homepage: None,
            description: "A friendly desktop fox".to_string(),
            created_with: None,
            source_provenance: None,
            minimum_openpet_version: "0.1.0".to_string(),
            atlases: vec!["atlas.json".to_string()],
            animations,
            behavior_tags: vec!["canine".to_string()],
            translations: None,
            sounds: None,
            hashes: HashMap::new(),
            breed: None,
            asset_resolution: Some(openpet_types::AssetResolution::Low64),
            supported_resolutions: vec![openpet_types::AssetResolution::Low64],
        };

        let manifest_file = temp_src.path().join("manifest.json");
        std::fs::write(&manifest_file, serde_json::to_vec(&manifest).unwrap()).unwrap();

        let dummy_atlas = temp_src.path().join("atlas.json");
        std::fs::write(&dummy_atlas, b"{\"atlas\":true}").unwrap();

        let package_out = temp_dest.path().join("sample-fox.openpet");
        build_petpack(temp_src.path(), &package_out).unwrap();
        assert!(package_out.exists());

        // Install atomically
        let packs_dir = temp_dest.path().join("packs");
        std::fs::create_dir_all(&packs_dir).unwrap();

        let metadata =
            install_petpack_atomically(&package_out, &packs_dir, temp_staging.path()).unwrap();

        assert_eq!(metadata.id.0, "sample-fox");
        assert!(packs_dir.join("sample-fox").join("manifest.json").exists());
        assert!(packs_dir.join("sample-fox").join("atlas.json").exists());
    }

    #[test]
    fn test_malicious_zip_slip_archive_rejected() {
        let temp = tempdir().unwrap();
        let archive_path = temp.path().join("malicious.openpet");
        let dest = temp.path().join("out");

        let file = File::create(&archive_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        zip.start_file("../evil.exe", options).unwrap();
        zip.write_all(b"malicious payload").unwrap();
        zip.finish().unwrap();

        let result = unpack_petpack_securely(&archive_path, &dest);
        assert!(result.is_err());
        match result {
            Err(PetPackError::PathTraversal(_)) => {}
            other => panic!("Expected PathTraversal, got {:?}", other),
        }
    }

    #[test]
    fn test_malicious_manifest_id_rejected() {
        assert!(validate_pet_id("good-cat_123").is_ok());
        assert!(validate_pet_id("../evil").is_err());
        assert!(validate_pet_id("evil/cat").is_err());
        assert!(validate_pet_id("evil\\cat").is_err());
        assert!(validate_pet_id("con").is_err());
        assert!(validate_pet_id("nul").is_err());
        assert!(validate_pet_id("").is_err());
    }

    #[test]
    fn test_install_petpack_creates_missing_packs_dir() {
        let temp_src = tempdir().unwrap();
        let temp_dest = tempdir().unwrap();
        let temp_staging = tempdir().unwrap();

        let mut animations = HashMap::new();
        for b in [
            BehaviorType::Idle.as_str(),
            BehaviorType::Walk.as_str(),
            BehaviorType::Sit.as_str(),
            BehaviorType::Sleep.as_str(),
        ] {
            animations.insert(
                b.to_string(),
                AnimationSequence {
                    frames: vec![AtlasFrame {
                        x: 0,
                        y: 0,
                        width: 64,
                        height: 64,
                        duration_ms: 100,
                    }],
                    loops: true,
                },
            );
        }

        let manifest = PetPackManifest {
            schema_version: 1,
            id: "fresh-puppy".to_string(),
            name: "Fresh Puppy".to_string(),
            version: "1.0.0".to_string(),
            author: "Tiyatrotist".to_string(),
            license: "AGPL-3.0".to_string(),
            homepage: None,
            description: "A puppy".to_string(),
            created_with: None,
            source_provenance: None,
            minimum_openpet_version: "0.1.0".to_string(),
            atlases: vec!["atlas.json".to_string()],
            animations,
            behavior_tags: vec!["canine".to_string()],
            translations: None,
            sounds: None,
            hashes: HashMap::new(),
            breed: None,
            asset_resolution: Some(openpet_types::AssetResolution::Low64),
            supported_resolutions: vec![openpet_types::AssetResolution::Low64],
        };

        let manifest_file = temp_src.path().join("manifest.json");
        std::fs::write(&manifest_file, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let dummy_atlas = temp_src.path().join("atlas.json");
        std::fs::write(&dummy_atlas, b"{\"atlas\":true}").unwrap();

        let package_out = temp_dest.path().join("fresh-puppy.openpet");
        build_petpack(temp_src.path(), &package_out).unwrap();

        // packs_dir intentionally does not exist yet!
        let packs_dir = temp_dest.path().join("non_existent_packs");
        assert!(!packs_dir.exists());

        let metadata =
            install_petpack_atomically(&package_out, &packs_dir, temp_staging.path()).unwrap();
        assert_eq!(metadata.id.0, "fresh-puppy");
        assert!(packs_dir.join("fresh-puppy").join("manifest.json").exists());
    }
}
