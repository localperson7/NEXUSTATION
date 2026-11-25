use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::games::GameId;
use crate::state::AppState;
use crate::theme::Palette;
use crate::ui::canvas::HalfBlockCanvas;

pub fn render_main_menu(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(area);

    render_cartridge_bay(frame, columns[0], app, pal);
    render_attract_monitor(frame, columns[1], app, pal);
}

fn render_cartridge_bay(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.border_active))
        .style(Style::default().bg(pal.panel_bg))
        .title(Line::from(vec![
            Span::styled(" ◆ NEXUS DECK ", pal.title_style()),
            Span::styled(
                format!("({} SLOTS · RUST CRAB) ", GameId::ALL.len()),
                Style::default().fg(pal.fg_muted),
            ),
        ]));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(6)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (idx, &game_id) in GameId::ALL.iter().enumerate() {
        let selected = idx == app.selected_game;
        let best = app.highscores.best_score(game_id);

        let cursor = if selected {
            if app.menu_anim_ms > 130 {
                " ⟫▶ "
            } else if app.menu_anim_ms > 0 || app.idle_tick.is_multiple_of(2) {
                "  ▶ "
            } else {
                "  ▹ "
            }
        } else {
            "    "
        };

        let row_bg = if selected {
            pal.highlight_bg
        } else {
            pal.panel_bg
        };
        let title_fg = if selected { pal.fg_bright } else { pal.fg };
        let code_fg = if selected { pal.accent } else { pal.fg_muted };
        let score_fg = if best > 0 {
            if selected { pal.warning } else { pal.accent }
        } else {
            pal.fg_muted
        };

        lines.push(Line::from(vec![
            Span::styled(
                cursor,
                Style::default()
                    .fg(pal.accent_secondary)
                    .bg(row_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}] ", idx + 1),
                Style::default().fg(code_fg).bg(row_bg),
            ),
            Span::styled(
                format!("{:<11}", game_id.title()),
                Style::default()
                    .fg(title_fg)
                    .bg(row_bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                format!(" HI {:06} ", best),
                Style::default().fg(score_fg).bg(row_bg),
            ),
        ]));

        let sub_fg = if selected { pal.accent } else { pal.fg_muted };
        lines.push(Line::from(vec![Span::styled(
            format!(
                "        {} // {:<18}",
                game_id.cartridge_code(),
                game_id.genre()
            ),
            Style::default().fg(sub_fg).bg(row_bg),
        )]));
    }

    frame.render_widget(Paragraph::new(lines), sections[0]);

    // Quick deck switches panel
    let deck_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(pal.bezel))
        .title(Span::styled(
            " YOUR TERMINAL. YOUR ARCADE. ",
            Style::default().fg(pal.fg_muted),
        ));
    let deck_inner = deck_block.inner(sections[1]);
    frame.render_widget(deck_block, sections[1]);

    let deck_lines = vec![
        Line::from(vec![
            Span::styled(" DIFFICULTY ", Style::default().fg(pal.fg_muted)),
            Span::styled("◀ ", Style::default().fg(pal.accent)),
            Span::styled(
                format!("{:<16}", app.settings.difficulty.badge()),
                Style::default()
                    .fg(pal.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("▶", Style::default().fg(pal.accent)),
        ]),
        Line::from(vec![
            Span::styled(" CRT THEME  ", Style::default().fg(pal.fg_muted)),
            Span::styled(
                format!("[T] {}", app.settings.theme.name()),
                Style::default().fg(pal.accent_secondary),
            ),
        ]),
        Line::from(vec![
            Span::styled(" ARCADE PTS ", Style::default().fg(pal.fg_muted)),
            Span::styled(
                format!("{:08}", app.highscores.total_arcade_points()),
                Style::default()
                    .fg(pal.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(deck_lines), deck_inner);
}

fn render_attract_monitor(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let game_id = app.current_game_id();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.bezel))
        .style(Style::default().bg(pal.panel_bg))
        .title(Line::from(vec![
            Span::styled(
                format!(" ◆ {} ", game_id.title()),
                Style::default()
                    .fg(pal.accent_secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("// {} ", game_id.cartridge_code()),
                Style::default().fg(pal.fg_muted),
            ),
        ]));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(inner);

    // 1. Pixel-art attract preview screen
    let preview_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(pal.bezel))
        .title(Span::styled(
            " ATTRACT PREVIEW ",
            Style::default().fg(pal.fg_muted),
        ));
    let preview_inner = preview_box.inner(chunks[0]);
    frame.render_widget(preview_box, chunks[0]);
    render_game_preview_art(frame, preview_inner, game_id, app, pal);

    // 2. Game description & controls
    let info_lines = vec![
        Line::from(vec![
            Span::styled(" GENRE: ", Style::default().fg(pal.fg_muted)),
            Span::styled(
                game_id.genre(),
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!(" {}", game_id.tagline()),
            Style::default().fg(pal.fg),
        )),
        Line::from(Span::styled(
            format!(" {}", game_id.controls_summary()),
            Style::default().fg(pal.fg_muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(info_lines).wrap(Wrap { trim: false }),
        chunks[1],
    );

    // 3. Top-3 leaderboard for highlighted cartridge
    let lb_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(pal.bezel))
        .title(Span::styled(
            " LOCAL HIGH SCORES ",
            Style::default()
                .fg(pal.warning)
                .add_modifier(Modifier::BOLD),
        ));
    let lb_inner = lb_block.inner(chunks[2]);
    frame.render_widget(lb_block, chunks[2]);

    let entries = app.highscores.entries(game_id);
    let mut lb_lines = Vec::new();
    if entries.is_empty() {
        lb_lines.push(Line::from(Span::styled(
            "  No records yet — set the first score!",
            Style::default().fg(pal.fg_muted),
        )));
    } else {
        for (rank, entry) in entries.iter().take(4).enumerate() {
            let medal_color = match rank {
                0 => pal.warning,
                1 => pal.fg_bright,
                _ => pal.accent,
            };
            lb_lines.push(Line::from(vec![
                Span::styled(
                    format!("  #{:<2} ", rank + 1),
                    Style::default()
                        .fg(medal_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:07} PTS   ", entry.score),
                    Style::default().fg(pal.fg_bright),
                ),
                Span::styled(
                    format!("[{}]", entry.difficulty.name().to_uppercase()),
                    Style::default().fg(pal.fg_muted),
                ),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lb_lines), lb_inner);

    // 4. Blinking INSERT COIN / PRESS ENTER banner
    let blink = app.idle_tick.is_multiple_of(2);
    let launch_style = if blink {
        Style::default()
            .fg(pal.bg)
            .bg(pal.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " ▶ [ENTER / SPACE] LAUNCH CARTRIDGE ◀ ",
            launch_style,
        )))
        .alignment(Alignment::Center),
        chunks[3],
    );
}

fn render_game_preview_art(
    frame: &mut Frame,
    area: Rect,
    game_id: GameId,
    app: &AppState,
    pal: &Palette,
) {
    if area.width < 12 || area.height < 4 {
        return;
    }
    let mut canvas = HalfBlockCanvas::new(area.width, area.height);
    let cx = i32::from(area.width) / 2;
    let phase = (app.idle_tick % 4) as i32;

    match game_id {
        GameId::Game2048 => {
            let tiles = [2u32, 16, 64, 2048, 4, 32, 128, 512];
            let start_x = cx - 18;
            for r in 0..2 {
                for c in 0..4 {
                    let v = tiles[r * 4 + c];
                    let (bg_c, _) = pal.tile_2048_colors(v);
                    canvas.fill_rect(start_x + (c as i32) * 9, 1 + (r as i32) * 6, 8, 5, bg_c);
                }
            }
            // Draw "2048" mini pixel banner on the special tile
            canvas.fill_rect(
                start_x + 27,
                1,
                8,
                5,
                if phase % 2 == 0 {
                    pal.warning
                } else {
                    pal.accent_secondary
                },
            );
        }
        GameId::Snake => {
            let y = 6;
            for seg in 0..8 {
                let sx = cx - 14 + seg * 3 + (phase % 2);
                let col = if seg == 7 { pal.fg_bright } else { pal.success };
                canvas.fill_rect(sx, y, 2, 2, col);
            }
            // Pulsing energy core
            let food_col = if phase % 2 == 0 {
                pal.danger
            } else {
                pal.warning
            };
            canvas.fill_rect(cx + 14, y, 2, 2, food_col);
        }
        GameId::DinoRun => {
            let ground_y = canvas.height_px() as i32 - 2;
            canvas.fill_rect(2, ground_y, i32::from(area.width) - 4, 1, pal.fg_muted);
            let dino_sprite = [
                "   #### ", "   #.## ", "   #### ", "# ##### ", "######  ", " #  #   ",
            ];
            let jump_off = if phase % 2 == 0 { 2 } else { 0 };
            canvas.draw_sprite(cx - 14, ground_y - 6 - jump_off, &dino_sprite, pal.accent);
            let cactus = [" # ", "###", "###", " # ", " # "];
            canvas.draw_sprite(
                cx + 6 - phase * 2,
                ground_y - 5,
                &cactus,
                pal.accent_secondary,
            );
        }
        GameId::Blockfall => {
            let base_y = canvas.height_px() as i32 - 1;
            // Stacked blocks at bottom
            for col in 0..7 {
                canvas.fill_rect(
                    cx - 14 + col * 4,
                    base_y - 2,
                    3,
                    2,
                    pal.piece_color(col as usize),
                );
            }
            // Falling T-piece
            let ty = 1 + (phase % 3);
            let tc = pal.piece_color(2);
            canvas.fill_rect(cx - 2, ty, 3, 2, tc);
            canvas.fill_rect(cx - 6, ty + 2, 11, 2, tc);
        }
        GameId::Pong => {
            let h = canvas.height_px() as i32;
            // Center dashed net
            for py in (1..h - 1).step_by(2) {
                canvas.set(cx, py, pal.fg_muted);
            }
            // Left & right paddles
            canvas.fill_rect(cx - 18, 3 + (phase % 2), 2, 5, pal.accent);
            canvas.fill_rect(cx + 16, 5 - (phase % 2), 2, 5, pal.accent_secondary);
            // Moving ball
            let bx = cx - 8 + phase * 5;
            let by = 4 + (phase % 3) * 2;
            canvas.fill_rect(bx, by, 2, 2, pal.fg_bright);
        }
        GameId::Breakout => {
            let h = canvas.height_px() as i32;
            // Mini brick wall
            for r in 0..3 {
                for c in 0..7 {
                    if r == 2 && c == 3 {
                        continue;
                    }
                    canvas.fill_rect(
                        cx - 17 + c * 5,
                        1 + r * 2,
                        4,
                        1,
                        pal.piece_color((r * 2 + c) as usize),
                    );
                }
            }
            // Ball & paddle
            canvas.fill_rect(cx - 4 + phase * 2, h - 5, 2, 1, pal.warning);
            canvas.fill_rect(cx - 6 + phase, h - 2, 10, 1, pal.accent);
        }
    }

    canvas.render(
        area,
        frame.buffer_mut(),
        pal.bg,
        pal.bg_scanline,
        app.settings.scanlines,
    );
}
