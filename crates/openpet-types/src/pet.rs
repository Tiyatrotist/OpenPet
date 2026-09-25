use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for an OpenPet pet instance or pet pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PetId(pub String);

impl PetId {
    /// Creates a new PetId after trimming whitespace.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into().trim().to_lowercase())
    }

    /// Default sample pet identifier included with the application.
    pub fn default_pet() -> Self {
        Self("mimi-cat".to_string())
    }
}

impl fmt::Display for PetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Circadian cycle phase of the pet based on local time and activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircadianPhase {
    Dawn,
    Day,
    Dusk,
    Night,
}

impl CircadianPhase {
    /// Calculates current phase from hour of day (0..=23).
    pub fn from_hour(hour: u32) -> Self {
        match hour {
            6..=8 => Self::Dawn,
            9..=18 => Self::Day,
            19..=22 => Self::Dusk,
            _ => Self::Night,
        }
    }
}

/// General activity intensity for the pet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityLevel {
    Resting,
    Low,
    Moderate,
    High,
}

/// Recognized domestic cat breeds and coat variations supported by the Real Cat architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CatBreed {
    #[default]
    Tabby, // Tekir - classic striped pattern
    Tuxedo,  // Smokin - black and white formal coat
    Calico,  // Alacalı / Üç Renkli - tricolor patches
    Ginger,  // Sarıman - warm marmalade ginger coat
    Siamese, // Siyam - pointed color pattern with blue eyes
    Black,   // Kara Kedi - solid midnight black coat
    White,   // Pamuk / Beyaz - pure snow white coat
}

impl CatBreed {
    pub fn display_name(&self, is_tr: bool) -> &'static str {
        match self {
            Self::Tabby => {
                if is_tr {
                    "Tekir (Tabby)"
                } else {
                    "Tabby (Tekir)"
                }
            }
            Self::Tuxedo => {
                if is_tr {
                    "Smokin (Tuxedo)"
                } else {
                    "Tuxedo"
                }
            }
            Self::Calico => {
                if is_tr {
                    "Alacalı (Calico)"
                } else {
                    "Calico"
                }
            }
            Self::Ginger => {
                if is_tr {
                    "Sarıman (Ginger)"
                } else {
                    "Ginger"
                }
            }
            Self::Siamese => {
                if is_tr {
                    "Siyam (Siamese)"
                } else {
                    "Siamese"
                }
            }
            Self::Black => {
                if is_tr {
                    "Kara Kedi (Black)"
                } else {
                    "Black Cat"
                }
            }
            Self::White => {
                if is_tr {
                    "Pamuk (White)"
                } else {
                    "White Cat"
                }
            }
        }
    }

    pub fn id_str(&self) -> &'static str {
        match self {
            Self::Tabby => "tabby",
            Self::Tuxedo => "tuxedo",
            Self::Calico => "calico",
            Self::Ginger => "ginger",
            Self::Siamese => "siamese",
            Self::Black => "black",
            Self::White => "white",
        }
    }

    pub fn all_breeds() -> &'static [CatBreed] {
        &[
            Self::Tabby,
            Self::Tuxedo,
            Self::Calico,
            Self::Ginger,
            Self::Siamese,
            Self::Black,
            Self::White,
        ]
    }
}

/// Standard resolutions supported for cat asset spritesheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetResolution {
    Low64,     // 64x64 Retro Pixel Art
    Medium128, // 128x128 Standard HD Desktop
    High256,   // 256x256 HiDPI / 4K Crisp Modern Art
}

impl Serialize for AssetResolution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Low64 => serializer.serialize_str("low64"),
            Self::Medium128 => serializer.serialize_str("medium128"),
            Self::High256 => serializer.serialize_str("high256"),
        }
    }
}

