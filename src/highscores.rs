use std::collections::BTreeMap;
use std::fs;
use std::io;

use serde::{Deserialize, Serialize};

use crate::config::{AppPaths, Difficulty, atomic_write_bytes};
use crate::games::GameId;

pub const MAX_LEADERBOARD_ENTRIES: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreEntry {
    pub score: u32,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HighScoreStore {
    boards: BTreeMap<GameId, Vec<ScoreEntry>>,
}

impl HighScoreStore {
    pub fn load_from(paths: &AppPaths) -> Self {
        let path = paths.highscores_file();
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save_to(&self, paths: &AppPaths) -> io::Result<()> {
        paths.ensure_dir()?;
        let serialized = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        atomic_write_bytes(&paths.highscores_file(), serialized.as_bytes(), None)
    }

    pub fn best_score(&self, game: GameId) -> u32 {
        self.boards
            .get(&game)
            .and_then(|entries| entries.first())
            .map(|e| e.score)
            .unwrap_or(0)
    }

    pub fn entries(&self, game: GameId) -> &[ScoreEntry] {
        self.boards.get(&game).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Records a score into the game's top-5 leaderboard.
    /// Returns `true` if this score establishes a strictly higher #1 personal best.
    pub fn record(&mut self, game: GameId, score: u32, difficulty: Difficulty) -> bool {
        if score == 0 {
            return false;
        }
        let previous_best = self.best_score(game);
        let list = self.boards.entry(game).or_default();
        list.push(ScoreEntry { score, difficulty });
        list.sort_by_key(|entry| std::cmp::Reverse(entry.score));
        list.truncate(MAX_LEADERBOARD_ENTRIES);
        score > previous_best
    }

    pub fn total_arcade_points(&self) -> u64 {
        GameId::ALL
            .iter()
            .map(|&g| u64::from(self.best_score(g)))
            .sum()
    }

    pub fn clear(&mut self) {
        self.boards.clear();
    }
}
