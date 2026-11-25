use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::games::GameId;
use crate::highscores::MAX_LEADERBOARD_ENTRIES;
use crate::state::AppState;
use crate::theme::Palette;

pub fn render_highscores(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.border_active))
        .style(Style::default().bg(pal.panel_bg))
        .title(Line::from(vec![
            Span::styled(" ◆ NEXUSTATION HALL OF FAME ", pal.title_style()),
            Span::styled("// LOCAL LEADERBOARDS ", Style::default().fg(pal.fg_muted)),
        ]));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(inner);

    // Left: game list with personal bests
    let mut left_lines = vec![Line::from("")];
    for (idx, &gid) in GameId::ALL.iter().enumerate() {
        let selected = idx == app.highscores_cursor % GameId::ALL.len();
        let bg = if selected {
            pal.highlight_bg
        } else {
            pal.panel_bg
        };
        let best = app.highscores.best_score(gid);
        left_lines.push(Line::from(vec![
            Span::styled(
                if selected { " ▶ " } else { "   " },
                Style::default()
                    .fg(pal.accent_secondary)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<12}", gid.title()),
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
                format!(" {:07} ", best),
                Style::default()
                    .fg(if best > 0 { pal.warning } else { pal.fg_muted })
                    .bg(bg),
            ),
        ]));
        left_lines.push(Line::from(""));
    }

    let left_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(pal.bezel))
        .title(Span::styled(
            " CARTRIDGES ",
            Style::default().fg(pal.fg_muted),
        ));
    let left_inner = left_block.inner(cols[0]);
    frame.render_widget(left_block, cols[0]);
    frame.render_widget(Paragraph::new(left_lines), left_inner);

    // Right: top-5 leaderboard for highlighted game
    let active_gid = GameId::ALL[app.highscores_cursor % GameId::ALL.len()];
    let entries = app.highscores.entries(active_gid);

    let mut right_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {}  ", active_gid.title()),
                Style::default()
                    .fg(pal.bg)
                    .bg(pal.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  {} // TOP {} RUNS",
                    active_gid.cartridge_code(),
                    MAX_LEADERBOARD_ENTRIES
                ),
                Style::default().fg(pal.fg_muted),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "   RANK    SCORE        DIFFICULTY",
            Style::default()
                .fg(pal.fg_muted)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   ────    ─────────    ──────────────",
            Style::default().fg(pal.bezel),
        )),
    ];

    for slot in 0..MAX_LEADERBOARD_ENTRIES {
        if let Some(entry) = entries.get(slot) {
            let rank_color = match slot {
                0 => pal.warning,
                1 => pal.fg_bright,
                2 => pal.accent_secondary,
                _ => pal.accent,
            };
            right_lines.push(Line::from(vec![
                Span::styled(
                    format!("   #{:<6} ", slot + 1),
                    Style::default().fg(rank_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:08}     ", entry.score),
                    Style::default()
                        .fg(pal.fg_bright)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(entry.difficulty.badge(), Style::default().fg(pal.fg)),
            ]));
        } else {
            right_lines.push(Line::from(Span::styled(
                format!("   #{:<6} --------     ---", slot + 1),
                Style::default().fg(pal.fg_muted),
            )));
        }
        right_lines.push(Line::from(""));
    }

    right_lines.push(Line::from(""));
    right_lines.push(Line::from(vec![
        Span::styled(
            "   TOTAL CONSOLE SCORE: ",
            Style::default().fg(pal.fg_muted),
        ),
        Span::styled(
            format!("{:09} PTS", app.highscores.total_arcade_points()),
            Style::default()
                .fg(pal.success)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    right_lines.push(Line::from(Span::styled(
        "   YOUR TERMINAL. YOUR ARCADE. // THE CRABS STAY",
        Style::default().fg(pal.fg_muted),
    )));

    frame.render_widget(
        Paragraph::new(right_lines).alignment(Alignment::Left),
        cols[1],
    );
}
