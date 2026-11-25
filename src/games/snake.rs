use std::collections::VecDeque;
use std::time::Duration;

use crate::config::Difficulty;
use crate::games::{ArcadeGame, GameId, GameStatus};
use crate::input::GameInput;
use crate::rng::Rng;

pub const GRID_W: i32 = 24;
pub const GRID_H: i32 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn is_opposite(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Up, Self::Down)
                | (Self::Down, Self::Up)
                | (Self::Left, Self::Right)
                | (Self::Right, Self::Left)
        )
    }

    pub fn delta(self) -> (i32, i32) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnakeGame {
    body: VecDeque<GridPoint>,
    dir: Direction,
    turn_queue: VecDeque<Direction>,
    food: GridPoint,
    bonus_food: Option<(GridPoint, u32)>,
    apples_eaten: u32,
    score: u32,
    difficulty: Difficulty,
    status: GameStatus,
    accum: Duration,
}

impl SnakeGame {
    pub fn new(difficulty: Difficulty, rng: &mut Rng) -> Self {
        let mut game = Self {
            body: VecDeque::new(),
            dir: Direction::Right,
            turn_queue: VecDeque::with_capacity(2),
            food: GridPoint { x: 10, y: 9 },
            bonus_food: None,
            apples_eaten: 0,
            score: 0,
            difficulty,
            status: GameStatus::Playing,
            accum: Duration::ZERO,
        };
        game.reset(difficulty, rng);
        game
    }

    /// Deterministic test constructor with explicit segments (`segments[0]` is head).
    pub fn from_segments(
        segments: &[GridPoint],
        dir: Direction,
        food: GridPoint,
        difficulty: Difficulty,
    ) -> Self {
        assert!(!segments.is_empty(), "snake must have at least one segment");
        Self {
            body: segments.iter().copied().collect(),
            dir,
            turn_queue: VecDeque::with_capacity(2),
            food,
            bonus_food: None,
            apples_eaten: 0,
            score: 0,
            difficulty,
            status: GameStatus::Playing,
            accum: Duration::ZERO,
        }
    }

    pub fn body(&self) -> &VecDeque<GridPoint> {
        &self.body
    }

    pub fn direction(&self) -> Direction {
        self.dir
    }

    pub fn food(&self) -> GridPoint {
        self.food
    }

    pub fn bonus_food(&self) -> Option<(GridPoint, u32)> {
        self.bonus_food
    }

    pub fn step_interval(&self) -> Duration {
        let base_ms: u64 = match self.difficulty {
            Difficulty::Easy => 145,
            Difficulty::Normal => 105,
            Difficulty::Hard => 72,
        };
        let speedup = u64::from(self.apples_eaten.min(25)) * 2;
        Duration::from_millis(base_ms.saturating_sub(speedup).max(45))
    }

    pub fn queue_direction(&mut self, next: Direction) {
        if self.turn_queue.len() >= 2 {
            return;
        }
        let reference = self.turn_queue.back().copied().unwrap_or(self.dir);
        if next != reference && !reference.is_opposite(next) {
            self.turn_queue.push_back(next);
        }
    }

    /// Advances the snake by exactly one discrete cell step.
    pub fn step_once(&mut self, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }

        if let Some(queued) = self.turn_queue.pop_front()
            && !self.dir.is_opposite(queued)
        {
            self.dir = queued;
        }

        let head = *self.body.front().expect("snake has head");
        let (dx, dy) = self.dir.delta();
        let next_head = GridPoint {
            x: head.x + dx,
            y: head.y + dy,
        };

        // Wall collision check
        if next_head.x < 0 || next_head.x >= GRID_W || next_head.y < 0 || next_head.y >= GRID_H {
            self.status = GameStatus::GameOver;
            return;
        }

        let eating_regular = next_head == self.food;
        let eating_bonus = self.bonus_food.is_some_and(|(pos, _)| pos == next_head);
        let growing = eating_regular || eating_bonus;

