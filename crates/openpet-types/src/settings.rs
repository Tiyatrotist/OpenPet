use crate::pet::PetId;
use serde::{Deserialize, Serialize};

/// Supported interface language codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SupportedLocale {
    #[serde(rename = "en-US")]
    #[default]
    EnUs,
    #[serde(rename = "tr-TR")]
    TrTr,
}

impl SupportedLocale {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::TrTr => "tr-TR",
        }
    }

    pub fn from_str_lenient(s: &str) -> Self {
        if s.to_lowercase().starts_with("tr") {
            Self::TrTr
        } else {
            Self::EnUs
        }
    }
}

/// Rendering quality setting affecting target FPS and particle effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AnimationQuality {
    Low,    // Cap at 30 FPS, reduce subtle animations
    Medium, // Standard dynamic scaling
    #[default]
    High, // Uncapped active 60 FPS, full motion
}

/// Policy for when another application enters exclusive fullscreen mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FullscreenPolicy {
    #[default]
    HidePet,
    StayOnTop,
    SendToBack,
}

/// Release update channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

/// Comprehensive, strongly-typed user preferences and system settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    /// Active UI and pet speech language
    pub locale: SupportedLocale,
    /// Automatically launch OpenPet with Windows session
    pub launch_on_startup: bool,
    /// Keep pet window above normal desktop windows
    pub always_on_top: bool,
    /// Response when a fullscreen window is active
    pub fullscreen_policy: FullscreenPolicy,
    /// Privacy mode: immediately halts OCR, screen capture, and sensitive hooks
    pub privacy_mode: bool,
    /// Opt-in flag for local screen activity analysis (defaults strictly to false!)
    pub screen_analysis_enabled: bool,
    /// Currently chosen active pet
    pub active_pet_id: PetId,
    /// Animation quality and frame scheduling mode
    pub animation_quality: AnimationQuality,
    /// Respect user's Windows accessibility reduced motion preference
    pub reduced_motion: bool,
    /// Update release train
    pub update_channel: UpdateChannel,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: SupportedLocale::EnUs,
            launch_on_startup: false,
            always_on_top: true,
            fullscreen_policy: FullscreenPolicy::HidePet,
            privacy_mode: false,
            // CRITICAL PRIVACY REQUIREMENT: screen_analysis_enabled MUST default to false!
            screen_analysis_enabled: false,
            active_pet_id: PetId::default_pet(),
            animation_quality: AnimationQuality::High,
            reduced_motion: false,
            update_channel: UpdateChannel::Stable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_defaults() {
        let settings = AppSettings::default();
        // Zero capture privacy guarantee verification:
        assert!(!settings.screen_analysis_enabled);
        assert!(!settings.privacy_mode);
    }
}
