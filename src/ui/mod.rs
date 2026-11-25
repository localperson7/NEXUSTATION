pub mod canvas;
pub mod games;
pub mod highscores;
pub mod menu;
pub mod overlays;
pub mod settings;
pub mod startup;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::games::ArcadeGame;
use crate::state::{AppState, Screen};
use crate::theme::Palette;

pub const MIN_WIDTH: u16 = 78;
pub const MIN_HEIGHT: u16 = 27;

pub fn render(frame: &mut Frame, app: &AppState) {
    let pal = app.settings.theme.palette();
    let full = frame.area();

    // Paint CRT backdrop (with optional subtle scanline alternation)
    paint_backdrop(frame, full, &pal, app.settings.scanlines);

    if full.width < MIN_WIDTH || full.height < MIN_HEIGHT {
        render_small_terminal_warning(frame, full, &pal);
        return;
    }

    if let Screen::Startup { elapsed_ms } = app.screen {
        startup::render_startup(frame, full, app, elapsed_ms, &pal);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(20),
            Constraint::Length(1),
        ])
        .split(full);

    render_header(frame, layout[0], app, &pal);

    match app.screen {
        Screen::Startup { .. } => {}
        Screen::MainMenu => menu::render_main_menu(frame, layout[1], app, &pal),
        Screen::Settings => settings::render_settings(frame, layout[1], app, &pal),
        Screen::HighScores => highscores::render_highscores(frame, layout[1], app, &pal),
        Screen::InGame => games::render_in_game(frame, layout[1], app, &pal),
    }

    render_footer(frame, layout[2], app, &pal);
}

fn paint_backdrop(frame: &mut Frame, area: Rect, pal: &Palette, scanlines: bool) {
    let buf = frame.buffer_mut();
    for y in area.y..area.y.saturating_add(area.height) {
        let bg = pal.row_bg(y, scanlines);
        let style = Style::default().bg(bg).fg(pal.fg);
        for x in area.x..area.x.saturating_add(area.width) {
            let cell = &mut buf[(x, y)];
            cell.set_symbol(" ").set_style(style);
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.bezel))
        .style(Style::default().bg(pal.panel_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let screen_tag = match app.screen {
        Screen::Startup { .. } => "BIOS",
        Screen::MainMenu => "MAIN DECK",
        Screen::Settings => "SETTINGS",
        Screen::HighScores => "HALL OF FAME",
        Screen::InGame => app
            .active_game
            .as_ref()
            .map(|g| g.id().title())
            .unwrap_or("GAME"),
    };

    let left = Line::from(vec![
        Span::styled(
            " ◆ NEXUSTATION ",
            Style::default()
                .fg(pal.bg)
                .bg(pal.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " v0.1 ",
            Style::default()
                .fg(pal.accent_secondary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(pal.bezel)),
        Span::styled(
            screen_tag,
            Style::default()
                .fg(pal.fg_bright)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let right = Line::from(vec![
        Span::styled(
            format!("[{}] ", app.settings.theme.name().to_uppercase()),
            Style::default().fg(pal.accent_secondary),
        ),
        Span::styled(
            format!("[{}] ", app.settings.difficulty.name().to_uppercase()),
            Style::default().fg(pal.warning),
        ),
        Span::styled(
            format!("PTS {:07} ", app.highscores.total_arcade_points()),
            Style::default()
                .fg(pal.success)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(inner);

    frame.render_widget(Paragraph::new(left).alignment(Alignment::Left), cols[0]);
    frame.render_widget(Paragraph::new(right).alignment(Alignment::Right), cols[1]);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let hints = match app.screen {
        Screen::Startup { .. } => "Press any key to skip boot sequence",
        Screen::MainMenu => {
            " [↑↓] Select [ENTER] Play [←→] Diff [T] Theme [O] Config [H] Scores [Q] Quit "
        }
        Screen::Settings => {
            " [↑↓] Row  [←→/ENTER] Change Value  [T] Cycle Theme  [ESC/TAB/Q] Back to Menu "
        }
        Screen::HighScores => {
            " [↑↓/←→] Switch Cartridge  [ENTER] Launch Game  [T] Theme  [ESC/H/Q] Back to Menu "
        }
        Screen::InGame => app
            .active_game
            .as_ref()
            .map(|g| g.id().controls_summary())
            .unwrap_or("[P/ESC] Pause  [R] Restart  [Q] Menu"),
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hints,
            Style::default().fg(pal.fg_muted).bg(pal.panel_bg),
        )))
        .alignment(Alignment::Center)
        .style(Style::default().bg(pal.panel_bg)),
        area,
    );
}

fn render_small_terminal_warning(frame: &mut Frame, area: Rect, pal: &Palette) {
    if area.width < 12 || area.height < 4 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.warning))
        .style(Style::default().bg(pal.panel_bg))
        .title(Span::styled(
            " SIGNAL OUT OF RANGE ",
            Style::default()
                .fg(pal.warning)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled(
            "NEXUSTATION REQUIRES MORE CRT SPACE",
            Style::default()
                .fg(pal.fg_bright)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                "Current: {}x{}  |  Minimum: {}x{}",
                area.width, area.height, MIN_WIDTH, MIN_HEIGHT
            ),
            Style::default().fg(pal.accent),
        )),
        Line::from(Span::styled(
            "Resize terminal window or press [Q] / [Ctrl+C] to exit.",
            Style::default().fg(pal.fg_muted),
        )),
    ];

    let vpad = inner.height.saturating_sub(3) / 2;
    let centered = Rect {
        x: inner.x,
        y: inner.y + vpad,
        width: inner.width,
        height: inner.height.saturating_sub(vpad),
    };
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), centered);
}
