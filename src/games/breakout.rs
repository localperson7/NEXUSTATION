use std::time::Duration;

use crate::config::Difficulty;
use crate::games::{ArcadeGame, GameId, GameStatus};
use crate::input::GameInput;
use crate::rng::Rng;

pub const ARENA_W: f32 = 80.0;
pub const ARENA_H: f32 = 44.0;
pub const PADDLE_Y: f32 = 40.5;
pub const BRICK_COLS: usize = 10;
pub const BRICK_ROWS: usize = 5;
pub const STARTING_LIVES: u32 = 3;

const FIXED_DT: f32 = 1.0 / 60.0;
const BALL_RADIUS: f32 = 0.85;

#[derive(Debug, Clone, Copy)]
pub struct Brick {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub row: usize,
    pub hp: u8,
    pub max_hp: u8,
}

#[derive(Debug, Clone)]
pub struct BreakoutGame {
    paddle_x: f32,
    paddle_vx: f32,
    paddle_hold_timer: f32,
    ball_x: f32,
    ball_y: f32,
    ball_vx: f32,
    ball_vy: f32,
    ball_attached: bool,
    bricks: Vec<Brick>,
    lives: u32,
    level: u32,
    combo: u32,
    score: u32,
    difficulty: Difficulty,
    status: GameStatus,
    accum: Duration,
}

impl BreakoutGame {
    pub fn new(difficulty: Difficulty, rng: &mut Rng) -> Self {
        let mut game = Self {
            paddle_x: ARENA_W * 0.5,
            paddle_vx: 0.0,
            paddle_hold_timer: 0.0,
            ball_x: ARENA_W * 0.5,
            ball_y: PADDLE_Y - 1.5,
            ball_vx: 0.0,
            ball_vy: 0.0,
            ball_attached: true,
            bricks: Vec::new(),
            lives: STARTING_LIVES,
            level: 1,
            combo: 0,
            score: 0,
            difficulty,
            status: GameStatus::Playing,
            accum: Duration::ZERO,
        };
        game.reset(difficulty, rng);
        game
    }

    pub fn paddle_width(&self) -> f32 {
        match self.difficulty {
            Difficulty::Easy => 16.0,
            Difficulty::Normal => 13.0,
            Difficulty::Hard => 10.5,
        }
    }

    pub fn paddle_x(&self) -> f32 {
        self.paddle_x
    }

    pub fn ball_pos(&self) -> (f32, f32) {
        (self.ball_x, self.ball_y)
    }

    pub fn ball_vel(&self) -> (f32, f32) {
        (self.ball_vx, self.ball_vy)
    }

    pub fn is_ball_attached(&self) -> bool {
        self.ball_attached
    }

    pub fn bricks(&self) -> &[Brick] {
        &self.bricks
    }

    pub fn bricks_mut(&mut self) -> &mut Vec<Brick> {
        &mut self.bricks
    }

    pub fn remaining_bricks(&self) -> usize {
        self.bricks.iter().filter(|b| b.hp > 0).count()
    }

