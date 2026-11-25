use std::time::Duration;

use crate::config::Difficulty;
use crate::games::{ArcadeGame, GameId, GameStatus};
use crate::input::GameInput;
use crate::rng::Rng;

pub const WELL_W: usize = 10;
pub const WELL_H: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Beam = 0,
    Core = 1,
    Pulse = 2,
    WaveS = 3,
    BoltZ = 4,
    HookJ = 5,
    WingL = 6,
}

impl BlockKind {
    pub const ALL: [Self; 7] = [
        Self::Beam,
        Self::Core,
        Self::Pulse,
        Self::WaveS,
        Self::BoltZ,
        Self::HookJ,
        Self::WingL,
    ];

    pub fn color_index(self) -> u8 {
        self as u8
    }

    /// Returns the 4 `(x, y)` block offsets inside a 4x4 local bounding box for `rotation` (`0..4`).
    pub fn cells(self, rotation: u8) -> [(i32, i32); 4] {
        let r = rotation & 3;
        match self {
            Self::Beam => match r {
                0 => [(0, 1), (1, 1), (2, 1), (3, 1)],
                1 => [(2, 0), (2, 1), (2, 2), (2, 3)],
                2 => [(0, 2), (1, 2), (2, 2), (3, 2)],
                _ => [(1, 0), (1, 1), (1, 2), (1, 3)],
            },
            Self::Core => [(1, 0), (2, 0), (1, 1), (2, 1)],
            Self::Pulse => match r {
                0 => [(1, 0), (0, 1), (1, 1), (2, 1)],
                1 => [(1, 0), (1, 1), (2, 1), (1, 2)],
                2 => [(0, 1), (1, 1), (2, 1), (1, 2)],
                _ => [(1, 0), (0, 1), (1, 1), (1, 2)],
            },
            Self::WaveS => match r {
                0 => [(1, 0), (2, 0), (0, 1), (1, 1)],
                1 => [(1, 0), (1, 1), (2, 1), (2, 2)],
                2 => [(1, 1), (2, 1), (0, 2), (1, 2)],
                _ => [(0, 0), (0, 1), (1, 1), (1, 2)],
            },
            Self::BoltZ => match r {
                0 => [(0, 0), (1, 0), (1, 1), (2, 1)],
                1 => [(2, 0), (1, 1), (2, 1), (1, 2)],
                2 => [(0, 1), (1, 1), (1, 2), (2, 2)],
                _ => [(1, 0), (0, 1), (1, 1), (0, 2)],
            },
            Self::HookJ => match r {
                0 => [(0, 0), (0, 1), (1, 1), (2, 1)],
                1 => [(1, 0), (2, 0), (1, 1), (1, 2)],
                2 => [(0, 1), (1, 1), (2, 1), (2, 2)],
                _ => [(1, 0), (1, 1), (0, 2), (1, 2)],
            },
            Self::WingL => match r {
                0 => [(2, 0), (0, 1), (1, 1), (2, 1)],
                1 => [(1, 0), (1, 1), (1, 2), (2, 2)],
                2 => [(0, 1), (1, 1), (2, 1), (0, 2)],
                _ => [(0, 0), (1, 0), (1, 1), (1, 2)],
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePiece {
    pub kind: BlockKind,
    pub x: i32,
    pub y: i32,
    pub rotation: u8,
}

impl ActivePiece {
    pub fn world_cells(self) -> [(i32, i32); 4] {
        self.kind
            .cells(self.rotation)
            .map(|(cx, cy)| (self.x + cx, self.y + cy))
    }
}

#[derive(Debug, Clone)]
pub struct BlockfallGame {
    well: [[Option<u8>; WELL_W]; WELL_H],
    active: ActivePiece,
    next_piece: BlockKind,
    bag: Vec<BlockKind>,
    score: u32,
    lines_cleared: u32,
    level: u32,
    difficulty: Difficulty,
    status: GameStatus,
    accum: Duration,
}

impl BlockfallGame {
    pub fn new(difficulty: Difficulty, rng: &mut Rng) -> Self {
        let mut game = Self {
            well: [[None; WELL_W]; WELL_H],
            active: ActivePiece {
                kind: BlockKind::Pulse,
                x: 3,
                y: 0,
                rotation: 0,
            },
            next_piece: BlockKind::Beam,
            bag: Vec::with_capacity(7),
            score: 0,
            lines_cleared: 0,
            level: 1,
            difficulty,
            status: GameStatus::Playing,
            accum: Duration::ZERO,
        };
        game.reset(difficulty, rng);
        game
    }

    pub fn well(&self) -> &[[Option<u8>; WELL_W]; WELL_H] {
        &self.well
    }

    pub fn active(&self) -> ActivePiece {
        self.active
    }

    pub fn next_piece(&self) -> BlockKind {
        self.next_piece
    }

    pub fn lines_cleared(&self) -> u32 {
        self.lines_cleared
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn set_cell(&mut self, x: usize, y: usize, val: Option<u8>) {
        if x < WELL_W && y < WELL_H {
            self.well[y][x] = val;
        }
    }

    pub fn set_active_piece(&mut self, piece: ActivePiece) {
        self.active = piece;
    }

    pub fn gravity_interval(&self) -> Duration {
        let base_ms: u64 = match self.difficulty {
            Difficulty::Easy => 760,
            Difficulty::Normal => 560,
            Difficulty::Hard => 380,
        };
        let reduction = u64::from(self.level.saturating_sub(1)) * 42;
        Duration::from_millis(base_ms.saturating_sub(reduction).max(65))
    }

    pub fn can_place(&self, candidate: ActivePiece) -> bool {
        for (wx, wy) in candidate.world_cells() {
            if wx < 0 || wx >= WELL_W as i32 || wy >= WELL_H as i32 {
                return false;
            }
            if wy >= 0 && self.well[wy as usize][wx as usize].is_some() {
                return false;
            }
        }
        true
    }

    pub fn ghost_y(&self) -> i32 {
        let mut probe = self.active;
        while self.can_place(ActivePiece {
            y: probe.y + 1,
            ..probe
        }) {
            probe.y += 1;
        }
        probe.y
    }

    pub fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let candidate = ActivePiece {
            x: self.active.x + dx,
            y: self.active.y + dy,
            ..self.active
        };
        if self.can_place(candidate) {
            self.active = candidate;
            true
        } else {
            false
        }
    }

    pub fn try_rotate(&mut self, clockwise: bool) -> bool {
        let next_rot = if clockwise {
            (self.active.rotation + 1) & 3
        } else {
            (self.active.rotation + 3) & 3
        };
        let kicks = [(0, 0), (-1, 0), (1, 0), (0, -1), (-2, 0), (2, 0)];
        for (kx, ky) in kicks {
            let candidate = ActivePiece {
                x: self.active.x + kx,
                y: self.active.y + ky,
                rotation: next_rot,
                ..self.active
            };
            if self.can_place(candidate) {
                self.active = candidate;
                return true;
            }
        }
        false
    }

    pub fn hard_drop(&mut self, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        let target_y = self.ghost_y();
        let dropped = (target_y - self.active.y).max(0) as u32;
        self.active.y = target_y;
        self.score = self.score.saturating_add(dropped * 2);
        self.lock_piece(rng);
    }

    /// Clears all full rows in the well and returns how many rows were cleared.
    pub fn clear_full_lines(&mut self) -> u32 {
        let mut next_well = [[None; WELL_W]; WELL_H];
        let mut write_row = WELL_H;
        let mut cleared = 0u32;

        for read_row in (0..WELL_H).rev() {
            let full = self.well[read_row].iter().all(Option::is_some);
            if full {
                cleared += 1;
            } else {
                write_row -= 1;
                next_well[write_row] = self.well[read_row];
            }
        }

        if cleared > 0 {
            self.well = next_well;
            self.lines_cleared = self.lines_cleared.saturating_add(cleared);
            self.level = 1 + self.lines_cleared / 10;
            let base_pts = match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                _ => 800,
            };
            self.score = self.score.saturating_add(base_pts * self.level);
        }

        cleared
    }

    fn draw_from_bag(&mut self, rng: &mut Rng) -> BlockKind {
        if self.bag.is_empty() {
            self.bag.extend_from_slice(&BlockKind::ALL);
            rng.shuffle(&mut self.bag);
        }
        self.bag.pop().unwrap_or(BlockKind::Beam)
    }

    fn spawn_next(&mut self, rng: &mut Rng) {
        let kind = self.next_piece;
        self.next_piece = self.draw_from_bag(rng);
        let spawned = ActivePiece {
            kind,
            x: 3,
            y: 0,
            rotation: 0,
        };
        if self.can_place(spawned) {
            self.active = spawned;
        } else {
            self.active = spawned;
            self.status = GameStatus::GameOver;
        }
    }

    fn lock_piece(&mut self, rng: &mut Rng) {
        let color = self.active.kind.color_index();
        let mut above_top = false;
        for (wx, wy) in self.active.world_cells() {
            if wy < 0 {
                above_top = true;
            } else if (wx as usize) < WELL_W && (wy as usize) < WELL_H {
                self.well[wy as usize][wx as usize] = Some(color);
            }
        }
        if above_top {
            self.status = GameStatus::GameOver;
            return;
        }
        self.clear_full_lines();
        self.accum = Duration::ZERO;
        self.spawn_next(rng);
    }

    pub fn step_gravity(&mut self, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock_piece(rng);
        }
    }
}

impl ArcadeGame for BlockfallGame {
    fn id(&self) -> GameId {
        GameId::Blockfall
    }

    fn reset(&mut self, difficulty: Difficulty, rng: &mut Rng) {
        self.well = [[None; WELL_W]; WELL_H];
        self.bag.clear();
        self.score = 0;
        self.lines_cleared = 0;
        self.level = 1;
        self.difficulty = difficulty;
        self.status = GameStatus::Playing;
        self.accum = Duration::ZERO;
        self.next_piece = self.draw_from_bag(rng);
        self.spawn_next(rng);
    }

    fn handle_input(&mut self, input: GameInput, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        match input {
            GameInput::Left => {
                self.try_move(-1, 0);
            }
            GameInput::Right => {
                self.try_move(1, 0);
            }
            GameInput::Down => {
                if self.try_move(0, 1) {
                    self.score = self.score.saturating_add(1);
                    self.accum = Duration::ZERO;
                }
            }
            GameInput::Up => {
                self.try_rotate(true);
            }
            GameInput::RotateCcw => {
                self.try_rotate(false);
            }
            GameInput::Action => {
                self.hard_drop(rng);
            }
            GameInput::ContinueEndless => {}
        }
    }

    fn tick(&mut self, dt: Duration, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        self.accum += dt.min(Duration::from_millis(250));
        while self.status == GameStatus::Playing {
            let interval = self.gravity_interval();
            if self.accum < interval {
                break;
            }
            self.accum -= interval;
            self.step_gravity(rng);
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
            ("LEVEL", self.level.to_string()),
            ("LINES", self.lines_cleared.to_string()),
            (
                "SPEED",
                format!("{}ms", self.gravity_interval().as_millis()),
            ),
        ]
    }
}
