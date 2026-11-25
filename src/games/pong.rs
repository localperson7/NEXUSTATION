use std::time::Duration;

use crate::config::Difficulty;
use crate::games::{ArcadeGame, GameId, GameStatus};
use crate::input::GameInput;
use crate::rng::Rng;

pub const COURT_W: f32 = 80.0;
pub const COURT_H: f32 = 40.0;
pub const PLAYER_X: f32 = 4.0;
pub const CPU_X: f32 = COURT_W - 4.0;
pub const PADDLE_W: f32 = 1.6;
pub const WIN_GOALS: u32 = 7;

const FIXED_DT: f32 = 1.0 / 60.0;

#[derive(Debug, Clone)]
pub struct PongGame {
    player_y: f32,
    player_vy: f32,
    player_hold_timer: f32,
    cpu_y: f32,
    ball_x: f32,
    ball_y: f32,
    ball_vx: f32,
    ball_vy: f32,
    trail: [(f32, f32); 4],
    serve_timer: f32,
    player_goals: u32,
    cpu_goals: u32,
    rally: u32,
    best_rally: u32,
    score: u32,
    difficulty: Difficulty,
    status: GameStatus,
    accum: Duration,
}

impl PongGame {
    pub fn new(difficulty: Difficulty, rng: &mut Rng) -> Self {
        let mut game = Self {
            player_y: COURT_H * 0.5,
            player_vy: 0.0,
            player_hold_timer: 0.0,
            cpu_y: COURT_H * 0.5,
            ball_x: COURT_W * 0.5,
            ball_y: COURT_H * 0.5,
            ball_vx: 38.0,
            ball_vy: 10.0,
            trail: [(COURT_W * 0.5, COURT_H * 0.5); 4],
            serve_timer: 0.6,
            player_goals: 0,
            cpu_goals: 0,
            rally: 0,
            best_rally: 0,
            score: 0,
            difficulty,
            status: GameStatus::Playing,
            accum: Duration::ZERO,
        };
        game.reset(difficulty, rng);
        game
    }

    pub fn paddle_height(&self) -> f32 {
        match self.difficulty {
            Difficulty::Easy => 9.5,
            Difficulty::Normal => 8.0,
            Difficulty::Hard => 7.0,
        }
    }

    pub fn player_y(&self) -> f32 {
        self.player_y
    }

    pub fn cpu_y(&self) -> f32 {
        self.cpu_y
    }

    pub fn ball_pos(&self) -> (f32, f32) {
        (self.ball_x, self.ball_y)
    }

    pub fn ball_vel(&self) -> (f32, f32) {
        (self.ball_vx, self.ball_vy)
    }

    pub fn trail(&self) -> &[(f32, f32); 4] {
        &self.trail
    }

    pub fn player_goals(&self) -> u32 {
        self.player_goals
    }

    pub fn cpu_goals(&self) -> u32 {
        self.cpu_goals
    }

    pub fn rally(&self) -> u32 {
        self.rally
    }

    pub fn set_ball(&mut self, x: f32, y: f32, vx: f32, vy: f32) {
        self.ball_x = x;
        self.ball_y = y;
        self.ball_vx = vx;
        self.ball_vy = vy;
        self.serve_timer = 0.0;
    }

    pub fn set_paddles(&mut self, player_y: f32, cpu_y: f32) {
        let half = self.paddle_height() * 0.5;
        self.player_y = player_y.clamp(half, COURT_H - half);
        self.cpu_y = cpu_y.clamp(half, COURT_H - half);
    }

    fn base_ball_speed(&self) -> f32 {
        match self.difficulty {
            Difficulty::Easy => 34.0,
            Difficulty::Normal => 42.0,
            Difficulty::Hard => 50.0,
        }
    }

    fn serve_ball(&mut self, toward_right: bool, rng: &mut Rng) {
        self.ball_x = COURT_W * 0.5;
        self.ball_y = COURT_H * 0.5;
        self.trail = [(self.ball_x, self.ball_y); 4];
        let speed = self.base_ball_speed();
        let angle = (rng.gen_f32() - 0.5) * 0.7;
        let dir = if toward_right { 1.0 } else { -1.0 };
        self.ball_vx = dir * speed * angle.cos();
        self.ball_vy = speed * angle.sin();
        self.rally = 0;
        self.serve_timer = 0.55;
    }

    pub fn step_physics(&mut self, dt: f32, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }

        let half_h = self.paddle_height() * 0.5;

        // Player smooth paddle momentum
        if self.player_hold_timer > 0.0 {
            self.player_y = (self.player_y + self.player_vy * dt).clamp(half_h, COURT_H - half_h);
            self.player_hold_timer = (self.player_hold_timer - dt).max(0.0);
        } else {
            self.player_vy = 0.0;
        }

        // CPU paddle AI tracking
        let (ai_speed, ai_deadzone) = match self.difficulty {
            Difficulty::Easy => (21.0, 2.4),
            Difficulty::Normal => (31.0, 1.1),
            Difficulty::Hard => (44.0, 0.35),
        };
        let ai_target = if self.ball_vx > 0.0 {
            self.ball_y
        } else {
            COURT_H * 0.5
        };
        let diff = ai_target - self.cpu_y;
        if diff.abs() > ai_deadzone {
            let step = diff.signum() * ai_speed * dt;
            self.cpu_y = (self.cpu_y + step).clamp(half_h, COURT_H - half_h);
        }

