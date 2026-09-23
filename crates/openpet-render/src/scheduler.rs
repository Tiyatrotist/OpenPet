use openpet_types::BehaviorType;
use std::time::{Duration, Instant};

/// Adaptive frame rate scheduler to optimize battery life and CPU usage.
///
/// Principles:
/// - Active motion: 60 FPS
/// - Minor idle: 24 FPS
/// - Deep sleep: 5 FPS / Event-driven
pub struct FrameScheduler {
    target_fps: u32,
    frame_interval: Duration,
    last_frame_instant: Instant,
}

impl FrameScheduler {
    pub fn new() -> Self {
        let initial_fps = 60;
        Self {
            target_fps: initial_fps,
            frame_interval: Duration::from_nanos(1_000_000_000 / initial_fps as u64),
            last_frame_instant: Instant::now(),
        }
    }

    /// Sets target frame rate based on current behavior state.
    pub fn update_behavior_mode(&mut self, behavior: BehaviorType, is_dragging: bool) {
        let desired_fps =
            if is_dragging || behavior.is_locomotive() || behavior == BehaviorType::Play {
                60
            } else if behavior == BehaviorType::Sleep {
                5
            } else if behavior == BehaviorType::Sit || behavior == BehaviorType::Idle {
                24
            } else {
                30
            };

        if desired_fps != self.target_fps {
            self.target_fps = desired_fps;
            self.frame_interval = Duration::from_nanos(1_000_000_000 / desired_fps as u64);
        }
    }

    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    /// Determines if sufficient time has elapsed to render the next frame.
    pub fn should_render(&self, now: Instant) -> bool {
        now.duration_since(self.last_frame_instant) >= self.frame_interval
    }

    /// Records that a frame was just submitted.
    pub fn mark_frame_rendered(&mut self, now: Instant) {
        self.last_frame_instant = now;
    }
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_fps_scaling() {
        let mut scheduler = FrameScheduler::new();

        scheduler.update_behavior_mode(BehaviorType::Walk, false);
        assert_eq!(scheduler.target_fps(), 60);

        scheduler.update_behavior_mode(BehaviorType::Sleep, false);
        assert_eq!(scheduler.target_fps(), 5);

        scheduler.update_behavior_mode(BehaviorType::Idle, false);
        assert_eq!(scheduler.target_fps(), 24);

        scheduler.update_behavior_mode(BehaviorType::Sleep, true); // dragging
        assert_eq!(scheduler.target_fps(), 60);
    }
}
