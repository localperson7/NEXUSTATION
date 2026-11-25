use std::time::Duration;

use crate::config::Difficulty;
use crate::games::{ArcadeGame, GameId, GameStatus};
use crate::input::GameInput;
use crate::rng::Rng;

pub const BOARD_SIZE: usize = 4;
pub const WIN_TILE: u32 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideDir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub struct Game2048 {
    board: [[u32; BOARD_SIZE]; BOARD_SIZE],
    merged_mask: [[bool; BOARD_SIZE]; BOARD_SIZE],
    last_spawn: Option<(usize, usize)>,
    score: u32,
    moves: u32,
    difficulty: Difficulty,
    status: GameStatus,
    endless_after_win: bool,
    flash_timer_ms: u32,
}

impl Game2048 {
    pub fn new(difficulty: Difficulty, rng: &mut Rng) -> Self {
        let mut game = Self {
            board: [[0; BOARD_SIZE]; BOARD_SIZE],
            merged_mask: [[false; BOARD_SIZE]; BOARD_SIZE],
            last_spawn: None,
            score: 0,
            moves: 0,
            difficulty,
            status: GameStatus::Playing,
            endless_after_win: false,
            flash_timer_ms: 0,
        };
        game.reset(difficulty, rng);
        game
    }

    /// Creates a game with a predetermined board state for deterministic unit tests.
    pub fn from_board(board: [[u32; BOARD_SIZE]; BOARD_SIZE], difficulty: Difficulty) -> Self {
        let mut game = Self {
            board,
            merged_mask: [[false; BOARD_SIZE]; BOARD_SIZE],
            last_spawn: None,
            score: 0,
            moves: 0,
            difficulty,
            status: GameStatus::Playing,
            endless_after_win: false,
            flash_timer_ms: 0,
        };
        game.refresh_status();
        game
    }

    pub fn board(&self) -> &[[u32; BOARD_SIZE]; BOARD_SIZE] {
        &self.board
    }

    pub fn merged_mask(&self) -> &[[bool; BOARD_SIZE]; BOARD_SIZE] {
        &self.merged_mask
    }

    pub fn last_spawn(&self) -> Option<(usize, usize)> {
        self.last_spawn
    }

    pub fn moves(&self) -> u32 {
        self.moves
    }

    pub fn max_tile(&self) -> u32 {
        self.board
            .iter()
            .flat_map(|row| row.iter().copied())
            .max()
            .unwrap_or(0)
    }

    pub fn is_flashing(&self) -> bool {
        self.flash_timer_ms > 0
    }

    /// Slides a single 4-cell line toward index 0 according to canonical 2048 rules.
    pub fn slide_line(line: [u32; BOARD_SIZE]) -> ([u32; BOARD_SIZE], [bool; BOARD_SIZE], u32) {
        let mut compact = [0u32; BOARD_SIZE];
        let mut len = 0usize;
        for v in line {
            if v != 0 {
                compact[len] = v;
                len += 1;
            }
        }

        let mut out = [0u32; BOARD_SIZE];
        let mut merged = [false; BOARD_SIZE];
        let mut gained = 0u32;
        let mut read = 0usize;
        let mut write = 0usize;

        while read < len {
            if read + 1 < len && compact[read] == compact[read + 1] {
                let combined = compact[read] * 2;
                out[write] = combined;
                merged[write] = true;
                gained += combined;
                read += 2;
            } else {
                out[write] = compact[read];
                read += 1;
            }
            write += 1;
        }

        (out, merged, gained)
    }

    /// Performs a board slide; returns `true` if any tile moved or merged.
    pub fn slide(&mut self, dir: SlideDir, rng: &mut Rng) -> bool {
        if self.status != GameStatus::Playing {
            return false;
        }

        let prev = self.board;
        let mut next_board = [[0u32; BOARD_SIZE]; BOARD_SIZE];
        let mut next_merged = [[false; BOARD_SIZE]; BOARD_SIZE];
        let mut total_gain = 0u32;

        match dir {
            SlideDir::Left => {
                for r in 0..BOARD_SIZE {
                    let (line, merged, gain) = Self::slide_line(self.board[r]);
                    next_board[r] = line;
                    next_merged[r] = merged;
                    total_gain += gain;
                }
            }
            SlideDir::Right => {
                for r in 0..BOARD_SIZE {
                    let mut rev = self.board[r];
                    rev.reverse();
                    let (mut line, mut merged, gain) = Self::slide_line(rev);
                    line.reverse();
                    merged.reverse();
                    next_board[r] = line;
                    next_merged[r] = merged;
                    total_gain += gain;
                }
            }
            SlideDir::Up => {
                for c in 0..BOARD_SIZE {
                    let col = [
                        self.board[0][c],
                        self.board[1][c],
                        self.board[2][c],
                        self.board[3][c],
                    ];
                    let (line, merged, gain) = Self::slide_line(col);
                    for r in 0..BOARD_SIZE {
                        next_board[r][c] = line[r];
                        next_merged[r][c] = merged[r];
                    }
                    total_gain += gain;
                }
            }
            SlideDir::Down => {
                for c in 0..BOARD_SIZE {
                    let col = [
                        self.board[3][c],
                        self.board[2][c],
                        self.board[1][c],
                        self.board[0][c],
                    ];
                    let (line, merged, gain) = Self::slide_line(col);
                    for r in 0..BOARD_SIZE {
                        next_board[BOARD_SIZE - 1 - r][c] = line[r];
                        next_merged[BOARD_SIZE - 1 - r][c] = merged[r];
                    }
                    total_gain += gain;
                }
            }
        }

        if next_board == prev {
            return false;
        }

        self.board = next_board;
        self.merged_mask = next_merged;
        self.score = self.score.saturating_add(total_gain);
        self.moves = self.moves.saturating_add(1);
        self.flash_timer_ms = 180;
        self.spawn_random_tile(rng);
        self.refresh_status();
        true
    }