        if self.serve_timer > 0.0 {
            self.serve_timer = (self.serve_timer - dt).max(0.0);
            return;
        }

        // Update subtle ball trail
        self.trail.rotate_right(1);
        self.trail[0] = (self.ball_x, self.ball_y);

        // Integrate ball motion
        self.ball_x += self.ball_vx * dt;
        self.ball_y += self.ball_vy * dt;

        // Top & bottom wall bounce
        if self.ball_y <= 1.0 {
            self.ball_y = 1.0;
            self.ball_vy = self.ball_vy.abs();
        } else if self.ball_y >= COURT_H - 1.0 {
            self.ball_y = COURT_H - 1.0;
            self.ball_vy = -self.ball_vy.abs();
        }

        // Left (Player) paddle collision
        if self.ball_vx < 0.0
            && self.ball_x <= PLAYER_X + PADDLE_W
            && self.ball_x >= PLAYER_X - 1.5
            && (self.ball_y - self.player_y).abs() <= half_h + 1.0
        {
            let rel = ((self.ball_y - self.player_y) / half_h).clamp(-1.0, 1.0);
            let max_angle = 0.92_f32; // ~53 deg
            let speed = (self.ball_vx.hypot(self.ball_vy) * 1.045).min(78.0);
            let angle = rel * max_angle;
            self.ball_x = PLAYER_X + PADDLE_W + 0.1;
            self.ball_vx = speed * angle.cos();
            self.ball_vy = speed * angle.sin();
            self.rally = self.rally.saturating_add(1);
            self.best_rally = self.best_rally.max(self.rally);
            self.score = self
                .score
                .saturating_add(15 * self.difficulty.score_multiplier());
        }

        // Right (CPU) paddle collision
        if self.ball_vx > 0.0
            && self.ball_x >= CPU_X - PADDLE_W
            && self.ball_x <= CPU_X + 1.5
            && (self.ball_y - self.cpu_y).abs() <= half_h + 1.0
        {
            let rel = ((self.ball_y - self.cpu_y) / half_h).clamp(-1.0, 1.0);
            let max_angle = 0.88_f32;
            let speed = (self.ball_vx.hypot(self.ball_vy) * 1.035).min(78.0);
            let angle = rel * max_angle;
            self.ball_x = CPU_X - PADDLE_W - 0.1;
            self.ball_vx = -speed * angle.cos();
            self.ball_vy = speed * angle.sin();
            self.rally = self.rally.saturating_add(1);
            self.best_rally = self.best_rally.max(self.rally);
        }

        // Goal scoring checks
        if self.ball_x > COURT_W + 1.0 {
            self.player_goals = self.player_goals.saturating_add(1);
            self.score = self
                .score
                .saturating_add(150 * self.difficulty.score_multiplier());
            if self.player_goals >= WIN_GOALS {
                self.score = self
                    .score
                    .saturating_add(300 * self.difficulty.score_multiplier());
                self.status = GameStatus::Won;
            } else {
                self.serve_ball(false, rng);
            }
        } else if self.ball_x < -1.0 {
            self.cpu_goals = self.cpu_goals.saturating_add(1);
            if self.cpu_goals >= WIN_GOALS {
                self.status = GameStatus::GameOver;
            } else {
                self.serve_ball(true, rng);
            }
        }
    }
}

impl ArcadeGame for PongGame {
    fn id(&self) -> GameId {
        GameId::Pong
    }

    fn reset(&mut self, difficulty: Difficulty, rng: &mut Rng) {
        self.difficulty = difficulty;
        self.player_y = COURT_H * 0.5;
        self.player_vy = 0.0;
        self.player_hold_timer = 0.0;
        self.cpu_y = COURT_H * 0.5;
        self.player_goals = 0;
        self.cpu_goals = 0;
        self.rally = 0;
        self.best_rally = 0;
        self.score = 0;
        self.status = GameStatus::Playing;
        self.accum = Duration::ZERO;
        self.serve_ball(true, rng);
    }

    fn handle_input(&mut self, input: GameInput, _rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        let half = self.paddle_height() * 0.5;
        match input {
            GameInput::Up => {
                self.player_y = (self.player_y - 2.2).clamp(half, COURT_H - half);
                self.player_vy = -34.0;
                self.player_hold_timer = 0.14;
            }
            GameInput::Down => {
                self.player_y = (self.player_y + 2.2).clamp(half, COURT_H - half);
                self.player_vy = 34.0;
                self.player_hold_timer = 0.14;
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
        self.score
    }

    fn status(&self) -> GameStatus {
        self.status
    }

    fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    fn stats(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                "MATCH",
                format!("{} : {}", self.player_goals, self.cpu_goals),
            ),
            ("FIRST TO", WIN_GOALS.to_string()),
            ("RALLY", format!("{} (MAX {})", self.rally, self.best_rally)),
        ]
    }
}
