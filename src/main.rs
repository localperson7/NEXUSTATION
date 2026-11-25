#![forbid(unsafe_code)]

use std::env;
use std::io;
use std::time::{Duration, Instant};

use crossterm::event;

use nexustation::{
    config::{AppPaths, migrate_legacy_data},
    input::decode_event,
    rng::Rng,
    state::{AppState, Screen},
    terminal::ConsoleSession,
    ui,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("nexustation {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "NEXUSTATION v{} — YOUR TERMINAL. YOUR ARCADE.",
            env!("CARGO_PKG_VERSION")
        );
        println!("NEXUS Ecosystem Retro Terminal Arcade Console (Powered by Rust 🦀)");
        println!();
        println!("USAGE:");
        println!("    nexustation [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    --no-boot      Skip startup BIOS animation");
        println!("    -V, --version  Print version");
        println!("    -h, --help     Print help");
        return;
    }

    let skip_boot = args.iter().any(|a| a == "--no-boot");

    if let Err(err) = run(skip_boot) {
        eprintln!("NEXUSTATION error: {err}");
        std::process::exit(1);
    }
}

fn run(skip_boot: bool) -> io::Result<()> {
    let paths = AppPaths::discover();
    // Safe non-destructive migration from legacy CrabStation user data (only when new files do not exist)
    if let Some(legacy_dir) = AppPaths::discover_legacy() {
        let _ = migrate_legacy_data(&legacy_dir, &paths);
    }

    let mut app = AppState::new(paths, Rng::from_entropy());
    if skip_boot {
        app.screen = Screen::MainMenu;
    }

    let mut session = ConsoleSession::enter()?;
    let mut last_tick = Instant::now();

    let loop_result: io::Result<()> = loop {
        if let Err(err) = session.terminal_mut().draw(|frame| ui::render(frame, &app)) {
            break Err(err);
        }

        if app.should_quit {
            break Ok(());
        }

        let timeout = app.poll_timeout();
        match event::poll(timeout) {
            Ok(true) => {
                // Drain all immediately available queued events for crisp responsiveness
                while event::poll(Duration::ZERO).unwrap_or(false) {
                    match event::read() {
                        Ok(raw) => {
                            if let Some(ev) = decode_event(raw) {
                                app.handle_event(ev);
                                if app.should_quit {
                                    break;
                                }
                            }
                        }
                        Err(err) => {
                            let _ = session.shutdown();
                            return Err(err);
                        }
                    }
                }
            }
            Ok(false) => {}
            Err(err) => break Err(err),
        }

        let now = Instant::now();
        let dt = now.saturating_duration_since(last_tick);
        last_tick = now;
        app.tick(dt);
    };

    let shutdown_result = session.shutdown();
    loop_result.and(shutdown_result)
}
