use std::cell::RefCell;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use nexustation::{
    config::{AppPaths, Difficulty, Settings, migrate_legacy_data},
    games::{
        ArcadeGame, GameId, GameStatus,
        blockfall::{ActivePiece, BlockKind, BlockfallGame, WELL_H, WELL_W},
        breakout::{ARENA_H, BreakoutGame, Brick},
        dino::{DINO_X, DinoGame, Obstacle, ObstacleKind},
        game_2048::{Game2048, SlideDir},
        pong::{COURT_H, COURT_W, PLAYER_X, PongGame, WIN_GOALS},
        snake::{Direction, GRID_W, GridPoint, SnakeGame},
    },
    highscores::{HighScoreStore, MAX_LEADERBOARD_ENTRIES},
    input::{ConsoleEvent, GameInput, decode_event},
    rng::Rng,
    state::{AppState, Screen},
    terminal::{LifecycleOps, TerminalGuard, write_enter_sequences, write_leave_sequences},
    theme::ThemeId,
    ui,
};

static TMP_SEQ: AtomicU64 = AtomicU64::new(1);

fn temp_data_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "nexustation-test-{label}-{}-{nanos}-{seq}",
        std::process::id()
    ))
}

// ============================================================================
// 1. 2048 MERGING, WIN & GAME-OVER TESTS
// ============================================================================

#[test]
fn test_2048_line_merging_rules() {
    let (out, merged, gain) = Game2048::slide_line([2, 2, 0, 0]);
    assert_eq!(out, [4, 0, 0, 0]);
    assert_eq!(merged, [true, false, false, false]);
    assert_eq!(gain, 4);

    // Must merge each tile at most once per move: [2, 2, 2, 2] -> [4, 4, 0, 0]
    let (out, merged, gain) = Game2048::slide_line([2, 2, 2, 2]);
    assert_eq!(out, [4, 4, 0, 0]);
    assert_eq!(merged, [true, true, false, false]);
    assert_eq!(gain, 8);

    // Must not double-merge newly formed 4 with preceding 4
    let (out, _, gain) = Game2048::slide_line([4, 2, 2, 0]);
    assert_eq!(out, [4, 4, 0, 0]);
    assert_eq!(gain, 4);

    // Gap compression before merge
    let (out, _, gain) = Game2048::slide_line([2, 0, 2, 4]);
    assert_eq!(out, [4, 4, 0, 0]);
    assert_eq!(gain, 4);
}

