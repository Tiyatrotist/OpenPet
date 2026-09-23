use openpet_types::BehaviorType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manifest schema version 1 for OpenPet pet packages.
pub const PETPACK_SCHEMA_VERSION_1: u32 = 1;

/// Frame definition in sprite atlas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasFrame {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u32,
}

/// Animation sequence mapping for a behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationSequence {
    pub frames: Vec<AtlasFrame>,
    pub loops: bool,
}

/// The complete manifest specification contained in `manifest.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PetPackManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub description: String,
    pub created_with: Option<String>,
    pub source_provenance: Option<String>,
    pub minimum_openpet_version: String,
    pub atlases: Vec<String>,
    pub animations: HashMap<String, AnimationSequence>,
    pub behavior_tags: Vec<String>,
    pub translations: Option<HashMap<String, String>>,
    pub sounds: Option<Vec<String>>,
    pub hashes: HashMap<String, String>,
}

impl PetPackManifest {
    /// Validates required fields and guarantees minimum core behavior coverage.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PETPACK_SCHEMA_VERSION_1 {
            return Err(format!(
                "Unsupported schema_version {}. Expected {}",
                self.schema_version, PETPACK_SCHEMA_VERSION_1
            ));
        }

        if self.id.trim().is_empty() {
            return Err("Manifest 'id' cannot be blank".into());
        }

        if self.name.trim().is_empty() {
            return Err("Manifest 'name' cannot be blank".into());
        }

        if self.license.trim().is_empty() {
            return Err("Manifest 'license' cannot be blank".into());
        }

        // Validate presence of at least the required V1 core behaviors
        let required = [
            BehaviorType::Idle.as_str(),
            BehaviorType::Walk.as_str(),
            BehaviorType::Sit.as_str(),
            BehaviorType::Sleep.as_str(),
        ];

        for req in required {
            if !self.animations.contains_key(req) {
                return Err(format!(
                    "Manifest missing mandatory animation sequence: '{}'",
                    req
                ));
            }
        }

        Ok(())
    }
}
