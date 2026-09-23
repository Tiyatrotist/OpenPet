use crate::hitmask::HitMask;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sprite definition in an atlas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteFrame {
    pub id: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub hitmask: Option<HitMask>,
}

/// Metadata mapping for a loaded sprite atlas.
#[derive(Debug, Clone, Default)]
pub struct SpriteAtlas {
    pub texture_path: String,
    pub width: u32,
    pub height: u32,
    pub frames: HashMap<String, SpriteFrame>,
}

impl SpriteAtlas {
    pub fn new(texture_path: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            texture_path: texture_path.into(),
            width,
            height,
            frames: HashMap::new(),
        }
    }

    pub fn insert_frame(&mut self, frame: SpriteFrame) {
        self.frames.insert(frame.id.clone(), frame);
    }

    pub fn get_frame(&self, id: &str) -> Option<&SpriteFrame> {
        self.frames.get(id)
    }
}