        // Self-collision check (tail cell is vacated if not growing)
        let check_len = if growing {
            self.body.len()
        } else {
            self.body.len().saturating_sub(1)
        };
        if self
            .body
            .iter()
            .take(check_len)
            .any(|&seg| seg == next_head)
        {
            self.status = GameStatus::GameOver;
            return;
        }

        self.body.push_front(next_head);
        if !growing {
            self.body.pop_back();
        }

        // Tick down bonus food timer
        if let Some((_, ttl)) = &mut self.bonus_food {
            *ttl = ttl.saturating_sub(1);
            if *ttl == 0 {
                self.bonus_food = None;
            }
        }

        if eating_regular {
            self.apples_eaten = self.apples_eaten.saturating_add(1);
            let pts = 10 * self.difficulty.score_multiplier();
            self.score = self.score.saturating_add(pts);
            if !self.spawn_food(rng) {
                self.status = GameStatus::Won;
                return;
            }
            if self.apples_eaten.is_multiple_of(5)
                && self.bonus_food.is_none()
                && let Some(pos) = self.find_empty_cell(rng)
            {
                self.bonus_food = Some((pos, 28));
            }
        } else if eating_bonus {
            let bonus_pts = 35 * self.difficulty.score_multiplier();
            self.score = self.score.saturating_add(bonus_pts);
            self.bonus_food = None;
        }
    }

    fn find_empty_cell(&self, rng: &mut Rng) -> Option<GridPoint> {
        let mut empties = Vec::with_capacity((GRID_W * GRID_H) as usize);
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let pt = GridPoint { x, y };
                let occupied = self.body.contains(&pt)
                    || pt == self.food
                    || self.bonus_food.is_some_and(|(b, _)| b == pt);
                if !occupied {
                    empties.push(pt);
                }
            }
        }
        if empties.is_empty() {
            None
        } else {
            Some(empties[rng.gen_range(empties.len())])
        }
    }

    fn spawn_food(&mut self, rng: &mut Rng) -> bool {
        let mut empties = Vec::with_capacity((GRID_W * GRID_H) as usize);
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let pt = GridPoint { x, y };
                let occupied =
                    self.body.contains(&pt) || self.bonus_food.is_some_and(|(b, _)| b == pt);
                if !occupied {
                    empties.push(pt);
                }
            }
        }
        if empties.is_empty() {
            false
        } else {
            self.food = empties[rng.gen_range(empties.len())];
            true
        }
    }
}

impl ArcadeGame for SnakeGame {
    fn id(&self) -> GameId {
        GameId::Snake
    }

    fn reset(&mut self, difficulty: Difficulty, rng: &mut Rng) {
        self.body.clear();
        let cy = GRID_H / 2;
        let cx = GRID_W / 3;
        self.body.push_back(GridPoint { x: cx, y: cy });
        self.body.push_back(GridPoint { x: cx - 1, y: cy });
        self.body.push_back(GridPoint { x: cx - 2, y: cy });
        self.dir = Direction::Right;
        self.turn_queue.clear();
        self.bonus_food = None;
        self.apples_eaten = 0;
        self.score = 0;
        self.difficulty = difficulty;
        self.status = GameStatus::Playing;
        self.accum = Duration::ZERO;
        self.food = GridPoint { x: 0, y: 0 };
        self.spawn_food(rng);
    }

    fn handle_input(&mut self, input: GameInput, _rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        match input {
            GameInput::Up => self.queue_direction(Direction::Up),
            GameInput::Down => self.queue_direction(Direction::Down),
            GameInput::Left => self.queue_direction(Direction::Left),
            GameInput::Right => self.queue_direction(Direction::Right),
            _ => {}
        }
    }

    fn tick(&mut self, dt: Duration, rng: &mut Rng) {
        if self.status != GameStatus::Playing {
            return;
        }
        self.accum += dt.min(Duration::from_millis(250));
        while self.status == GameStatus::Playing {
            let interval = self.step_interval();
            if self.accum < interval {
                break;
            }
            self.accum -= interval;
            self.step_once(rng);
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
            ("LENGTH", self.body.len().to_string()),
            ("CORES", self.apples_eaten.to_string()),
            ("PACE", format!("{}ms", self.step_interval().as_millis())),
        ]
    }
}