impl<'de> Deserialize<'de> for AssetResolution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AssetResolutionVisitor;

        impl<'de> serde::de::Visitor<'de> for AssetResolutionVisitor {
            type Value = AssetResolution;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "a resolution name ('low64', 'medium128', 'high256') or integer (64, 128, 256)",
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<AssetResolution, E>
            where
                E: serde::de::Error,
            {
                match value.to_lowercase().replace(['-', '_'], "").as_str() {
                    "low64" | "64" | "low" => Ok(AssetResolution::Low64),
                    "medium128" | "128" | "medium" | "standard" => Ok(AssetResolution::Medium128),
                    "high256" | "256" | "high" | "hidpi" => Ok(AssetResolution::High256),
                    _ => Err(E::custom(format!("Unknown asset resolution: {}", value))),
                }
            }

            fn visit_u64<E>(self, value: u64) -> Result<AssetResolution, E>
            where
                E: serde::de::Error,
            {
                match value {
                    64 => Ok(AssetResolution::Low64),
                    128 => Ok(AssetResolution::Medium128),
                    256 => Ok(AssetResolution::High256),
                    _ => Err(E::custom(format!(
                        "Unsupported asset resolution dimension: {} (expected 64, 128, or 256)",
                        value
                    ))),
                }
            }

            fn visit_i64<E>(self, value: i64) -> Result<AssetResolution, E>
            where
                E: serde::de::Error,
            {
                if value > 0 {
                    self.visit_u64(value as u64)
                } else {
                    Err(E::custom("Asset resolution must be positive"))
                }
            }
        }

        deserializer.deserialize_any(AssetResolutionVisitor)
    }
}

impl AssetResolution {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Low64 => (64, 64),
            Self::Medium128 => (128, 128),
            Self::High256 => (256, 256),
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Low64 => 64,
            Self::Medium128 => 128,
            Self::High256 => 256,
        }
    }
}

/// Specifications for Real Cat asset spritesheets and animation packs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatAssetSpec {
    pub breed: CatBreed,
    pub resolution: (u32, u32),
    pub frame_rate_fps: u32,
    pub format: String,
    pub required_animations: Vec<String>,
}

impl Default for CatAssetSpec {
    fn default() -> Self {
        Self {
            breed: CatBreed::Tabby,
            resolution: (128, 128),
            frame_rate_fps: 10,
            format: "png/rgba32".to_string(),
            required_animations: vec![
                "idle".into(),
                "walk".into(),
                "run".into(),
                "sit".into(),
                "sleep".into(),
                "purr".into(),
                "knead".into(),
                "zoomies".into(),
                "loaf".into(),
                "groom".into(),
                "eat".into(),
                "drink".into(),
                "play".into(),
                "pet".into(),
                "surprised".into(),
            ],
        }
    }
}

fn default_hunger() -> f32 {
    0.85
}

fn default_thirst() -> f32 {
    0.90
}

fn default_hygiene() -> f32 {
    0.80
}

/// The internal state of the pet runtime, normalized from 0.0 to 1.0.
///
/// Deterministically modified by behavior ticks, interactions, and elapsed time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PetState {
    /// Emotional happiness index (0.0 = distressed/sad, 1.0 = ecstatic)
    pub mood: f32,
    /// Physical stamina (0.0 = exhausted, 1.0 = energetic)
    pub energy: f32,
    /// Desire to inspect surroundings (0.0 = indifferent, 1.0 = inquisitive)
    pub curiosity: f32,
    /// Relationship depth with owner (0.0 = stranger, 1.0 = deeply bonded)
    pub bond: f32,
    /// Need for stimulation (0.0 = content, 1.0 = desperately bored)
    pub boredom: f32,
    /// Need for sleep (0.0 = wide awake, 1.0 = drowsy/asleep)
    pub sleepiness: f32,
    /// Fullness level (0.0 = starving, 1.0 = satisfied/full)
    #[serde(default = "default_hunger")]
    pub hunger: f32,
    /// Hydration level (0.0 = parched, 1.0 = well-hydrated)
    #[serde(default = "default_thirst")]
    pub thirst: f32,
    /// Coat hygiene / grooming status (0.0 = messy, 1.0 = clean and groomed)
    #[serde(default = "default_hygiene")]
    pub hygiene: f32,
    /// Current activity level
    pub activity: ActivityLevel,
    /// Timestamp of most recent user interaction
    pub last_interaction: DateTime<Utc>,
    /// Natural circadian rhythm
    pub circadian_phase: CircadianPhase,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            mood: 0.8,
            energy: 0.85,
            curiosity: 0.7,
            bond: 0.5,
            boredom: 0.1,
            sleepiness: 0.1,
            hunger: 0.85,
            thirst: 0.90,
            hygiene: 0.80,
            activity: ActivityLevel::Moderate,
            last_interaction: Utc::now(),
            circadian_phase: CircadianPhase::Day,
        }
    }
}

