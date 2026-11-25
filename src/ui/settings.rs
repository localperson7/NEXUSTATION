use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::state::AppState;
use crate::theme::{Palette, ThemeId};

pub fn render_settings(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.border_active))
        .style(Style::default().bg(pal.panel_bg))
        .title(Line::from(vec![
            Span::styled(" ◆ NEXUSTATION CONFIG ", pal.title_style()),
            Span::styled("// DIP SWITCHES ", Style::default().fg(pal.fg_muted)),
        ]));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Min(2),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Adjust console visuals, default arcade difficulty, and local NVRAM records.",
            Style::default().fg(pal.fg_muted),
        )))
        .alignment(Alignment::Center),
        chunks[0],
    );

    let rows: [(&str, String, &str); 5] = [
        (
            "CRT VISUAL THEME",
            format!("◀  {}  ▶", app.settings.theme.name()),
            app.settings.theme.subtitle(),
        ),
        (
            "DEFAULT DIFFICULTY",
            format!("◀  {}  ▶", app.settings.difficulty.badge()),
            "Applies when launching or restarting a cartridge",
        ),
        (
            "CRT SCANLINE SHIMMER",
            if app.settings.scanlines {
                "[ ENABLED ]".to_string()
            } else {
                "[ DISABLED ]".to_string()
            },
            "Subtle alternating phosphor scanlines across background",
        ),
        (
            "STARTUP BIOS SEQUENCE",
            if app.settings.boot_animation {
                "[ ENABLED ]".to_string()
            } else {
                "[ FAST BOOT ]".to_string()
            },
            "Play animated NEXUSTATION BIOS (with pixel crab) on boot",
        ),
        (
            "CLEAR HIGH SCORES",
            if app.confirm_reset_scores {
                "⚠ PRESS [ENTER] AGAIN TO WIPE ALL SCORES ⚠".to_string()
            } else {
                "[ RESET NVRAM LEADERBOARDS ]".to_string()
            },
            "Erases local high-score tables while keeping preferences",
        ),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (idx, (label, value, hint)) in rows.iter().enumerate() {
        let selected = idx == app.settings_cursor;
        let bg = if selected {
            pal.highlight_bg
        } else {
            pal.panel_bg
        };
        let marker = if selected { "  ▶ " } else { "    " };
        let val_color = if idx == 4 && app.confirm_reset_scores {
            pal.danger
        } else if selected {
            pal.warning
        } else {
            pal.accent
        };

        lines.push(Line::from(vec![
            Span::styled(
                marker,
                Style::default()
                    .fg(pal.accent_secondary)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<24}", label),
                Style::default()
                    .fg(if selected { pal.fg_bright } else { pal.fg })
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                format!(" {:<42}", value),
                Style::default()
                    .fg(val_color)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!("      {}", hint),
            Style::default().fg(pal.fg_muted).bg(bg),
        )));
    }
    frame.render_widget(Paragraph::new(lines), chunks[1]);

    // Theme Swatch Strip
    let swatch_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(pal.bezel))
        .title(Span::styled(
            " INSTALLED CRT PALETTES ",
            Style::default().fg(pal.fg_muted),
        ));
    let swatch_inner = swatch_block.inner(chunks[2]);
    frame.render_widget(swatch_block, chunks[2]);

    let mut swatch_row1 = vec![Span::raw("  ")];
    let mut swatch_row2 = vec![Span::raw("  ")];
    for (idx, theme) in ThemeId::ALL.iter().enumerate() {
        let tp = theme.palette();
        let active = *theme == app.settings.theme;
        let row = if idx < 2 {
            &mut swatch_row1
        } else {
            &mut swatch_row2
        };
        row.push(Span::styled(
            if active { "◆ " } else { "  " },
            Style::default().fg(tp.accent),
        ));
        row.push(Span::styled(
            format!(" {:<19} ", theme.name()),
            Style::default()
                .fg(tp.fg_bright)
                .bg(tp.highlight_bg)
                .add_modifier(if active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ));
        row.push(Span::styled("██", Style::default().fg(tp.accent)));
        row.push(Span::styled("██", Style::default().fg(tp.accent_secondary)));
        row.push(Span::raw("    "));
    }

    let storage_line = Line::from(vec![
        Span::styled("  DATA DIR: ", Style::default().fg(pal.fg_muted)),
        Span::styled(
            app.paths.root().display().to_string(),
            Style::default().fg(pal.fg),
        ),
        Span::styled(
            "  (Offline · Rust Crab Core)",
            Style::default().fg(pal.fg_muted),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(swatch_row1),
            Line::from(swatch_row2),
            Line::from(""),
            storage_line,
        ]),
        swatch_inner,
    );
}