#[test]
fn test_2048_noop_win_and_gameover() {
    let mut rng = Rng::from_seed(42);

    // No-op slide must not spawn tiles or advance move count
    let mut g = Game2048::from_board(
        [[2, 4, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        Difficulty::Normal,
    );
    assert!(!g.slide(SlideDir::Left, &mut rng));
    assert_eq!(g.moves(), 0);
    assert_eq!(g.board()[0], [2, 4, 0, 0]);

    // Win at 2048 and continue endless mode
    let mut win_game = Game2048::from_board(
        [[1024, 1024, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        Difficulty::Normal,
    );
    assert!(win_game.slide(SlideDir::Left, &mut rng));
    assert_eq!(win_game.status(), GameStatus::Won);
    assert_eq!(win_game.score(), 2048);

    win_game.handle_input(GameInput::ContinueEndless, &mut rng);
    assert_eq!(win_game.status(), GameStatus::Playing);

    // Deadlocked full board triggers GameOver
    let dead = Game2048::from_board(
        [[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]],
        Difficulty::Normal,
    );
    assert_eq!(dead.status(), GameStatus::GameOver);
    assert!(!dead.has_valid_moves());
}

// ============================================================================
// 2. SNAKE COLLISIONS & GROWTH TESTS
// ============================================================================

#[test]
fn test_snake_wall_and_self_collisions_and_growth() {
    let mut rng = Rng::from_seed(7);

    // Wall collision on right boundary
    let mut wall_snake = SnakeGame::from_segments(
        &[
            GridPoint {
                x: GRID_W - 1,
                y: 5,
            },
            GridPoint {
                x: GRID_W - 2,
                y: 5,
            },
        ],
        Direction::Right,
        GridPoint { x: 2, y: 2 },
        Difficulty::Normal,
    );
    wall_snake.step_once(&mut rng);
    assert_eq!(wall_snake.status(), GameStatus::GameOver);

    // Self collision when looping into body
    let mut self_snake = SnakeGame::from_segments(
        &[
            GridPoint { x: 5, y: 5 },
            GridPoint { x: 6, y: 5 },
            GridPoint { x: 6, y: 6 },
            GridPoint { x: 5, y: 6 },
            GridPoint { x: 4, y: 6 },
        ],
        Direction::Down,
        GridPoint { x: 1, y: 1 },
        Difficulty::Normal,
    );
    self_snake.step_once(&mut rng);
    assert_eq!(self_snake.status(), GameStatus::GameOver);

    // Moving onto tail cell while NOT growing is legal (tail vacates simultaneously)
    let mut chase_tail = SnakeGame::from_segments(
        &[
            GridPoint { x: 5, y: 5 },
            GridPoint { x: 6, y: 5 },
            GridPoint { x: 6, y: 6 },
            GridPoint { x: 5, y: 6 },
        ],
        Direction::Down,
        GridPoint { x: 1, y: 1 },
        Difficulty::Normal,
    );
    chase_tail.step_once(&mut rng);
    assert_eq!(chase_tail.status(), GameStatus::Playing);

    // Eating food grows snake and increases score
    let mut eater = SnakeGame::from_segments(
        &[
            GridPoint { x: 5, y: 5 },
            GridPoint { x: 4, y: 5 },
            GridPoint { x: 3, y: 5 },
        ],
        Direction::Right,
        GridPoint { x: 6, y: 5 },
        Difficulty::Normal,
    );
    eater.step_once(&mut rng);
    assert_eq!(eater.status(), GameStatus::Playing);
    assert_eq!(eater.body().len(), 4);
    assert!(eater.score() > 0);
    assert!(!eater.body().contains(&eater.food()));
}

// ============================================================================
// 3. DINO RUN COLLISION, JUMP & CROUCH TESTS
// ============================================================================

#[test]
fn test_dino_obstacle_collision_jump_and_crouch() {
    let mut rng = Rng::from_seed(99);

    // 1. Direct collision with ground cactus -> GameOver
    let mut hit_game = DinoGame::new(Difficulty::Normal, &mut rng);
    hit_game.clear_obstacles();
    hit_game.inject_obstacle(Obstacle::new(DINO_X + 6.0, ObstacleKind::SmallCactus));
    for _ in 0..30 {
        hit_game.step_physics(1.0 / 60.0, &mut rng);
    }
    assert_eq!(hit_game.status(), GameStatus::GameOver);

    // 2. Jumping clears ground cactus cleanly
    let mut jump_game = DinoGame::new(Difficulty::Normal, &mut rng);
    jump_game.clear_obstacles();
    jump_game.inject_obstacle(Obstacle::new(DINO_X + 12.0, ObstacleKind::SmallCactus));
    jump_game.handle_input(GameInput::Action, &mut rng);
    let initial_speed = jump_game.speed();
    for _ in 0..50 {
        jump_game.step_physics(1.0 / 60.0, &mut rng);
    }
    assert_eq!(jump_game.status(), GameStatus::Playing);
    assert!(jump_game.speed() > initial_speed);
    assert!(jump_game.score() > 0);

    // 3. Mid-altitude pterodactyl hits standing dino, but crouching clears underneath!
    let mut stand_ptero = DinoGame::new(Difficulty::Normal, &mut rng);
    stand_ptero.clear_obstacles();
    stand_ptero.inject_obstacle(Obstacle::new(DINO_X + 6.0, ObstacleKind::PteroMid));
    for _ in 0..25 {
        stand_ptero.step_physics(1.0 / 60.0, &mut rng);
    }
    assert_eq!(stand_ptero.status(), GameStatus::GameOver);

    let mut duck_ptero = DinoGame::new(Difficulty::Normal, &mut rng);
    duck_ptero.clear_obstacles();
    duck_ptero.inject_obstacle(Obstacle::new(DINO_X + 6.0, ObstacleKind::PteroMid));
    for _ in 0..35 {
        duck_ptero.handle_input(GameInput::Down, &mut rng);
        duck_ptero.step_physics(1.0 / 60.0, &mut rng);
    }
    assert_eq!(duck_ptero.status(), GameStatus::Playing);
}

// ============================================================================
// 4. BLOCKFALL ROTATION, WALL-KICK & LINE CLEARING TESTS
// ============================================================================

#[test]
fn test_blockfall_rotation_and_line_clearing() {
    let mut rng = Rng::from_seed(123);
    let mut game = BlockfallGame::new(Difficulty::Normal, &mut rng);

    // Rotation CW & CCW around open center
    game.set_active_piece(ActivePiece {
        kind: BlockKind::Pulse,
        x: 4,
        y: 5,
        rotation: 0,
    });
    assert!(game.try_rotate(true));
    assert_eq!(game.active().rotation, 1);
    assert!(game.try_rotate(false));
    assert_eq!(game.active().rotation, 0);

    // Wall-kick against left edge for vertical Beam piece rotating horizontal
    game.set_active_piece(ActivePiece {
        kind: BlockKind::Beam,
        x: -2,
        y: 5,
        rotation: 1,
    });
    assert!(game.can_place(game.active()));
    assert!(game.try_rotate(true));
    assert_eq!(game.active().rotation, 2);
    assert!(game.active().x >= 0);

    // Fill bottom 2 rows completely and put 1 marker block on row WELL_H - 3
    for y in (WELL_H - 2)..WELL_H {
        for x in 0..WELL_W {
            game.set_cell(x, y, Some(1));
        }
    }
    game.set_cell(3, WELL_H - 3, Some(5));

    let cleared = game.clear_full_lines();
    assert_eq!(cleared, 2);
    assert_eq!(game.lines_cleared(), 2);
    assert_eq!(game.score(), 300);
    // Marker block should have shifted down by 2 rows to bottom row WELL_H - 1
    assert_eq!(game.well()[WELL_H - 1][3], Some(5));
    assert_eq!(game.well()[WELL_H - 3][3], None);
}

// ============================================================================
// 5. PONG SCORING & PADDLE BOUNCE TESTS
// ============================================================================

#[test]
fn test_pong_paddle_bounce_and_scoring() {
    let mut rng = Rng::from_seed(55);
    let mut pong = PongGame::new(Difficulty::Normal, &mut rng);

    // Paddle bounce reverses ball horizontal velocity and awards rally score
    pong.set_paddles(COURT_H * 0.5, COURT_H * 0.5);
    pong.set_ball(PLAYER_X + 2.0, COURT_H * 0.5, -40.0, 0.0);
    for _ in 0..5 {
        pong.step_physics(1.0 / 60.0, &mut rng);
    }
    assert!(
        pong.ball_vel().0 > 0.0,
        "ball should bounce rightward off player paddle"
    );
    assert!(pong.score() > 0);

    // Player scores goals up to WIN_GOALS -> Won
    for expected in 1..=WIN_GOALS {
        pong.set_ball(COURT_W + 0.8, COURT_H * 0.5, 40.0, 0.0);
        pong.step_physics(1.0 / 60.0, &mut rng);
        assert_eq!(pong.player_goals(), expected);
    }
    assert_eq!(pong.status(), GameStatus::Won);

    // CPU scores up to WIN_GOALS -> GameOver
    let mut cpu_win = PongGame::new(Difficulty::Normal, &mut rng);
    for expected in 1..=WIN_GOALS {
        cpu_win.set_ball(-0.8, COURT_H * 0.5, -40.0, 0.0);
        cpu_win.step_physics(1.0 / 60.0, &mut rng);
        assert_eq!(cpu_win.cpu_goals(), expected);
    }
    assert_eq!(cpu_win.status(), GameStatus::GameOver);
}

// ============================================================================
// 6. BREAKOUT BRICK COLLISIONS, LIVES & LEVEL ADVANCEMENT TESTS
// ============================================================================

#[test]
fn test_breakout_bricks_lives_and_levels() {
    let mut rng = Rng::from_seed(77);
    let mut game = BreakoutGame::new(Difficulty::Normal, &mut rng);

    // Replace brick list with one 2-HP armored brick and one 1-HP brick
    *game.bricks_mut() = vec![
        Brick {
            x: 35.0,
            y: 20.0,
            w: 10.0,
            h: 2.0,
            row: 0,
            hp: 2,
            max_hp: 2,
        },
        Brick {
            x: 10.0,
            y: 10.0,
            w: 10.0,
            h: 2.0,
            row: 2,
            hp: 1,
            max_hp: 1,
        },
    ];

    // Strike 2-HP brick from below: HP drops 2 -> 1, velocity reflects downward
    game.set_ball(40.0, 22.9, 0.0, -35.0);
    game.step_physics(1.0 / 60.0);
    assert_eq!(game.bricks()[0].hp, 1);
    assert!(game.ball_vel().1 > 0.0);
    assert!(game.score() > 0);

    // Strike again to destroy it
    game.set_ball(40.0, 22.9, 0.0, -35.0);
    game.step_physics(1.0 / 60.0);
    assert_eq!(game.bricks()[0].hp, 0);
    assert_eq!(game.remaining_bricks(), 1);

    // Destroy final brick -> advances to Level 2 with fresh bricks
    game.set_ball(15.0, 12.9, 0.0, -35.0);
    game.step_physics(1.0 / 60.0);
    assert_eq!(game.level(), 2);
    assert!(game.remaining_bricks() > 10);

    // Drain all lives -> GameOver
    while game.lives() > 0 && game.status() == GameStatus::Playing {
        game.set_ball(40.0, ARENA_H + 1.5, 0.0, 30.0);
        game.step_physics(1.0 / 60.0);
    }
    assert_eq!(game.lives(), 0);
    assert_eq!(game.status(), GameStatus::GameOver);
}

// ============================================================================
// 7. HIGH-SCORE & SETTINGS PERSISTENCE TESTS
// ============================================================================

#[test]
fn test_highscore_and_settings_persistence() {
    let dir = temp_data_dir("persist");
    let paths = AppPaths::with_root(&dir);

    let mut store = HighScoreStore::default();
    assert!(!store.record(GameId::Snake, 0, Difficulty::Normal));
    assert!(store.record(GameId::Snake, 120, Difficulty::Easy));
    assert!(store.record(GameId::Snake, 450, Difficulty::Hard));
    assert!(!store.record(GameId::Snake, 300, Difficulty::Normal));
    for extra in [50, 90, 210, 600] {
        store.record(GameId::Snake, extra, Difficulty::Normal);
    }
    assert_eq!(store.best_score(GameId::Snake), 600);
    assert_eq!(store.entries(GameId::Snake).len(), MAX_LEADERBOARD_ENTRIES);
    assert_eq!(
        store
            .entries(GameId::Snake)
            .iter()
            .map(|e| e.score)
            .collect::<Vec<_>>(),
        vec![600, 450, 300, 210, 120]
    );

    store.save_to(&paths).expect("save highscores");
    let reloaded = HighScoreStore::load_from(&paths);
    assert_eq!(reloaded, store);

    let settings = Settings {
        theme: ThemeId::RetroGreenCrt,
        difficulty: Difficulty::Hard,
        scanlines: false,
        boot_animation: false,
    };
    settings.save_to(&paths).expect("save settings");
    let loaded_settings = Settings::load_from(&paths);
    assert_eq!(loaded_settings, settings);

    // Corrupted JSON falls back cleanly to defaults
    std::fs::write(paths.highscores_file(), "{broken-json").expect("write corrupt");
    assert_eq!(HighScoreStore::load_from(&paths), HighScoreStore::default());

    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// 8. PAUSE & RESUME TESTS
// ============================================================================

#[test]
fn test_pause_and_resume_freezes_game_state() {
    let dir = temp_data_dir("pause");
    let paths = AppPaths::with_root(&dir);
    let mut app = AppState::new(paths, Rng::from_seed(11));
    app.screen = Screen::MainMenu;
    app.launch_game(GameId::DinoRun);

    // Advance slightly so distance > 0
    for _ in 0..20 {
        app.tick(Duration::from_millis(16));
    }
    let score_before_pause = app.active_game.as_ref().unwrap().score();
    assert!(score_before_pause > 0);

    // Pause game via 'P'
    app.handle_event(ConsoleEvent::PauseToggle);
    assert!(app.paused);

    // Tick for 1 second while paused: score must remain completely frozen
    for _ in 0..60 {
        app.tick(Duration::from_millis(16));
    }
    assert_eq!(
        app.active_game.as_ref().unwrap().score(),
        score_before_pause
    );

    // Resume via Esc
    app.handle_event(ConsoleEvent::BackOrPause);
    assert!(!app.paused);

    // Advance again: score increases
    for _ in 0..20 {
        app.tick(Duration::from_millis(16));
    }
    assert!(app.active_game.as_ref().unwrap().score() > score_before_pause);

    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// 9. INPUT HANDLING TESTS
// ============================================================================

#[test]
fn test_input_decoding_and_release_filtering() {
    let press_w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
    assert_eq!(decode_event(Event::Key(press_w)), Some(ConsoleEvent::Up));
    assert_eq!(ConsoleEvent::Up.to_game_input(), Some(GameInput::Up));

    // Key release events must be ignored (prevents double-input on Windows & Kitty enhanced mode)
    let mut release_w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
    release_w.kind = KeyEventKind::Release;
    assert_eq!(decode_event(Event::Key(release_w)), None);

    // Ctrl+C must decode to ForceQuit
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(
        decode_event(Event::Key(ctrl_c)),
        Some(ConsoleEvent::ForceQuit)
    );

    // Quick-launch digits and Blockfall CCW rotation
    let key_4 = KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE);
    assert_eq!(
        decode_event(Event::Key(key_4)),
        Some(ConsoleEvent::Number(4))
    );

    let key_z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
    assert_eq!(
        decode_event(Event::Key(key_z)).and_then(ConsoleEvent::to_game_input),
        Some(GameInput::RotateCcw)
    );
}

// ============================================================================
// 10. TERMINAL CLEANUP & RAII RESTORATION TESTS
// ============================================================================

#[derive(Default)]
struct MockTerminalState {
    raw_mode: bool,
    enter_calls: usize,
    leave_calls: usize,
}

struct MockOps(Rc<RefCell<MockTerminalState>>);

impl LifecycleOps for MockOps {
    fn enter(&mut self) -> io::Result<()> {
        let mut s = self.0.borrow_mut();
        s.raw_mode = true;
        s.enter_calls += 1;
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        let mut s = self.0.borrow_mut();
        s.raw_mode = false;
        s.leave_calls += 1;
        Ok(())
    }
}

#[test]
fn test_terminal_guard_cleanup_on_exit_error_and_panic() {
    // Normal exit + idempotent double restore
    let st = Rc::new(RefCell::new(MockTerminalState::default()));
    {
        let mut guard = TerminalGuard::acquire(MockOps(st.clone())).unwrap();
        assert!(st.borrow().raw_mode);
        assert!(guard.is_active());
        guard.restore().unwrap();
        guard.restore().unwrap(); // Must be idempotent
        assert!(!st.borrow().raw_mode);
    }
    assert_eq!(st.borrow().enter_calls, 1);
    assert_eq!(st.borrow().leave_calls, 1);

    // Recoverable early error return via Drop
    let st_err = Rc::new(RefCell::new(MockTerminalState::default()));
    let res: Result<(), &str> = {
        let _guard = TerminalGuard::acquire(MockOps(st_err.clone())).unwrap();
        assert!(st_err.borrow().raw_mode);
        Err("simulated recoverable error")
    };
    assert!(res.is_err());
    assert!(!st_err.borrow().raw_mode);
    assert_eq!(st_err.borrow().leave_calls, 1);

    // Panic unwind still restores raw mode
    let st_panic = Rc::new(RefCell::new(MockTerminalState::default()));
    let panic_res = catch_unwind(AssertUnwindSafe(|| {
        let _guard = TerminalGuard::acquire(MockOps(st_panic.clone())).unwrap();
        assert!(st_panic.borrow().raw_mode);
        panic!("simulated panic");
    }));
    assert!(panic_res.is_err());
    assert!(!st_panic.borrow().raw_mode);
    assert_eq!(st_panic.borrow().leave_calls, 1);

    // Verify real Crossterm escape sequences for enter & leave
    let mut ansi = Vec::new();
    write_enter_sequences(&mut ansi).unwrap();
    write_leave_sequences(&mut ansi).unwrap();
    let seq = String::from_utf8_lossy(&ansi);
    assert!(
        seq.contains("?1049h"),
        "expected EnterAlternateScreen sequence"
    );
    assert!(seq.contains("?25l"), "expected Hide cursor sequence");
    assert!(
        seq.contains("?1049l"),
        "expected LeaveAlternateScreen sequence"
    );
    assert!(seq.contains("?25h"), "expected Show cursor sequence");
}

// ============================================================================
// 11. FULL UI RENDERING & SMALL-TERMINAL RESIZE SAFETY TESTS
// ============================================================================

#[test]
fn test_ui_renders_all_screens_games_themes_and_tiny_sizes_without_panic() {
    let dir = temp_data_dir("ui");
    let paths = AppPaths::with_root(&dir);
    let mut app = AppState::new(paths, Rng::from_seed(2026));

    let backend = TestBackend::new(104, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    // Render Startup, Menu, Settings, HighScores across all 4 themes
    for theme in ThemeId::ALL {
        app.settings.theme = theme;
        for screen in [
            Screen::Startup { elapsed_ms: 900 },
            Screen::MainMenu,
            Screen::Settings,
            Screen::HighScores,
        ] {
            app.screen = screen;
            terminal.draw(|f| ui::render(f, &app)).unwrap();
        }
    }

    // Render all 6 games, plus Pause overlay
    for &gid in &GameId::ALL {
        app.launch_game(gid);
        app.tick(Duration::from_millis(50));
        terminal.draw(|f| ui::render(f, &app)).unwrap();
        app.paused = true;
        terminal.draw(|f| ui::render(f, &app)).unwrap();
        app.paused = false;
    }

    // Verify Startup BIOS renders NEXUSTATION branding AND keeps the animated pixel crab
    app.screen = Screen::Startup { elapsed_ms: 900 };
    app.settings.theme = ThemeId::NeonArcade;
    terminal.draw(|f| ui::render(f, &app)).unwrap();
    let buf = terminal.backend().buffer();
    let rendered: String = buf
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(
        rendered.contains("NEXUSTATION BIOS"),
        "startup BIOS must show NEXUSTATION branding"
    );
    assert!(
        rendered.contains("YOUR TERMINAL. YOUR ARCADE."),
        "startup BIOS must display official tagline"
    );
    assert!(
        rendered.contains("RUST-CRAB") && rendered.contains("RELEASE THE CRAB"),
        "startup BIOS must preserve Rust crab references"
    );
    assert!(
        rendered.contains('█') && (rendered.contains('▀') || rendered.contains('▄')),
        "startup BIOS must render pixel crab graphics"
    );

    // Verify small terminal resize never panics
    for (w, h) in [(60, 20), (30, 10), (12, 4), (1, 1)] {
        let mut tiny = Terminal::new(TestBackend::new(w, h)).unwrap();
        tiny.draw(|f| ui::render(f, &app)).unwrap();
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// 12. SAFE LEGACY CRABSTATION -> NEXUSTATION USER DATA MIGRATION TESTS
// ============================================================================

#[test]
fn test_legacy_migration_preserves_settings_scores_permissions_and_originals() {
    let legacy_dir = temp_data_dir("mig-legacy");
    let target_dir = temp_data_dir("mig-target");
    let legacy_paths = AppPaths::with_root(&legacy_dir);
    let target_paths = AppPaths::with_root(&target_dir);

    let orig_settings = Settings {
        theme: ThemeId::RetroGreenCrt,
        difficulty: Difficulty::Hard,
        scanlines: false,
        boot_animation: true,
    };
    orig_settings.save_to(&legacy_paths).unwrap();

    let mut orig_scores = HighScoreStore::default();
    orig_scores.record(GameId::Game2048, 4096, Difficulty::Normal);
    orig_scores.record(GameId::Snake, 340, Difficulty::Hard);
    orig_scores.save_to(&legacy_paths).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            legacy_paths.settings_file(),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        std::fs::set_permissions(
            legacy_paths.highscores_file(),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }

    let report = migrate_legacy_data(&legacy_dir, &target_paths);
    assert!(report.migrated_settings);
    assert!(report.migrated_highscores);
    assert_eq!(report.skipped_existing, 0);
    assert_eq!(report.skipped_corrupt, 0);

    // Migrated files match originals exactly
    assert_eq!(Settings::load_from(&target_paths), orig_settings);
    assert_eq!(HighScoreStore::load_from(&target_paths), orig_scores);

    // Original legacy files must NOT be deleted
    assert!(legacy_paths.settings_file().is_file());
    assert!(legacy_paths.highscores_file().is_file());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode_s = std::fs::metadata(target_paths.settings_file())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let mode_h = std::fs::metadata(target_paths.highscores_file())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode_s, 0o600);
        assert_eq!(mode_h, 0o600);
    }

    let _ = std::fs::remove_dir_all(&legacy_dir);
    let _ = std::fs::remove_dir_all(&target_dir);
}

#[test]
fn test_legacy_migration_never_overwrites_existing_nexustation_data() {
    let legacy_dir = temp_data_dir("mig-nooverwrite-legacy");
    let target_dir = temp_data_dir("mig-nooverwrite-target");
    let legacy_paths = AppPaths::with_root(&legacy_dir);
    let target_paths = AppPaths::with_root(&target_dir);

    Settings {
        theme: ThemeId::ClassicMonochrome,
        difficulty: Difficulty::Easy,
        scanlines: true,
        boot_animation: false,
    }
    .save_to(&legacy_paths)
    .unwrap();

    let mut legacy_scores = HighScoreStore::default();
    legacy_scores.record(GameId::Pong, 150, Difficulty::Easy);
    legacy_scores.save_to(&legacy_paths).unwrap();

    let newer_settings = Settings {
        theme: ThemeId::PurpleCyber,
        difficulty: Difficulty::Hard,
        scanlines: true,
        boot_animation: true,
    };
    newer_settings.save_to(&target_paths).unwrap();

    let mut newer_scores = HighScoreStore::default();
    newer_scores.record(GameId::Pong, 9999, Difficulty::Hard);
    newer_scores.save_to(&target_paths).unwrap();

    let report = migrate_legacy_data(&legacy_dir, &target_paths);
    assert!(!report.migrated_settings);
    assert!(!report.migrated_highscores);
    assert_eq!(report.skipped_existing, 2);

    assert_eq!(Settings::load_from(&target_paths), newer_settings);
    assert_eq!(HighScoreStore::load_from(&target_paths), newer_scores);

    let _ = std::fs::remove_dir_all(&legacy_dir);
    let _ = std::fs::remove_dir_all(&target_dir);
}

#[test]
fn test_legacy_migration_handles_corrupt_and_missing_files_gracefully() {
    let missing_dir = temp_data_dir("mig-missing");
    let target_dir = temp_data_dir("mig-corrupt-target");
    let target_paths = AppPaths::with_root(&target_dir);

    // Non-existent legacy directory is a safe no-op
    let noop = migrate_legacy_data(&missing_dir, &target_paths);
    assert!(!noop.any_migrated());
    assert!(!target_dir.exists());

    // Corrupt settings + valid highscores: skips corrupt, migrates valid
    let legacy_dir = temp_data_dir("mig-corrupt-legacy");
    let legacy_paths = AppPaths::with_root(&legacy_dir);
    legacy_paths.ensure_dir().unwrap();
    std::fs::write(legacy_paths.settings_file(), "{corrupt json!!!").unwrap();

    let mut valid_scores = HighScoreStore::default();
    valid_scores.record(GameId::Breakout, 1250, Difficulty::Normal);
    valid_scores.save_to(&legacy_paths).unwrap();

    let report = migrate_legacy_data(&legacy_dir, &target_paths);
    assert!(!report.migrated_settings);
    assert_eq!(report.skipped_corrupt, 1);
    assert!(report.migrated_highscores);
    assert!(!target_paths.settings_file().exists());
    assert_eq!(HighScoreStore::load_from(&target_paths), valid_scores);

    let _ = std::fs::remove_dir_all(&legacy_dir);
    let _ = std::fs::remove_dir_all(&target_dir);
}