    pub fn spawn_random_tile(&mut self, rng: &mut Rng) -> Option<(usize, usize)> {
        let mut empties = [(0usize, 0usize); BOARD_SIZE * BOARD_SIZE];
        let mut count = 0usize;
        for (r, row) in self.board.iter().enumerate() {
            for (c, &val) in row.iter().enumerate() {
                if val == 0 {
                    empties[count] = (r, c);
                    count += 1;
                }
            }
        }
        if count == 0 {
            self.last_spawn = None;
            return None;
        }
        let chosen = empties[rng.gen_range(count)];
        let four_chance = match self.difficulty {
            Difficulty::Easy => 0.08,
            Difficulty::Normal => 0.10,
            Difficulty::Hard => 0.22,
        };
        let value = if rng.gen_bool(four_chance) { 4 } else { 2 };
        self.board[chosen.0][chosen.1] = value;
        self.last_spawn = Some(chosen);
        Some(chosen)
    }

    pub fn has_valid_moves(&self) -> bool {
        for r in 0..BOARD_SIZE {
            for c in 0..BOARD_SIZE {
                let v = self.board[r][c];
                if v == 0 {
                    return true;
                }
                if r + 1 < BOARD_SIZE && self.board[r + 1][c] == v {
                    return true;
                }
                if c + 1 < BOARD_SIZE && self.board[r][c + 1] == v {
                    return true;
                }
            }
        }
        false
    }

    fn refresh_status(&mut self) {
        if !self.endless_after_win && self.max_tile() >= WIN_TILE {
            self.status = GameStatus::Won;
        } else if !self.has_valid_moves() {
            self.status = GameStatus::GameOver;
        } else {
            self.status = GameStatus::Playing;
        }
    }
}

impl ArcadeGame for Game2048 {
    fn id(&self) -> GameId {
        GameId::Game2048
    }

    fn reset(&mut self, difficulty: Difficulty, rng: &mut Rng) {
        self.board = [[0; BOARD_SIZE]; BOARD_SIZE];
        self.merged_mask = [[false; BOARD_SIZE]; BOARD_SIZE];
        self.last_spawn = None;
        self.score = 0;
        self.moves = 0;
        self.difficulty = difficulty;
        self.status = GameStatus::Playing;
        self.endless_after_win = false;
        self.flash_timer_ms = 0;
        self.spawn_random_tile(rng);
        self.spawn_random_tile(rng);
    }

    fn handle_input(&mut self, input: GameInput, rng: &mut Rng) {
        if self.status == GameStatus::Won && input == GameInput::ContinueEndless {
            self.endless_after_win = true;
            self.refresh_status();
            return;
        }

        match input {
            GameInput::Left => {
                self.slide(SlideDir::Left, rng);
            }
            GameInput::Right => {
                self.slide(SlideDir::Right, rng);
            }
            GameInput::Up => {
                self.slide(SlideDir::Up, rng);
            }
            GameInput::Down => {
                self.slide(SlideDir::Down, rng);
            }
            _ => {}
        }
    }

    fn tick(&mut self, dt: Duration, _rng: &mut Rng) {
        if self.flash_timer_ms > 0 {
            let elapsed = dt.as_millis().min(u128::from(u32::MAX)) as u32;
            self.flash_timer_ms = self.flash_timer_ms.saturating_sub(elapsed);
            if self.flash_timer_ms == 0 {
                self.merged_mask = [[false; BOARD_SIZE]; BOARD_SIZE];
            }
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

    fn is_realtime(&self) -> bool {
        self.flash_timer_ms > 0
    }

    fn stats(&self) -> Vec<(&'static str, String)> {
        vec![
            ("MAX TILE", self.max_tile().to_string()),
            ("MOVES", self.moves.to_string()),
            ("TARGET", WIN_TILE.to_string()),
        ]
    }
}
