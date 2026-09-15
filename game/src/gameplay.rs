//! 搬箱子的纯玩法状态，不依赖 Fyrox 场景和 UI。
use fyrox::core::{algebra::Vector2, reflect::prelude::*, visitor::prelude::*};

const PLAYER_SPEED: f32 = 4.0;
const CONTACT_DISTANCE: f32 = 0.9;
const GOAL_DISTANCE: f32 = 0.65;

#[derive(Default, Debug, Clone, Copy)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Visit, Reflect)]
#[reflect(type_uuid = "c2b7f3c4-0d63-4f88-bf4d-2fd0a2e5f6c1")]
pub struct GameState {
    pub player: Vector2<f32>,
    pub boxes: Vec<Vector2<f32>>,
    pub goals: Vec<Vector2<f32>>,
    pub completed: bool,
}

impl GameState {
    pub fn level() -> Self {
        Self {
            player: Vector2::new(-5.5, 0.0),
            boxes: vec![Vector2::new(-1.5, -2.0), Vector2::new(-1.5, 2.0)],
            goals: vec![Vector2::new(5.0, -2.0), Vector2::new(5.0, 2.0)],
            completed: false,
        }
    }

    /// 推箱采用确定性的运动学规则，避免把玩法状态交给 Lua 或 UI。
    pub fn step(&mut self, input: InputState, dt: f32) -> bool {
        let was_complete = self.completed;
        self.completed = self.is_complete();
        let mut direction = Vector2::new(
            f32::from(input.right) - f32::from(input.left),
            f32::from(input.up) - f32::from(input.down),
        );
        if direction.norm_squared() <= f32::EPSILON {
            return !was_complete && self.completed;
        }
        direction = direction.normalize();
        let frame_time = if dt.is_finite() {
            dt.clamp(0.0, 0.05)
        } else {
            0.0
        };
        let delta = direction * PLAYER_SPEED * frame_time;
        let next = self.player + delta;
        if next.x.abs() > 7.5 || next.y.abs() > 3.8 {
            return false;
        }

        if let Some(index) = self
            .boxes
            .iter()
            .position(|position| (next - *position).norm() < CONTACT_DISTANCE)
        {
            let pushed = self.boxes[index] + delta;
            let blocked = pushed.x.abs() > 7.2
                || pushed.y.abs() > 3.5
                || self.boxes.iter().enumerate().any(|(other, position)| {
                    other != index && (pushed - *position).norm() < CONTACT_DISTANCE
                });
            if blocked {
                return false;
            }
            self.boxes[index] = pushed;
        }
        self.player = next;
        self.completed = self.is_complete();
        !was_complete && self.completed
    }

    fn is_complete(&self) -> bool {
        !self.goals.is_empty()
            && self.boxes.len() >= self.goals.len()
            && self.goals.iter().all(|goal| {
                self.boxes
                    .iter()
                    .any(|position| (*position - *goal).norm() < GOAL_DISTANCE)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_a_box() {
        let mut state = GameState {
            player: Vector2::new(0.0, 0.0),
            boxes: vec![Vector2::new(0.8, 0.0)],
            goals: vec![],
            completed: false,
        };
        state.step(
            InputState {
                right: true,
                ..Default::default()
            },
            0.05,
        );
        assert!(state.player.x > 0.0);
        assert!(state.boxes[0].x > 0.8);
    }

    #[test]
    fn blocked_box_stops_player() {
        let mut state = GameState {
            player: Vector2::new(6.5, 0.0),
            boxes: vec![Vector2::new(7.2, 0.0)],
            goals: vec![],
            completed: false,
        };
        state.step(
            InputState {
                right: true,
                ..Default::default()
            },
            0.05,
        );
        assert_eq!(state.player.x, 6.5);
    }

    #[test]
    fn reports_completion_once() {
        let mut state = GameState {
            player: Vector2::new(0.0, 0.0),
            boxes: vec![Vector2::new(0.8, 0.0)],
            goals: vec![Vector2::new(1.0, 0.0)],
            completed: false,
        };
        assert!(state.step(
            InputState {
                right: true,
                ..Default::default()
            },
            0.05
        ));
        assert!(!state.step(InputState::default(), 0.05));
    }

    #[test]
    fn empty_goals_are_not_complete() {
        let mut state = GameState::default();
        assert!(!state.step(
            InputState {
                right: true,
                ..Default::default()
            },
            0.05,
        ));
        assert!(!state.completed);
    }

    #[test]
    fn detects_completion_without_input() {
        let mut state = GameState {
            player: Vector2::new(0.0, 0.0),
            boxes: vec![Vector2::new(1.0, 0.0)],
            goals: vec![Vector2::new(1.0, 0.0)],
            completed: false,
        };
        assert!(state.step(InputState::default(), 0.05));
        assert!(!state.step(InputState::default(), 0.05));
    }
}
