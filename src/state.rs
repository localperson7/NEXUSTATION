use std::time::Duration;

use crate::config::{AppPaths, Settings};
use crate::games::{ActiveCartridge, ArcadeGame, GameId, GameStatus};
use crate::highscores::HighScoreStore;
use crate::input::ConsoleEvent;
use crate::rng::Rng;

pub const BOOT_DURATION_MS: u32 = 2100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Startup { elapsed_ms: u32 },
    MainMenu,
    Settings,
    HighScores,
    InGame,
}

pub struct AppState {
    pub screen: Screen,
    pub selected_game: usize,
    pub menu_anim_ms: u32,
    pub idle_tick: u64,
    pub settings_cursor: usize,
    pub confirm_reset_scores: bool,
    pub highscores_cursor: usize,
    pub active_game: Option<ActiveCartridge>,
    pub paused: bool,
    pub pause_cursor: usize,
    pub new_high_score: bool,
    committed_run_score: u32,
    pub settings: Settings,
    pub highscores: HighScoreStore,
    pub paths: AppPaths,
    pub rng: Rng,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(paths: AppPaths, rng: Rng) -> Self {
        let settings = Settings::load_from(&paths);
        let highscores = HighScoreStore::load_from(&paths);
        let initial_screen = if settings.boot_animation {
            Screen::Startup { elapsed_ms: 0 }
        } else {
            Screen::MainMenu
        };
        Self {
            screen: initial_screen,
            selected_game: 0,
            menu_anim_ms: 280,
            idle_tick: 0,
            settings_cursor: 0,
            confirm_reset_scores: false,
            highscores_cursor: 0,
            active_game: None,
            paused: false,
            pause_cursor: 0,
            new_high_score: false,
            committed_run_score: 0,
            settings,
            highscores,
            paths,
            rng,
            should_quit: false,
        }
    }

    pub fn current_game_id(&self) -> GameId {
        GameId::ALL[self.selected_game % GameId::ALL.len()]
    }

    /// Adaptive poll timeout: 60Hz during active real-time gameplay, 30Hz during short UI
    /// transitions, and low-frequency ~3-4Hz sleep during idle menus/pause to keep CPU near 0%.
    pub fn poll_timeout(&self) -> Duration {
        match self.screen {
            Screen::Startup { .. } => Duration::from_millis(33),
            Screen::MainMenu => {
                if self.menu_anim_ms > 0 {
                    Duration::from_millis(33)
                } else {
                    Duration::from_millis(320)
                }
            }
            Screen::Settings | Screen::HighScores => Duration::from_millis(320),
            Screen::InGame => {
                if self.paused {
                    return Duration::from_millis(250);
                }
                match &self.active_game {
                    Some(game) if game.status() == GameStatus::Playing && game.is_realtime() => {
                        Duration::from_millis(16)
                    }
                    _ => Duration::from_millis(250),
                }
            }
        }
    }

    pub fn launch_game(&mut self, id: GameId) {
        if let Some(idx) = GameId::ALL.iter().position(|&g| g == id) {
            self.selected_game = idx;
        }
        let cartridge = ActiveCartridge::launch(id, self.settings.difficulty, &mut self.rng);
        self.active_game = Some(cartridge);
        self.paused = false;
        self.pause_cursor = 0;
        self.new_high_score = false;
        self.committed_run_score = 0;
        self.screen = Screen::InGame;
    }

    pub fn restart_active_game(&mut self) {
        self.commit_score_if_needed();
        if let Some(game) = &mut self.active_game {
            game.reset(self.settings.difficulty, &mut self.rng);
        }
        self.paused = false;
        self.pause_cursor = 0;
        self.new_high_score = false;
        self.committed_run_score = 0;
    }

    pub fn exit_to_menu(&mut self) {
        self.commit_score_if_needed();
        self.active_game = None;
        self.paused = false;
        self.menu_anim_ms = 240;
        self.screen = Screen::MainMenu;
    }

    fn commit_score_if_needed(&mut self) {
        if let Some(game) = &self.active_game {
            let score = game.score();
            if score > self.committed_run_score {
                let is_best = self.highscores.record(game.id(), score, game.difficulty());
                if is_best {
                    self.new_high_score = true;
                }
                self.committed_run_score = score;
                let _ = self.highscores.save_to(&self.paths);
            }
        }
    }

    fn persist_settings(&self) {
        let _ = self.settings.save_to(&self.paths);
    }