impl PetState {
    /// Clamps all internal state values to the valid [0.0, 1.0] interval.
    pub fn clamp_bounds(&mut self) {
        self.mood = self.mood.clamp(0.0, 1.0);
        self.energy = self.energy.clamp(0.0, 1.0);
        self.curiosity = self.curiosity.clamp(0.0, 1.0);
        self.bond = self.bond.clamp(0.0, 1.0);
        self.boredom = self.boredom.clamp(0.0, 1.0);
        self.sleepiness = self.sleepiness.clamp(0.0, 1.0);
        self.hunger = self.hunger.clamp(0.0, 1.0);
        self.thirst = self.thirst.clamp(0.0, 1.0);
        self.hygiene = self.hygiene.clamp(0.0, 1.0);
    }
}

/// Installed or discoverable pet metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PetMetadata {
    pub id: PetId,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub homepage: Option<String>,
    pub created_with: Option<String>,
    pub source_provenance: Option<String>,
    pub minimum_openpet_version: String,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pet_state_clamp() {
        let mut state = PetState {
            mood: 1.5,
            energy: -0.2,
            hunger: 1.2,
            thirst: -0.5,
            hygiene: 2.0,
            ..Default::default()
        };
        state.clamp_bounds();
        assert_eq!(state.mood, 1.0);
        assert_eq!(state.energy, 0.0);
        assert_eq!(state.hunger, 1.0);
        assert_eq!(state.thirst, 0.0);
        assert_eq!(state.hygiene, 1.0);
    }

    #[test]
    fn test_circadian_phase() {
        assert_eq!(CircadianPhase::from_hour(7), CircadianPhase::Dawn);
        assert_eq!(CircadianPhase::from_hour(14), CircadianPhase::Day);
        assert_eq!(CircadianPhase::from_hour(20), CircadianPhase::Dusk);
        assert_eq!(CircadianPhase::from_hour(2), CircadianPhase::Night);
    }

    #[test]
    fn test_cat_breeds_and_asset_spec() {
        let breeds = CatBreed::all_breeds();
        assert_eq!(breeds.len(), 7);
        let spec = CatAssetSpec::default();
        assert_eq!(spec.resolution, (128, 128));
        assert!(spec.required_animations.contains(&"purr".to_string()));
        assert!(spec.required_animations.contains(&"knead".to_string()));
        assert!(spec.required_animations.contains(&"zoomies".to_string()));
    }

    #[test]
    fn test_asset_resolution_serde() {
        let r1: AssetResolution = serde_json::from_str("\"low64\"").unwrap();
        assert_eq!(r1, AssetResolution::Low64);
        assert_eq!(r1.as_u32(), 64);

        let r2: AssetResolution = serde_json::from_str("\"medium128\"").unwrap();
        assert_eq!(r2, AssetResolution::Medium128);
        assert_eq!(r2.as_u32(), 128);

        let r3: AssetResolution = serde_json::from_str("256").unwrap();
        assert_eq!(r3, AssetResolution::High256);
        assert_eq!(r3.as_u32(), 256);

        let ser = serde_json::to_string(&r2).unwrap();
        assert_eq!(ser, "\"medium128\"");
    }
}
