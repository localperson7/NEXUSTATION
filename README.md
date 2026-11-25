# NEXUSTATION v0.1 🦀

**YOUR TERMINAL. YOUR ARCADE.**

Part of the **NEXUS Ecosystem** — a standalone retro arcade gaming console running entirely inside your terminal, written in pure Rust (`ratatui` + `crossterm`).

> **The Crabs Stay.** Originally born as CrabStation, NEXUSTATION preserves its animated pixel crab mascot and Rust-inspired BIOS tribute.

## Included Games (6 Cartridges)

| Cartridge | Game | Genre | Highlights |
|---|---|---|---|
| `NX-01` | **2048** | Logic Puzzle | 4×4 board, canonical merge rules, tile highlights, 2048 victory + endless mode |
| `NX-02` | **Snake** | Arcade Action | Fixed-timestep movement, 2-turn input buffer, bonus cores, 3 difficulty speeds |
| `NX-03` | **Dino Run** | Endless Runner | Sub-pixel half-block sprite animation, jump & crouch physics, cacti & sky-raptors |
| `NX-04` | **Blockfall** | Matrix Puzzle | 7 polyomino shapes, 7-bag generator, CW/CCW wall-kicks, ghost piece, next preview |
| `NX-05` | **Pong** | Retro Sports | Player vs CPU to 7 goals, paddle spin reflection, giant pixel score digits |
| `NX-06` | **Breakout** | Brick Smasher | Multi-stage brick walls, armored 2-HP bricks, 3 lives, level progression |

## Visual Themes

Press `T` anywhere (or open Settings with `O` / `Tab`) to switch between 4 CRT themes:
- **Neon Arcade** (default) — NEXUS graphite-black with electric violet, blue, and cyan accents
- **Retro Green CRT** — Classic P1 emerald phosphor monitor
- **Classic Monochrome** — Crisp obsidian & ivory high-contrast CRT
- **Purple Cyber** — Synthwave dusk plum & electric orchid

## Quick Start & User-Scoped Installation

```bash
cargo build --locked --release
./target/release/nexustation
```

This build has been checked on Linux x86-64. Windows and macOS builds have not been verified. The prebuilt Linux binary requires glibc 2.39 or newer.

The project is licensed under MIT; see `LICENSE`. Dependency license texts for the Linux build are collected in `THIRD_PARTY_NOTICES.txt`.

CLI options:
- `nexustation` — Launch with animated BIOS boot sequence (pixel crab mascot)
- `nexustation --no-boot` — Skip BIOS sequence straight to Main Deck
- `nexustation --version` / `-V` — Print version
- `nexustation --help` / `-h` — Show usage

## Local Storage & Safe Migration from CrabStation

NEXUSTATION stores **only** local settings (`settings.json`) and local high scores (`highscores.json`). No accounts, no cloud sync, no telemetry, no network access.

| Platform | Legacy CrabStation Path | New NEXUSTATION Path |
|---|---|---|
| Linux / Unix | `$XDG_DATA_HOME/crabstation` or `~/.local/share/crabstation` | `$XDG_DATA_HOME/nexustation` or `~/.local/share/nexustation` |
| macOS | `~/Library/Application Support/CrabStation` | `~/Library/Application Support/NEXUSTATION` |
| Windows | `%LOCALAPPDATA%\CrabStation` | `%LOCALAPPDATA%\NEXUSTATION` |
| Env Override | `CRABSTATION_DATA_DIR` | `NEXUSTATION_DATA_DIR` |

On startup, NEXUSTATION checks for legacy CrabStation files and **non-destructively migrates** any valid `settings.json` / `highscores.json` that do not yet exist in the NEXUSTATION directory:
- Never deletes or modifies legacy CrabStation data.
- Never overwrites newer existing NEXUSTATION data.
- Validates JSON before writing and preserves file permissions via atomic temp-file + rename writes.

## Verification

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```