    pub fn lives(&self) -> u32 {
        self.lives
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn set_ball(&mut self, x: f32, y: f32, vx: f32, vy: f32) {
        self.ball_x = x;
        self.ball_y = y;
        self.ball_vx = vx;
        self.ball_vy = vy;
        self.ball_attached = false;
    }

    fn base_speed(&self) -> f32 {
        let base = match self.difficulty {
            Difficulty::Easy => 28.0,
            Difficulty::Normal => 34.0,
            Difficulty::Hard => 41.0,
        };
        (base + (self.level.saturating_sub(1) as f32) * 3.0).min(58.0)
    }

    pub fn launch_ball(&mut self, rng: &mut Rng) {
        if !self.ball_attached {
            return;
        }
        let speed = self.base_speed();
        let angle = -std::f32::consts::FRAC_PI_2 + (rng.gen_f32() - 0.5) * 0.65;
        self.ball_vx = speed * angle.cos();
        self.ball_vy = -speed * angle.sin().abs();
        self.ball_attached = false;
    }

    fn attach_ball(&mut self) {
        self.ball_attached = true;
        self.ball_x = self.paddle_x;
        self.ball_y = PADDLE_Y - 1.6;
        self.ball_vx = 0.0;
        self.ball_vy = 0.0;
        self.combo = 0;
    }

    fn build_level_bricks(&mut self, level: u32) {
        self.bricks.clear();
        let margin_x = 4.0_f32;
        let gap_x = 0.9_f32;
        let total_w = ARENA_W - margin_x * 2.0;
        let brick_w = (total_w - gap_x * ((BRICK_COLS - 1) as f32)) / (BRICK_COLS as f32);
        let brick_h = 2.0_f32;
        let gap_y = 0.7_f32;
        let top_y = 4.5_f32;

        let pattern = (level.saturating_sub(1)) % 4;
        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                let hp: u8 = match pattern {
                    0 => {
                        // Classic full wall; top row armored
                        if row == 0 { 2 } else { 1 }
                    }
                    1 => {
                        // Diamond fortress
                        let dr = (row as i32 - 2).abs();
                        let dc = (col as i32 - 4).min((col as i32 - 5).abs()).abs();
                        if dr + dc <= 4 {
                            if row <= 1 { 2 } else { 1 }
                        } else {
                            0
                        }
                    }
                    2 => {
                        // Arcade checkerboard
                        if (row + col) % 2 == 0 {
                            if row == 0 || row == 2 { 2 } else { 1 }
                        } else {
                            0
                        }
                    }
                    _ => {
                        // Twin citadels with center channel
                        if col == 4 || col == 5 {
                            0
                        } else if row <= 1 {
                            2
                        } else {
                            1
                        }
                    }
                };

                if hp > 0 {
                    let x = margin_x + (col as f32) * (brick_w + gap_x);
                    let y = top_y + (row as f32) * (brick_h + gap_y);
                    self.bricks.push(Brick {
                        x,
                        y,
                        w: brick_w,
                        h: brick_h,
                        row,
                        hp,
                        max_hp: hp,
                    });
                }
            }
        }
    }

    pub fn step_physics(&mut self, dt: f32) {
        if self.status != GameStatus::Playing {
            return;
        }

        let half_p = self.paddle_width() * 0.5;
        if self.paddle_hold_timer > 0.0 {
            self.paddle_x =
                (self.paddle_x + self.paddle_vx * dt).clamp(half_p + 1.0, ARENA_W - half_p - 1.0);
            self.paddle_hold_timer = (self.paddle_hold_timer - dt).max(0.0);
        } else {
            self.paddle_vx = 0.0;
        }

        if self.ball_attached {
            self.ball_x = self.paddle_x;
            self.ball_y = PADDLE_Y - 1.6;
            return;
        }

        self.ball_x += self.ball_vx * dt;
        self.ball_y += self.ball_vy * dt;

        // Side & top walls
        if self.ball_x - BALL_RADIUS <= 0.5 {
            self.ball_x = 0.5 + BALL_RADIUS;
            self.ball_vx = self.ball_vx.abs();
        } else if self.ball_x + BALL_RADIUS >= ARENA_W - 0.5 {
            self.ball_x = ARENA_W - 0.5 - BALL_RADIUS;
            self.ball_vx = -self.ball_vx.abs();
        }
        if self.ball_y - BALL_RADIUS <= 0.5 {
            self.ball_y = 0.5 + BALL_RADIUS;
            self.ball_vy = self.ball_vy.abs();
        }

        // Paddle bounce
        if self.ball_vy > 0.0
            && self.ball_y + BALL_RADIUS >= PADDLE_Y
            && self.ball_y - BALL_RADIUS <= PADDLE_Y + 1.8
            && (self.ball_x - self.paddle_x).abs() <= half_p + BALL_RADIUS
        {
            let rel = ((self.ball_x - self.paddle_x) / half_p).clamp(-1.0, 1.0);
            let max_angle = 1.12_f32; // ~64 deg from vertical
            let angle = rel * max_angle;
            let speed = self.ball_vx.hypot(self.ball_vy).max(self.base_speed());
            self.ball_y = PADDLE_Y - BALL_RADIUS - 0.1;
            self.ball_vx = speed * angle.sin();
            self.ball_vy = -speed * angle.cos().max(0.32);
            self.combo = 0;
        }

        // Brick collisions (resolve at most one brick per substep for crisp reflection)
        let mut hit_index: Option<(usize, bool)> = None;
        for (idx, brick) in self.bricks.iter().enumerate() {
            if brick.hp == 0 {
                continue;
            }
            let closest_x = self.ball_x.clamp(brick.x, brick.x + brick.w);
            let closest_y = self.ball_y.clamp(brick.y, brick.y + brick.h);
            let dx = self.ball_x - closest_x;
            let dy = self.ball_y - closest_y;
            if dx * dx + dy * dy <= BALL_RADIUS * BALL_RADIUS {
                let overlap_x = (self.ball_x - (brick.x + brick.w * 0.5)).abs() / (brick.w * 0.5);
                let overlap_y = (self.ball_y - (brick.y + brick.h * 0.5)).abs() / (brick.h * 0.5);
                let horizontal_hit = overlap_x > overlap_y;
                hit_index = Some((idx, horizontal_hit));
                break;
            }
        }

        if let Some((idx, horizontal_hit)) = hit_index {
            if horizontal_hit {
                self.ball_vx = -self.ball_vx;
            } else {
                self.ball_vy = -self.ball_vy;
            }
            let brick = &mut self.bricks[idx];
            brick.hp = brick.hp.saturating_sub(1);
            self.combo = self.combo.saturating_add(1);
            let row_bonus = (BRICK_ROWS - brick.row) as u32;
            let pts = if brick.hp == 0 {
                (row_bonus * 20 + self.combo.min(8) * 5) * self.difficulty.score_multiplier()
            } else {
                15 * self.difficulty.score_multiplier()
            };
            self.score = self.score.saturating_add(pts);
        }

        // Check level clear
        if self.remaining_bricks() == 0 {
            self.score = self
                .score
                .saturating_add(250 * self.level * self.difficulty.score_multiplier());
            self.level = self.level.saturating_add(1);
            self.build_level_bricks(self.level);
            self.attach_ball();
            return;
        }

        // Check bottom miss
        if self.ball_y - BALL_RADIUS > ARENA_H + 0.5 {
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.status = GameStatus::GameOver;
            } else {
                self.attach_ball();
            }
        }
    }
}

