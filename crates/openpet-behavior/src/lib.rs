//! # OpenPet Behavior Engine
//!
//! Deterministic, utility-based behavior selector and state machine.
//!
//! ## Architectural Principles
//! - Zero LLM dependency in the real-time tick path.
//! - Runs on a dedicated tick frequency (~10-20 Hz) isolated from GPU rendering.
//! - Drag interactions override autonomous locomotion.

use chrono::Utc;
use openpet_types::{
    AnimationCommand, BehaviorType, CircadianPhase, InteractionType, PetEmote, PetState,
    ScreenPosition,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// Configurable bounds and weights for the Utility AI engine.
#[derive(Debug, Clone)]
pub struct BehaviorConfig {
    pub min_wander_distance: f32,
    pub max_wander_distance: f32,
    pub drag_interrupt_priority: f32,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            min_wander_distance: 30.0,
            max_wander_distance: 180.0,
            drag_interrupt_priority: 100.0,
        }
    }
}

/// The core Behavior Engine managing pet personality, state transitions, and actions.
pub struct BehaviorEngine {
    state: PetState,
    current_behavior: BehaviorType,
    position: ScreenPosition,
    is_dragging: bool,
    config: BehaviorConfig,
    rng: StdRng,
    behavior_elapsed: f32,
}

impl BehaviorEngine {
    /// Creates a new BehaviorEngine with a specific seed for deterministic testing or runtime.
    pub fn new_with_seed(seed: u64) -> Self {
        let state = PetState {
            last_interaction: chrono::DateTime::from_timestamp(1700000000, 0).unwrap(),
            ..Default::default()
        };
        Self {
            state,
            current_behavior: BehaviorType::Idle,
            position: ScreenPosition::new(500.0, 500.0),
            is_dragging: false,
            config: BehaviorConfig::default(),
            rng: StdRng::seed_from_u64(seed),
            behavior_elapsed: 0.0,
        }
    }

    pub fn new() -> Self {
        Self::new_with_seed(rand::thread_rng().gen())
    }
}

impl Default for BehaviorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorEngine {
    pub fn config(&self) -> &BehaviorConfig {
        &self.config
    }

    pub fn state(&self) -> &PetState {
        &self.state
    }

    pub fn current_behavior(&self) -> BehaviorType {
        self.current_behavior
    }

