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
            ..Default::default()
        };
        state.clamp_bounds();
        assert_eq!(state.mood, 1.0);
        assert_eq!(state.energy, 0.0);
    }

    #[test]
    fn test_circadian_phase() {
        assert_eq!(CircadianPhase::from_hour(7), CircadianPhase::Dawn);
        assert_eq!(CircadianPhase::from_hour(14), CircadianPhase::Day);
        assert_eq!(CircadianPhase::from_hour(20), CircadianPhase::Dusk);
        assert_eq!(CircadianPhase::from_hour(2), CircadianPhase::Night);
    }
}
