pub mod blockfall;
pub mod breakout;
pub mod dino;
pub mod game_2048;
pub mod pong;
pub mod snake;

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::Difficulty;
use crate::games::{
    blockfall::BlockfallGame, breakout::BreakoutGame, dino::DinoGame, game_2048::Game2048,
    pong::PongGame, snake::SnakeGame,
};
use crate::input::GameInput;
use crate::rng::Rng;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum GameId {
    #[default]
    Game2048,
    Snake,
    DinoRun,
    Blockfall,
    Pong,
    Breakout,
}

impl GameId {
    pub const ALL: [Self; 6] = [
        Self::Game2048,
        Self::Snake,
        Self::DinoRun,
        Self::Blockfall,
        Self::Pong,
        Self::Breakout,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Game2048 => "2048",
            Self::Snake => "SNAKE",
            Self::DinoRun => "DINO RUN",
            Self::Blockfall => "BLOCKFALL",
            Self::Pong => "PONG",
            Self::Breakout => "BREAKOUT",
        }
    }

    pub fn cartridge_code(self) -> &'static str {
        match self {
            Self::Game2048 => "NX-01",
            Self::Snake => "NX-02",
            Self::DinoRun => "NX-03",
            Self::Blockfall => "NX-04",
            Self::Pong => "NX-05",
            Self::Breakout => "NX-06",
        }
    }

    pub fn genre(self) -> &'static str {
        match self {
            Self::Game2048 => "LOGIC PUZZLE",
            Self::Snake => "ARCADE ACTION",
            Self::DinoRun => "ENDLESS RUNNER",
            Self::Blockfall => "MATRIX PUZZLE",
            Self::Pong => "RETRO SPORTS",
            Self::Breakout => "BRICK SMASHER",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            Self::Game2048 => "Slide & fuse power-of-two tiles on a 4x4 grid to forge 2048.",
            Self::Snake => {
                "Guide the cyber-serpent, devour energy cores, and never bite your tail."
            }
            Self::DinoRun => "Sprint across the desert horizon, vault cacti, and duck sky-raptors.",
            Self::Blockfall => "Rotate seven orbital polyominoes, stack cleanly, and clear lines.",
            Self::Pong => "Duel the arcade CPU in high-speed table-tennis rallies to 7 points.",
            Self::Breakout => {
                "Ricochet energy spheres off your paddle to shatter multi-stage walls."
            }
        }
    }

    pub fn controls_summary(self) -> &'static str {
        match self {
            Self::Game2048 => "[←↑↓→ / WASD] Slide   [R] Restart   [P/ESC] Pause   [Q] Menu",
            Self::Snake => "[←↑↓→ / WASD] Steer   [R] Restart   [P/ESC] Pause   [Q] Menu",
            Self::DinoRun => "[SPACE/↑/W] Jump   [↓/S] Crouch/Drop   [P/ESC] Pause   [Q] Menu",
            Self::Blockfall => "[←→] Move  [↑/W] Rotate  [Z] CCW  [↓] Soft  [SPACE] Hard Drop",
            Self::Pong => "[↑↓ / WS] Move Paddle   [R] Restart   [P/ESC] Pause   [Q] Menu",
            Self::Breakout => {
                "[←→ / AD] Move Paddle   [SPACE] Launch Ball   [P/ESC] Pause   [Q] Menu"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Playing,
    Won,
    GameOver,
}

/// Reusable lifecycle contract implemented independently by every NEXUSTATION game.
pub trait ArcadeGame {
    fn id(&self) -> GameId;
    fn reset(&mut self, difficulty: Difficulty, rng: &mut Rng);
    fn handle_input(&mut self, input: GameInput, rng: &mut Rng);
    fn tick(&mut self, dt: Duration, rng: &mut Rng);
    fn score(&self) -> u32;
    fn status(&self) -> GameStatus;
    fn difficulty(&self) -> Difficulty;
    fn stats(&self) -> Vec<(&'static str, String)>;
    fn is_realtime(&self) -> bool {
        true
    }
}

/// Type-safe active cartridge wrapper delegating lifecycle operations without `dyn Any`.
#[derive(Debug, Clone)]
pub enum ActiveCartridge {
    Game2048(Game2048),
    Snake(SnakeGame),
    Dino(DinoGame),
    Blockfall(Box<BlockfallGame>),
    Pong(PongGame),
    Breakout(BreakoutGame),
}

impl ActiveCartridge {
    pub fn launch(id: GameId, difficulty: Difficulty, rng: &mut Rng) -> Self {
        match id {
            GameId::Game2048 => Self::Game2048(Game2048::new(difficulty, rng)),
            GameId::Snake => Self::Snake(SnakeGame::new(difficulty, rng)),
            GameId::DinoRun => Self::Dino(DinoGame::new(difficulty, rng)),
            GameId::Blockfall => Self::Blockfall(Box::new(BlockfallGame::new(difficulty, rng))),
            GameId::Pong => Self::Pong(PongGame::new(difficulty, rng)),
            GameId::Breakout => Self::Breakout(BreakoutGame::new(difficulty, rng)),
        }
    }
}

impl ArcadeGame for ActiveCartridge {
    fn id(&self) -> GameId {
        match self {
            Self::Game2048(g) => g.id(),
            Self::Snake(g) => g.id(),
            Self::Dino(g) => g.id(),
            Self::Blockfall(g) => g.id(),
            Self::Pong(g) => g.id(),
            Self::Breakout(g) => g.id(),
        }
    }

    fn reset(&mut self, difficulty: Difficulty, rng: &mut Rng) {
        match self {
            Self::Game2048(g) => g.reset(difficulty, rng),
            Self::Snake(g) => g.reset(difficulty, rng),
            Self::Dino(g) => g.reset(difficulty, rng),
            Self::Blockfall(g) => g.reset(difficulty, rng),
            Self::Pong(g) => g.reset(difficulty, rng),
            Self::Breakout(g) => g.reset(difficulty, rng),
        }
    }

    fn handle_input(&mut self, input: GameInput, rng: &mut Rng) {
        match self {
            Self::Game2048(g) => g.handle_input(input, rng),
            Self::Snake(g) => g.handle_input(input, rng),
            Self::Dino(g) => g.handle_input(input, rng),
            Self::Blockfall(g) => g.handle_input(input, rng),
            Self::Pong(g) => g.handle_input(input, rng),
            Self::Breakout(g) => g.handle_input(input, rng),
        }
    }

    fn tick(&mut self, dt: Duration, rng: &mut Rng) {
        match self {
            Self::Game2048(g) => g.tick(dt, rng),
            Self::Snake(g) => g.tick(dt, rng),
            Self::Dino(g) => g.tick(dt, rng),
            Self::Blockfall(g) => g.tick(dt, rng),
            Self::Pong(g) => g.tick(dt, rng),
            Self::Breakout(g) => g.tick(dt, rng),
        }
    }

    fn score(&self) -> u32 {
        match self {
            Self::Game2048(g) => g.score(),
            Self::Snake(g) => g.score(),
            Self::Dino(g) => g.score(),
            Self::Blockfall(g) => g.score(),
            Self::Pong(g) => g.score(),
            Self::Breakout(g) => g.score(),
        }
    }

    fn status(&self) -> GameStatus {
        match self {
            Self::Game2048(g) => g.status(),
            Self::Snake(g) => g.status(),
            Self::Dino(g) => g.status(),
            Self::Blockfall(g) => g.status(),
            Self::Pong(g) => g.status(),
            Self::Breakout(g) => g.status(),
        }
    }

    fn difficulty(&self) -> Difficulty {
        match self {
            Self::Game2048(g) => g.difficulty(),
            Self::Snake(g) => g.difficulty(),
            Self::Dino(g) => g.difficulty(),
            Self::Blockfall(g) => g.difficulty(),
            Self::Pong(g) => g.difficulty(),
            Self::Breakout(g) => g.difficulty(),
        }
    }

    fn stats(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::Game2048(g) => g.stats(),
            Self::Snake(g) => g.stats(),
            Self::Dino(g) => g.stats(),
            Self::Blockfall(g) => g.stats(),
            Self::Pong(g) => g.stats(),
            Self::Breakout(g) => g.stats(),
        }
    }

    fn is_realtime(&self) -> bool {
        match self {
            Self::Game2048(g) => g.is_realtime(),
            Self::Snake(g) => g.is_realtime(),
            Self::Dino(g) => g.is_realtime(),
            Self::Blockfall(g) => g.is_realtime(),
            Self::Pong(g) => g.is_realtime(),
            Self::Breakout(g) => g.is_realtime(),
        }
    }
}