    pub fn position(&self) -> ScreenPosition {
        self.position
    }

    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    /// Evaluates user touch/mouse interactions, immediately prioritizing response.
    pub fn handle_interaction(&mut self, interaction: InteractionType) -> Option<AnimationCommand> {
        self.state.last_interaction = Utc::now();

        match interaction {
            InteractionType::DragStart { x, y } => {
                self.is_dragging = true;
                self.position = ScreenPosition::new(x, y);
                self.current_behavior = BehaviorType::Surprised;
                Some(AnimationCommand {
                    behavior: BehaviorType::Surprised,
                    loops: 1,
                    emote: Some(PetEmote::Exclamation),
                    target_pos: Some(self.position),
                })
            }
            InteractionType::DragMove { x, y } => {
                if self.is_dragging {
                    self.position = ScreenPosition::new(x, y);
                }
                None
            }
            InteractionType::DragEnd { x, y } => {
                self.is_dragging = false;
                self.position = ScreenPosition::new(x, y);
                self.current_behavior = BehaviorType::Sit;
                Some(AnimationCommand {
                    behavior: BehaviorType::Sit,
                    loops: 1,
                    emote: None,
                    target_pos: Some(self.position),
                })
            }
            InteractionType::SingleClick => {
                self.state.curiosity += 0.05;
                self.state.boredom -= 0.1;
                self.state.clamp_bounds();
                self.current_behavior = BehaviorType::Curious;
                Some(AnimationCommand {
                    behavior: BehaviorType::Curious,
                    loops: 1,
                    emote: Some(PetEmote::Question),
                    target_pos: None,
                })
            }
            InteractionType::DoubleClick => {
                self.state.mood += 0.1;
                self.state.clamp_bounds();
                self.current_behavior = BehaviorType::Happy;
                Some(AnimationCommand {
                    behavior: BehaviorType::Happy,
                    loops: 1,
                    emote: Some(PetEmote::Heart),
                    target_pos: None,
                })
            }
            InteractionType::Petting { intensity } => {
                self.state.bond += 0.05 * intensity;
                self.state.mood += 0.08 * intensity;
                self.state.boredom -= 0.15;
                self.state.clamp_bounds();
                self.current_behavior = BehaviorType::Pet;
                Some(AnimationCommand {
                    behavior: BehaviorType::Pet,
                    loops: 2,
                    emote: Some(PetEmote::Heart),
                    target_pos: None,
                })
            }
            InteractionType::HighFive => {
                self.state.bond += 0.1;
                self.state.energy -= 0.05;
                self.state.clamp_bounds();
                self.current_behavior = BehaviorType::HighFive;
                Some(AnimationCommand {
                    behavior: BehaviorType::HighFive,
                    loops: 1,
                    emote: Some(PetEmote::Music),
                    target_pos: None,
                })
            }
            InteractionType::Feed => {
                self.state.energy += 0.2;
                self.state.mood += 0.15;
                self.state.clamp_bounds();
                self.current_behavior = BehaviorType::Happy;
                Some(AnimationCommand {
                    behavior: BehaviorType::Happy,
                    loops: 2,
                    emote: Some(PetEmote::Heart),
                    target_pos: None,
                })
            }
            InteractionType::Play => {
                self.state.boredom -= 0.3;
                self.state.energy -= 0.15;
                self.state.clamp_bounds();
                self.current_behavior = BehaviorType::Play;
                Some(AnimationCommand {
                    behavior: BehaviorType::Play,
                    loops: 2,
                    emote: Some(PetEmote::Music),
                    target_pos: None,
                })
            }
            InteractionType::SleepToggle => {
                if self.current_behavior == BehaviorType::Sleep {
                    self.current_behavior = BehaviorType::Idle;
                    self.behavior_elapsed = 0.0;
                    self.state.sleepiness = 0.1;
                    Some(AnimationCommand {
                        behavior: BehaviorType::Idle,
                        loops: 1,
                        emote: Some(PetEmote::Music),
                        target_pos: None,
                    })
                } else {
                    self.current_behavior = BehaviorType::Sleep;
                    self.behavior_elapsed = 0.0;
                    self.state.sleepiness = 0.9;
                    Some(AnimationCommand {
                        behavior: BehaviorType::Sleep,
                        loops: 10,
                        emote: Some(PetEmote::Zzz),
                        target_pos: None,
                    })
                }
            }
        }
    }

    /// Explicitly forces a specific behavior mode and resets elapsed duration.
    pub fn set_behavior(&mut self, behavior: BehaviorType) {
        self.current_behavior = behavior;
        self.behavior_elapsed = 0.0;
    }

    /// Computes the utility score for a given candidate behavior given current PetState.
    pub fn compute_utility(&self, behavior: BehaviorType) -> f32 {
        let s = &self.state;

        match behavior {
            BehaviorType::Sleep => {
                if s.energy < 0.2 || s.sleepiness > 0.8 {
                    0.9 + (1.0 - s.energy) * 0.1
                } else if s.circadian_phase == CircadianPhase::Night {
                    0.7
                } else {
                    0.05
                }
            }
            BehaviorType::Sit => {
                if s.energy < 0.4 {
                    0.6
                } else {
                    0.2
                }
            }
            BehaviorType::Walk => {
                if s.energy > 0.3 && s.boredom > 0.4 {
                    0.7
                } else {
                    0.3
                }
            }
            BehaviorType::Run => {
                if s.energy > 0.7 && s.mood > 0.6 {
                    0.65
                } else {
                    0.1
                }
            }
            BehaviorType::Play => {
                if s.energy > 0.5 && s.boredom > 0.5 {
                    0.75
                } else {
                    0.2
                }
            }
            BehaviorType::Stretch => 0.25,
            BehaviorType::Curious => s.curiosity * 0.7,
            BehaviorType::Idle => 0.4,
            _ => 0.15,
        }
    }

    /// Evaluates the highest utility behavior candidates.
    pub fn select_best_behavior(&mut self) -> BehaviorType {
        let candidates = [
            BehaviorType::Idle,
            BehaviorType::Sit,
            BehaviorType::Walk,
            BehaviorType::Run,
            BehaviorType::Play,
            BehaviorType::Sleep,
            BehaviorType::Stretch,
            BehaviorType::Curious,
        ];

        let mut best_behavior = BehaviorType::Idle;
        let mut max_score = -1.0;

        for candidate in candidates {
            let score = self.compute_utility(candidate) + self.rng.gen_range(0.0..0.05);
            if score > max_score {
                max_score = score;
                best_behavior = candidate;
            }
        }

        best_behavior
    }

