use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::games::{ActiveCartridge, ArcadeGame, GameId, GameStatus};
use crate::state::AppState;
use crate::theme::Palette;

pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

pub fn render_hud_sidebar(
    frame: &mut Frame,
    area: Rect,
    game: &ActiveCartridge,
    app: &AppState,
    pal: &Palette,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.bezel))
        .style(Style::default().bg(pal.panel_bg))
        .title(Span::styled(" SCOREBOARD ", pal.title_style()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let live_score = game.score();
    let stored_best = app.highscores.best_score(game.id());
    let live_best = stored_best.max(live_score);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(" SCORE", Style::default().fg(pal.fg_muted))),
        Line::from(Span::styled(
            format!(" {:07}", live_score),
            Style::default()
                .fg(pal.fg_bright)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " HIGH SCORE",
            Style::default().fg(pal.fg_muted),
        )),
        Line::from(Span::styled(
            format!(" {:07}", live_best),
            Style::default()
                .fg(pal.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(" MODE: {}", game.difficulty().name().to_uppercase()),
            Style::default().fg(pal.accent),
        )),
        Line::from(""),
    ];

    for (label, val) in game.stats() {
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<8} ", label), Style::default().fg(pal.fg_muted)),
            Span::styled(
                val,
                Style::default()
                    .fg(pal.accent_secondary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // Special Next-Piece mini box for Blockfall
    if let ActiveCartridge::Blockfall(bf) = game {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " NEXT PIECE",
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        )));
        let next = bf.next_piece();
        let cells = next.cells(0);
        let piece_col = pal.piece_color(usize::from(next.color_index()));
        for py in 0..3 {
            let mut spans = vec![Span::raw("   ")];
            for px in 0..4 {
                let filled = cells.contains(&(px, py));
                if filled {
                    spans.push(Span::styled("██", Style::default().fg(piece_col)));
                } else {
                    spans.push(Span::raw("  "));
                }
            }
            lines.push(Line::from(spans));
        }
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(5)])
        .split(inner);

    frame.render_widget(Paragraph::new(lines), sections[0]);

    let keys = vec![
        Line::from(Span::styled(
            " ──────────────",
            Style::default().fg(pal.bezel),
        )),
        Line::from(Span::styled(
            " [P/ESC] Pause",
            Style::default().fg(pal.fg_muted),
        )),
        Line::from(Span::styled(
            " [R]     Restart",
            Style::default().fg(pal.fg_muted),
        )),
        Line::from(Span::styled(
            " [Q]     Menu",
            Style::default().fg(pal.fg_muted),
        )),
    ];
    frame.render_widget(Paragraph::new(keys), sections[1]);
}

pub fn render_pause_overlay(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let modal = centered_rect(44, 14, area);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.warning))
        .style(Style::default().bg(pal.panel_bg))
        .title(Line::from(Span::styled(
            " ⏸  GAME PAUSED ",
            Style::default()
                .fg(pal.warning)
                .add_modifier(Modifier::BOLD),
        )));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let diff_label = format!("DIFFICULTY: {} (RESTART)", app.settings.difficulty.name());
    let options = [
        "RESUME CARTRIDGE       [P / ESC]",
        "RESTART RUN            [R]",
        diff_label.as_str(),
        "EXIT TO MAIN MENU      [Q]",
    ];

    let mut lines = vec![Line::from("")];
    for (idx, label) in options.iter().enumerate() {
        let sel = idx == app.pause_cursor;
        let bg = if sel { pal.highlight_bg } else { pal.panel_bg };
        let fg = if sel { pal.fg_bright } else { pal.fg };
        lines.push(Line::from(vec![
            Span::styled(
                if sel { "  ▶ " } else { "    " },
                Style::default()
                    .fg(pal.accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<34}", label),
                Style::default().fg(fg).bg(bg).add_modifier(if sel {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
        ]));
        lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn render_gameover_overlay(
    frame: &mut Frame,
    area: Rect,
    game: &ActiveCartridge,
    app: &AppState,
    pal: &Palette,
) {
    let won = game.status() == GameStatus::Won;
    let modal = centered_rect(48, 14, area);
    frame.render_widget(Clear, modal);

    let border_col = if won { pal.success } else { pal.danger };
    let header_title = if won {
        " ★ VICTORY! STAGE CLEARED ★ "
    } else {
        " ✖ GAME OVER // INSERT COIN "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_col))
        .style(Style::default().bg(pal.panel_bg))
        .title(Line::from(Span::styled(
            header_title,
            Style::default().fg(border_col).add_modifier(Modifier::BOLD),
        )));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let best = app.highscores.best_score(game.id()).max(game.score());
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("FINAL SCORE: ", Style::default().fg(pal.fg_muted)),
            Span::styled(
                format!("{:07}", game.score()),
                Style::default()
                    .fg(pal.fg_bright)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("HIGH SCORE : ", Style::default().fg(pal.fg_muted)),
            Span::styled(
                format!("{:07}", best),
                Style::default()
                    .fg(pal.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    if app.new_high_score {
        lines.push(Line::from(Span::styled(
            "★ NEW PERSONAL RECORD SAVED! ★",
            Style::default()
                .fg(pal.warning)
                .add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "TRY AGAIN TO BEAT THE RECORD!",
            Style::default().fg(pal.fg_muted),
        )));
    }
    lines.push(Line::from(""));

    if won && game.id() == GameId::Game2048 {
        lines.push(Line::from(Span::styled(
            "[C] CONTINUE ENDLESS   [R] RESTART",
            Style::default()
                .fg(pal.success)
                .add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "[R / ENTER] PLAY AGAIN",
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::from(Span::styled(
        "[Q / ESC]   RETURN TO MENU",
        Style::default().fg(pal.fg),
    )));

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}