impl ArcadeGame for BreakoutGame {
    fn id(&self) -> GameId {
        GameId::Breakout
    }

    fn reset(&mut self, difficulty: Difficulty, _rng: &mut Rng) {
        self.difficulty = difficulty;
        self.paddle_x = ARENA_W * 0.5;
        self.paddle_vx = 0.0;
        self.paddle_hold_timer = 0.0;
        self.lives = STARTING_LIVES;
        self.level = 1;
        self.combo = 0;
        self.score = 0;
        self.status = GameStatus::Playing;
        self.accum = Duration::ZERO;
        self.build_level_bricks(1);
        self.attach_ball();
    }

    fn handle_input(&mut self, input: GameInput, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        let half_p = self.paddle_width() * 0.5;
        match input {
            GameInput::Left => {
                self.paddle_x = (self.paddle_x - 3.6).clamp(half_p + 1.0, ARENA_W - half_p - 1.0);
                self.paddle_vx = -48.0;
                self.paddle_hold_timer = 0.13;
                if self.ball_attached {
                    self.ball_x = self.paddle_x;
                }
            }
            GameInput::Right => {
                self.paddle_x = (self.paddle_x + 3.6).clamp(half_p + 1.0, ARENA_W - half_p - 1.0);
                self.paddle_vx = 48.0;
                self.paddle_hold_timer = 0.13;
                if self.ball_attached {
                    self.ball_x = self.paddle_x;
                }
            }
            GameInput::Action | GameInput::Up => {
                self.launch_ball(rng);
            }
            _ => {}
        }
    }

    fn tick(&mut self, dt: Duration, _rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        self.accum += dt.min(Duration::from_millis(100));
        let step = Duration::from_secs_f32(FIXED_DT);
        while self.status == GameStatus::Playing && self.accum >= step {
            self.accum -= step;
            self.step_physics(FIXED_DT);
        }
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn status(&self) -> GameStatus {
        self.status
    }

    fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    fn stats(&self) -> Vec<(&'static str, String)> {
        let hearts = "♥".repeat(self.lives as usize);
        vec![
            ("LEVEL", self.level.to_string()),
            (
                "LIVES",
                if hearts.is_empty() {
                    "0".to_string()
                } else {
                    hearts
                },
            ),
            ("BRICKS", self.remaining_bricks().to_string()),
        ]
    }
}