    /// Main logic tick called at 10-20 Hz.
    ///
    /// Computes natural state decay, checks if behavior duration elapsed,
    /// and issues an AnimationCommand when a state change occurs.
    pub fn tick(&mut self, dt: f32) -> Option<AnimationCommand> {
        // While user is dragging, do not autonomously wander or mutate coordinate!
        if self.is_dragging {
            return None;
        }

        self.behavior_elapsed += dt;

        // Gradual state decay:
        if self.current_behavior == BehaviorType::Sleep {
            self.state.energy += 0.05 * dt;
            self.state.sleepiness -= 0.08 * dt;
        } else {
            self.state.energy -= 0.01 * dt;
            self.state.boredom += 0.015 * dt;
            self.state.sleepiness += 0.008 * dt;
        }
        self.state.clamp_bounds();

        // Behavior duration: every 4 to 8 seconds or if state requires immediate shift
        let threshold = if self.current_behavior == BehaviorType::Sleep {
            15.0
        } else {
            6.0
        };

        if self.behavior_elapsed >= threshold {
            self.behavior_elapsed = 0.0;
            let next = self.select_best_behavior();

            if next != self.current_behavior {
                self.current_behavior = next;

                let target_pos = if next.is_locomotive() {
                    let dx = self.rng.gen_range(-100.0..100.0);
                    let dy = self.rng.gen_range(-40.0..40.0);
                    let new_pos = ScreenPosition::new(
                        (self.position.x + dx).clamp(100.0, 1800.0),
                        (self.position.y + dy).clamp(100.0, 1000.0),
                    );
                    self.position = new_pos;
                    Some(new_pos)
                } else {
                    None
                };

                let emote = if next == BehaviorType::Sleep {
                    Some(PetEmote::Zzz)
                } else if next == BehaviorType::Play {
                    Some(PetEmote::Music)
                } else {
                    None
                };

                return Some(AnimationCommand {
                    behavior: next,
                    loops: if next == BehaviorType::Sleep { 10 } else { 1 },
                    emote,
                    target_pos,
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drag_coordinate_invariance() {
        let mut engine = BehaviorEngine::new_with_seed(42);

        // Start drag at (300, 400)
        engine.handle_interaction(InteractionType::DragStart { x: 300.0, y: 400.0 });
        assert!(engine.is_dragging());
        assert_eq!(engine.position(), ScreenPosition::new(300.0, 400.0));

        // Drag move to (450, 600)
        engine.handle_interaction(InteractionType::DragMove { x: 450.0, y: 600.0 });
        assert_eq!(engine.position(), ScreenPosition::new(450.0, 600.0));

        // Tick during drag MUST NOT alter coordinates
        for _ in 0..100 {
            let cmd = engine.tick(0.1);
            assert!(cmd.is_none());
            assert_eq!(engine.position(), ScreenPosition::new(450.0, 600.0));
        }

        // End drag
        engine.handle_interaction(InteractionType::DragEnd { x: 450.0, y: 600.0 });
        assert!(!engine.is_dragging());
    }

    #[test]
    fn test_deterministic_simulation() {
        let mut engine1 = BehaviorEngine::new_with_seed(1337);
        let mut engine2 = BehaviorEngine::new_with_seed(1337);

        for _ in 0..50 {
            let cmd1 = engine1.tick(0.5);
            let cmd2 = engine2.tick(0.5);
            assert_eq!(cmd1, cmd2);
            assert_eq!(engine1.state(), engine2.state());
        }
    }

    #[test]
    fn test_sleep_toggle_and_stat_adjustments() {
        let mut engine = BehaviorEngine::new_with_seed(123);
        assert_ne!(engine.current_behavior(), BehaviorType::Sleep);

        // Toggle sleep on
        let cmd = engine.handle_interaction(InteractionType::SleepToggle);
        assert_eq!(cmd.unwrap().behavior, BehaviorType::Sleep);
        assert_eq!(engine.current_behavior(), BehaviorType::Sleep);

        // Toggle sleep off (wake)
        let cmd2 = engine.handle_interaction(InteractionType::SleepToggle);
        assert_eq!(cmd2.unwrap().behavior, BehaviorType::Idle);
        assert_eq!(engine.current_behavior(), BehaviorType::Idle);

        // Feed adjusts energy and mood
        let initial_energy = engine.state().energy;
        engine.handle_interaction(InteractionType::Feed);
        assert!(engine.state().energy >= initial_energy);
    }
}