    pub fn tick(&mut self, dt: Duration) {
        self.idle_tick = self.idle_tick.wrapping_add(1);

        match &mut self.screen {
            Screen::Startup { elapsed_ms } => {
                let add = dt.as_millis().min(200) as u32;
                *elapsed_ms = elapsed_ms.saturating_add(add);
                if *elapsed_ms >= BOOT_DURATION_MS {
                    self.menu_anim_ms = 280;
                    self.screen = Screen::MainMenu;
                }
            }
            Screen::MainMenu => {
                let sub = dt.as_millis().min(200) as u32;
                self.menu_anim_ms = self.menu_anim_ms.saturating_sub(sub);
            }
            Screen::Settings | Screen::HighScores => {}
            Screen::InGame => {
                if !self.paused {
                    let mut ended_now = false;
                    if let Some(game) = &mut self.active_game {
                        let before = game.status();
                        game.tick(dt, &mut self.rng);
                        let after = game.status();
                        if before == GameStatus::Playing && after != GameStatus::Playing {
                            ended_now = true;
                        }
                    }
                    if ended_now {
                        self.commit_score_if_needed();
                    }
                }
            }
        }
    }

    pub fn handle_event(&mut self, event: ConsoleEvent) {
        if event == ConsoleEvent::ForceQuit {
            self.commit_score_if_needed();
            self.should_quit = true;
            return;
        }
        if matches!(event, ConsoleEvent::Resize(_, _)) {
            return;
        }

        match self.screen {
            Screen::Startup { .. } => {
                self.menu_anim_ms = 280;
                self.screen = Screen::MainMenu;
            }
            Screen::MainMenu => self.handle_menu_event(event),
            Screen::Settings => self.handle_settings_event(event),
            Screen::HighScores => self.handle_highscores_event(event),
            Screen::InGame => self.handle_ingame_event(event),
        }
    }

