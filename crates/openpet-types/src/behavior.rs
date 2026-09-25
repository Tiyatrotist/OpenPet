use serde::{Deserialize, Serialize};

/// High-level behavior tags supported by OpenPet animations and AI utility models.
/// Supports at least the required 16 behavior states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorType {
    Idle,
    Walk,
    Run,
    Sit,
    Sleep,
    Wake,
    Stretch,
    Play,
    Pet,
    HighFive,
    Think,
    Talk,
    Happy,
    Sad,
    Curious,
    Surprised,
    Summon,
    Dismiss,
    // Real Cat behavior cycles:
    Purr,
    Knead,
    Zoomies,
    Loaf,
    Hunting,
}

impl BehaviorType {
    /// Returns the semantic string identifier of the behavior.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Walk => "walk",
            Self::Run => "run",
            Self::Sit => "sit",
            Self::Sleep => "sleep",
            Self::Wake => "wake",
            Self::Stretch => "stretch",
            Self::Play => "play",
            Self::Pet => "pet",
            Self::HighFive => "high_five",
            Self::Think => "think",
            Self::Talk => "talk",
            Self::Happy => "happy",
            Self::Sad => "sad",
            Self::Curious => "curious",
            Self::Surprised => "surprised",
            Self::Summon => "summon",
            Self::Dismiss => "dismiss",
            Self::Purr => "purr",
            Self::Knead => "knead",
            Self::Zoomies => "zoomies",
            Self::Loaf => "loaf",
            Self::Hunting => "hunting",
        }
    }

    /// Whether this behavior indicates locomotion (active screen coordinate changes).
    pub fn is_locomotive(&self) -> bool {
        matches!(self, Self::Walk | Self::Run | Self::Zoomies)
    }

    /// List of mandatory V1 core behaviors for any compliant PetPack.
    pub fn core_v1_behaviors() -> &'static [BehaviorType] {
        &[
            Self::Idle,
            Self::Walk,
            Self::Run,
            Self::Sit,
            Self::Sleep,
            Self::Wake,
            Self::Stretch,
            Self::Play,
            Self::Pet,
            Self::HighFive,
            Self::Think,
            Self::Talk,
            Self::Happy,
            Self::Sad,
            Self::Curious,
            Self::Surprised,
        ]
    }
}

/// Direct user interaction with the pet on desktop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InteractionType {
    SingleClick,
    DoubleClick,
    DragStart { x: f32, y: f32 },
    DragMove { x: f32, y: f32 },
    DragEnd { x: f32, y: f32 },
    Petting { intensity: f32 },
    HighFive,
    Feed,
    Play,
    SleepToggle,
    Water,
    Groom,
}

/// Screen coordinates representing pet placement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenPosition {
    pub x: f32,
    pub y: f32,
}

impl ScreenPosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Emote bubble or floating visual icon over the pet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PetEmote {
    Heart,
    Exclamation,
    Question,
    Zzz,
    Sweat,
    Lightbulb,
    Music,
}

/// High-level animation command issued by behavior engine or plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationCommand {
    pub behavior: BehaviorType,
    pub loops: u32,
    pub emote: Option<PetEmote>,
    pub target_pos: Option<ScreenPosition>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior_core_v1_count() {
        assert!(BehaviorType::core_v1_behaviors().len() >= 12);
    }
}
