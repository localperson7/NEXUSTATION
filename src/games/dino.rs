use std::time::Duration;

use crate::config::Difficulty;
use crate::games::{ArcadeGame, GameId, GameStatus};
use crate::input::GameInput;
use crate::rng::Rng;

pub const WORLD_WIDTH: f32 = 120.0;
pub const DINO_X: f32 = 12.0;
const FIXED_DT: f32 = 1.0 / 60.0;
const JUMP_VELOCITY: f32 = 40.0;
const GRAVITY: f32 = 104.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObstacleKind {
    SmallCactus,
    TallCactus,
    CactusCluster,
    PteroMid,
    PteroHigh,
}

impl ObstacleKind {
    pub fn dimensions(self) -> (f32, f32, f32) {
        // (width, height, base_y_above_ground)
        match self {
            Self::SmallCactus => (3.4, 4.2, 0.0),
            Self::TallCactus => (4.0, 5.8, 0.0),
            Self::CactusCluster => (6.8, 4.5, 0.0),
            Self::PteroMid => (6.0, 3.0, 4.6),
            Self::PteroHigh => (6.0, 3.0, 8.4),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Obstacle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub kind: ObstacleKind,
}

impl Obstacle {
    pub fn new(x: f32, kind: ObstacleKind) -> Self {
        let (width, height, y) = kind.dimensions();
        Self {
            x,
            y,
            width,
            height,
            kind,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DinoGame {
    dino_y: f32,
    dino_vy: f32,
    duck_timer: f32,
    jump_buffer: f32,
    distance: f32,
    speed: f32,
    spawn_cooldown: f32,
    anim_time: f32,
    obstacles: Vec<Obstacle>,
    difficulty: Difficulty,
    status: GameStatus,
    accum: Duration,
}

impl DinoGame {
    pub fn new(difficulty: Difficulty, rng: &mut Rng) -> Self {
        let mut game = Self {
            dino_y: 0.0,
            dino_vy: 0.0,
            duck_timer: 0.0,
            jump_buffer: 0.0,
            distance: 0.0,
            speed: Self::base_speed(difficulty),
            spawn_cooldown: 42.0,
            anim_time: 0.0,
            obstacles: Vec::new(),
            difficulty,
            status: GameStatus::Playing,
            accum: Duration::ZERO,
        };
        game.reset(difficulty, rng);
        game
    }

    pub fn base_speed(difficulty: Difficulty) -> f32 {
        match difficulty {
            Difficulty::Easy => 34.0,
            Difficulty::Normal => 42.0,
            Difficulty::Hard => 52.0,
        }
    }

    pub fn max_speed(difficulty: Difficulty) -> f32 {
        match difficulty {
            Difficulty::Easy => 68.0,
            Difficulty::Normal => 84.0,
            Difficulty::Hard => 98.0,
        }
    }

    pub fn dino_y(&self) -> f32 {
        self.dino_y
    }

    pub fn is_ducking(&self) -> bool {
        self.dino_y <= 0.01 && self.duck_timer > 0.0
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn speed_multiplier(&self) -> f32 {
        self.speed / Self::base_speed(self.difficulty)
    }

    pub fn anim_frame(&self) -> usize {
        ((self.anim_time * 10.0) as usize) % 2
    }

    pub fn ground_offset(&self) -> usize {
        (self.distance as usize) % 120
    }

    pub fn obstacles(&self) -> &[Obstacle] {
        &self.obstacles
    }

    pub fn clear_obstacles(&mut self) {
        self.obstacles.clear();
        self.spawn_cooldown = 999.0;
    }

    pub fn inject_obstacle(&mut self, obstacle: Obstacle) {
        self.obstacles.push(obstacle);
    }

    /// Returns `(x, y, width, height)` hitbox for the player dino.
    pub fn dino_hitbox(&self) -> (f32, f32, f32, f32) {
        if self.is_ducking() {
            (DINO_X + 0.4, self.dino_y, 6.2, 3.6)
        } else {
            (DINO_X + 0.6, self.dino_y, 4.6, 6.4)
        }
    }

    fn aabb_overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
        a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
    }

    pub fn step_physics(&mut self, dt: f32, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }

        self.anim_time += dt;
        self.duck_timer = (self.duck_timer - dt).max(0.0);
        self.jump_buffer = (self.jump_buffer - dt).max(0.0);

        // Trigger buffered jump as soon as grounded
        if self.jump_buffer > 0.0 && self.dino_y <= 0.01 {
            self.dino_vy = JUMP_VELOCITY;
            self.jump_buffer = 0.0;
            self.duck_timer = 0.0;
        }

        // Integrate vertical motion
        if self.dino_y > 0.0 || self.dino_vy > 0.0 {
            self.dino_y += self.dino_vy * dt;
            self.dino_vy -= GRAVITY * dt;
            if self.dino_y <= 0.0 {
                self.dino_y = 0.0;
                self.dino_vy = 0.0;
            }
        }

        // Advance world scroll and smoothly ramp speed
        let delta_dist = self.speed * dt;
        self.distance += delta_dist;
        let accel = match self.difficulty {
            Difficulty::Easy => 0.55,
            Difficulty::Normal => 0.85,
            Difficulty::Hard => 1.15,
        };
        self.speed = (self.speed + accel * dt).min(Self::max_speed(self.difficulty));

        // Move obstacles and cull passed ones
        for obs in &mut self.obstacles {
            obs.x -= delta_dist;
        }
        self.obstacles.retain(|obs| obs.x + obs.width > -4.0);

        // Spawn next obstacle with speed-scaled safe gap
        self.spawn_cooldown -= delta_dist;
        if self.spawn_cooldown <= 0.0 {
            let allow_sky = self.distance > 140.0;
            let kind = match rng.gen_range(if allow_sky { 7 } else { 5 }) {
                0 | 1 => ObstacleKind::SmallCactus,
                2 | 3 => ObstacleKind::TallCactus,
                4 => ObstacleKind::CactusCluster,
                5 => ObstacleKind::PteroMid,
                _ => ObstacleKind::PteroHigh,
            };
            self.obstacles.push(Obstacle::new(WORLD_WIDTH + 4.0, kind));
            let min_gap = 28.0 + self.speed * 0.45;
            let extra_gap = rng.gen_f32() * 26.0;
            self.spawn_cooldown = min_gap + extra_gap;
        }

        // Check collisions
        let dino_box = self.dino_hitbox();
        for obs in &self.obstacles {
            if Self::aabb_overlap(dino_box, (obs.x, obs.y, obs.width, obs.height)) {
                self.status = GameStatus::GameOver;
                break;
            }
        }
    }
}

impl ArcadeGame for DinoGame {
    fn id(&self) -> GameId {
        GameId::DinoRun
    }

    fn reset(&mut self, difficulty: Difficulty, _rng: &mut Rng) {
        self.dino_y = 0.0;
        self.dino_vy = 0.0;
        self.duck_timer = 0.0;
        self.jump_buffer = 0.0;
        self.distance = 0.0;
        self.speed = Self::base_speed(difficulty);
        self.spawn_cooldown = 44.0;
        self.anim_time = 0.0;
        self.obstacles.clear();
        self.difficulty = difficulty;
        self.status = GameStatus::Playing;
        self.accum = Duration::ZERO;
    }

    fn handle_input(&mut self, input: GameInput, _rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        match input {
            GameInput::Up | GameInput::Action => {
                if self.dino_y <= 0.01 {
                    self.dino_vy = JUMP_VELOCITY;
                    self.duck_timer = 0.0;
                } else {
                    self.jump_buffer = 0.10;
                }
            }
            GameInput::Down => {
                if self.dino_y > 0.05 {
                    self.dino_vy = self.dino_vy.min(-48.0);
                } else {
                    self.duck_timer = 0.28;
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self, dt: Duration, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        self.accum += dt.min(Duration::from_millis(100));
        let step = Duration::from_secs_f32(FIXED_DT);
        while self.status == GameStatus::Playing && self.accum >= step {
            self.accum -= step;
            self.step_physics(FIXED_DT, rng);
        }
    }

    fn score(&self) -> u32 {
        (self.distance * 0.5) as u32
    }

    fn status(&self) -> GameStatus {
        self.status
    }

    fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    fn stats(&self) -> Vec<(&'static str, String)> {
        vec![
            ("DISTANCE", format!("{}m", self.distance as u32)),
            ("SPEED", format!("{:.1}x", self.speed_multiplier())),
            (
                "STATE",
                if self.dino_y > 0.05 {
                    "AIRBORNE".to_string()
                } else if self.is_ducking() {
                    "CROUCH".to_string()
                } else {
                    "SPRINT".to_string()
                },
            ),
        ]
    }
}