    fn handle_menu_event(&mut self, event: ConsoleEvent) {
        let count = GameId::ALL.len();
        match event {
            ConsoleEvent::Up => {
                self.selected_game = (self.selected_game + count - 1) % count;
                self.menu_anim_ms = 260;
            }
            ConsoleEvent::Down => {
                self.selected_game = (self.selected_game + 1) % count;
                self.menu_anim_ms = 260;
            }
            ConsoleEvent::Left => {
                self.settings.difficulty = self.settings.difficulty.prev();
                self.persist_settings();
            }
            ConsoleEvent::Right | ConsoleEvent::CycleDifficulty => {
                self.settings.difficulty = self.settings.difficulty.next();
                self.persist_settings();
            }
            ConsoleEvent::Confirm | ConsoleEvent::ActionSpace => {
                let id = self.current_game_id();
                self.launch_game(id);
            }
            ConsoleEvent::Number(n) => {
                let idx = (n.saturating_sub(1)) as usize;
                if idx < count {
                    self.selected_game = idx;
                    let id = GameId::ALL[idx];
                    self.launch_game(id);
                }
            }
            ConsoleEvent::CycleTheme => {
                self.settings.theme = self.settings.theme.next();
                self.persist_settings();
            }
            ConsoleEvent::OpenSettings | ConsoleEvent::ContinueEndless => {
                self.settings_cursor = 0;
                self.confirm_reset_scores = false;
                self.screen = Screen::Settings;
            }
            ConsoleEvent::OpenHighScores => {
                self.highscores_cursor = self.selected_game;
                self.screen = Screen::HighScores;
            }
            ConsoleEvent::QuitOrMenu | ConsoleEvent::BackOrPause => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_settings_event(&mut self, event: ConsoleEvent) {
        const ROWS: usize = 5;
        match event {
            ConsoleEvent::Up => {
                self.settings_cursor = (self.settings_cursor + ROWS - 1) % ROWS;
                self.confirm_reset_scores = false;
            }
            ConsoleEvent::Down => {
                self.settings_cursor = (self.settings_cursor + 1) % ROWS;
                self.confirm_reset_scores = false;
            }
            ConsoleEvent::Left => {
                self.adjust_setting(false);
            }
            ConsoleEvent::Right | ConsoleEvent::Confirm | ConsoleEvent::ActionSpace => {
                self.adjust_setting(true);
            }
            ConsoleEvent::CycleTheme => {
                self.settings.theme = self.settings.theme.next();
                self.persist_settings();
            }
            ConsoleEvent::BackOrPause | ConsoleEvent::QuitOrMenu | ConsoleEvent::OpenSettings => {
                self.confirm_reset_scores = false;
                self.screen = Screen::MainMenu;
            }
            _ => {}
        }
    }

    fn adjust_setting(&mut self, forward: bool) {
        match self.settings_cursor {
            0 => {
                self.settings.theme = if forward {
                    self.settings.theme.next()
                } else {
                    self.settings.theme.prev()
                };
                self.persist_settings();
            }
            1 => {
                self.settings.difficulty = if forward {
                    self.settings.difficulty.next()
                } else {
                    self.settings.difficulty.prev()
                };
                self.persist_settings();
            }
            2 => {
                self.settings.scanlines = !self.settings.scanlines;
                self.persist_settings();
            }
            3 => {
                self.settings.boot_animation = !self.settings.boot_animation;
                self.persist_settings();
            }
            4 => {
                if self.confirm_reset_scores {
                    self.highscores.clear();
                    let _ = self.highscores.save_to(&self.paths);
                    self.confirm_reset_scores = false;
                } else {
                    self.confirm_reset_scores = true;
                }
            }
            _ => {}
        }
    }

    fn handle_highscores_event(&mut self, event: ConsoleEvent) {
        let count = GameId::ALL.len();
        match event {
            ConsoleEvent::Up | ConsoleEvent::Left => {
                self.highscores_cursor = (self.highscores_cursor + count - 1) % count;
            }
            ConsoleEvent::Down | ConsoleEvent::Right => {
                self.highscores_cursor = (self.highscores_cursor + 1) % count;
            }
            ConsoleEvent::CycleTheme => {
                self.settings.theme = self.settings.theme.next();
                self.persist_settings();
            }
            ConsoleEvent::Confirm | ConsoleEvent::ActionSpace => {
                let id = GameId::ALL[self.highscores_cursor % count];
                self.launch_game(id);
            }
            ConsoleEvent::BackOrPause | ConsoleEvent::QuitOrMenu | ConsoleEvent::OpenHighScores => {
                self.screen = Screen::MainMenu;
            }
            _ => {}
        }
    }

    fn handle_ingame_event(&mut self, event: ConsoleEvent) {
        if event == ConsoleEvent::CycleTheme {
            self.settings.theme = self.settings.theme.next();
            self.persist_settings();
            return;
        }

        if self.paused {
            const PAUSE_ITEMS: usize = 4;
            match event {
                ConsoleEvent::PauseToggle | ConsoleEvent::BackOrPause => {
                    self.paused = false;
                }
                ConsoleEvent::Restart => {
                    self.restart_active_game();
                }
                ConsoleEvent::QuitOrMenu => {
                    self.exit_to_menu();
                }
                ConsoleEvent::Up => {
                    self.pause_cursor = (self.pause_cursor + PAUSE_ITEMS - 1) % PAUSE_ITEMS;
                }
                ConsoleEvent::Down => {
                    self.pause_cursor = (self.pause_cursor + 1) % PAUSE_ITEMS;
                }
                ConsoleEvent::Confirm | ConsoleEvent::ActionSpace => match self.pause_cursor {
                    0 => self.paused = false,
                    1 => self.restart_active_game(),
                    2 => {
                        self.settings.difficulty = self.settings.difficulty.next();
                        self.persist_settings();
                        self.restart_active_game();
                    }
                    _ => self.exit_to_menu(),
                },
                _ => {}
            }
            return;
        }

        let status = self
            .active_game
            .as_ref()
            .map(ArcadeGame::status)
            .unwrap_or(GameStatus::GameOver);

        if status != GameStatus::Playing {
            match event {
                ConsoleEvent::Restart | ConsoleEvent::Confirm | ConsoleEvent::ActionSpace => {
                    self.restart_active_game();
                }
                ConsoleEvent::ContinueEndless if status == GameStatus::Won => {
                    if let Some(game) = &mut self.active_game {
                        game.handle_input(crate::input::GameInput::ContinueEndless, &mut self.rng);
                    }
                }
                ConsoleEvent::QuitOrMenu | ConsoleEvent::BackOrPause => {
                    self.exit_to_menu();
                }
                _ => {}
            }
            return;
        }

        // Active unpaused gameplay
        match event {
            ConsoleEvent::PauseToggle | ConsoleEvent::BackOrPause => {
                self.paused = true;
                self.pause_cursor = 0;
            }
            ConsoleEvent::Restart => {
                self.restart_active_game();
            }
            ConsoleEvent::QuitOrMenu => {
                self.exit_to_menu();
            }
            other => {
                if let Some(game_input) = other.to_game_input() {
                    let mut ended_now = false;
                    if let Some(game) = &mut self.active_game {
                        let before = game.status();
                        game.handle_input(game_input, &mut self.rng);
                        let after = game.status();
                        if before == GameStatus::Playing && after != GameStatus::Playing {
                            ended_now = true;
                        }
                    }
                    if ended_now {
                        self.commit_score_if_needed();
                    }
                }
            }
        }
    }
}
