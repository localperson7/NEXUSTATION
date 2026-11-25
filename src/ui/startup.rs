use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::state::{AppState, BOOT_DURATION_MS};
use crate::theme::Palette;
use crate::ui::canvas::HalfBlockCanvas;

pub fn render_startup(
    frame: &mut Frame,
    area: Rect,
    app: &AppState,
    elapsed_ms: u32,
    pal: &Palette,
) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.border_active))
        .title(Line::from(vec![
            Span::styled(" ◆ NEXUSTATION BIOS v0.1 ", pal.title_style()),
            Span::styled("// NEXUS ECOSYSTEM ", Style::default().fg(pal.fg_muted)),
        ]));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if inner.height < 16 || inner.width < 50 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    // 1. Animated pixel Crab mascot on HalfBlockCanvas
    render_pixel_crab(frame, chunks[1], elapsed_ms, pal, app.settings.scanlines);

    // 2. Retro block-art NEXUSTATION console logo
    let logo_style = Style::default().fg(pal.accent).add_modifier(Modifier::BOLD);
    let sub_style = Style::default()
        .fg(pal.accent_secondary)
        .add_modifier(Modifier::BOLD);
    let logo = vec![
        Line::from(Span::styled(
            "█▄░█ █▀▀ ▀▄▀ █░█ █▀ ▀█▀ ▄▀█ ▀█▀ █ █▀█ █▄░█",
            logo_style,
        )),
        Line::from(Span::styled(
            "█░▀█ ██▄ █░█ █▄█ ▄█ ░█░ █▀█ ░█░ █ █▄█ █░▀█",
            logo_style,
        )),
        Line::from(""),
        Line::from(Span::styled(
            "═══ YOUR TERMINAL. YOUR ARCADE.  //  NEXUS v0.1 ═══",
            sub_style,
        )),
    ];
    frame.render_widget(Paragraph::new(logo).alignment(Alignment::Center), chunks[2]);

    // 3. Progressive hardware self-test checklist (with Rust crab core tribute)
    let ratio = (elapsed_ms as f32 / BOOT_DURATION_MS as f32).clamp(0.0, 1.0);
    let steps = [
        (0.12, "RUST-CRAB 6502X CORE  ", "4.77 MHz  [OK]"),
        (0.35, "NEXUS CRT VRAM BUFFER ", "64 KB     [OK]"),
        (0.60, "CARTRIDGE MULTI-BUS   ", "6 SLOTS   [OK]"),
        (0.82, "LOCAL NVRAM HIGHSCORES", "READY     [OK]"),
    ];
    let mut diag_lines = Vec::with_capacity(5);
    for (threshold, label, result) in steps {
        if ratio >= threshold {
            diag_lines.push(Line::from(vec![
                Span::styled("  ▸ ", Style::default().fg(pal.accent)),
                Span::styled(label, Style::default().fg(pal.fg)),
                Span::styled(" ...... ", Style::default().fg(pal.fg_muted)),
                Span::styled(
                    result,
                    Style::default()
                        .fg(pal.success)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            diag_lines.push(Line::from(vec![
                Span::styled("  ▹ ", Style::default().fg(pal.fg_muted)),
                Span::styled(label, Style::default().fg(pal.fg_muted)),
                Span::styled(" ...... ", Style::default().fg(pal.fg_muted)),
                Span::styled("WAIT", Style::default().fg(pal.warning)),
            ]));
        }
    }

    let diag_width = 48.min(chunks[3].width);
    let diag_rect = Rect {
        x: chunks[3].x + (chunks[3].width.saturating_sub(diag_width)) / 2,
        y: chunks[3].y,
        width: diag_width,
        height: chunks[3].height,
    };
    frame.render_widget(Paragraph::new(diag_lines), diag_rect);

    // 4. Progress bar
    let bar_cells = 36usize;
    let filled = ((bar_cells as f32) * ratio).round() as usize;
    let bar_str = format!(
        "[{}{}] {:>3}%",
        "█".repeat(filled.min(bar_cells)),
        "░".repeat(bar_cells.saturating_sub(filled)),
        (ratio * 100.0) as u32
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            bar_str,
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        chunks[4],
    );

    // 5. Blinking prompt
    let blink_on = (elapsed_ms / 260).is_multiple_of(2);
    let prompt_style = if blink_on {
        Style::default()
            .fg(pal.warning)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(pal.fg_muted)
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "INSERT COIN  //  RELEASE THE CRAB  //  PRESS ANY KEY",
            prompt_style,
        )))
        .alignment(Alignment::Center),
        chunks[5],
    );
}

fn render_pixel_crab(
    frame: &mut Frame,
    area: Rect,
    elapsed_ms: u32,
    pal: &Palette,
    scanlines: bool,
) {
    const CRAB_OPEN: [&str; 10] = [
        "  ##          ##  ",
        " #  #        #  # ",
        " #  #  ####  #  # ",
        "  ##  ######  ##  ",
        "   ############   ",
        "  ###..####..###  ",
        "  ##############  ",
        "   ############   ",
        "  ##  ##  ##  ##  ",
        " #   #      #   # ",
    ];
    const CRAB_SNAP: [&str; 10] = [
        "   ##        ##   ",
        "  ####      ####  ",
        "   ##  ####  ##   ",
        "  ##  ######  ##  ",
        "   ############   ",
        "  ###..####..###  ",
        "  ##############  ",
        "   ############   ",
        "   ## ##  ## ##   ",
        "  #  #      #  #  ",
    ];

    let sprite_w = 18 * 2; // 2 cols per logical pixel for square aspect
    let canvas_w = area.width.min(44);
    let canvas_rect = Rect {
        x: area.x + (area.width.saturating_sub(canvas_w)) / 2,
        y: area.y,
        width: canvas_w,
        height: area.height,
    };

    let mut canvas = HalfBlockCanvas::new(canvas_rect.width, canvas_rect.height);
    let snap = ((elapsed_ms / 220) % 2) == 1;
    let rows = if snap { &CRAB_SNAP } else { &CRAB_OPEN };
    let ox = (i32::from(canvas_rect.width) - sprite_w) / 2;
    let oy = if snap { 1 } else { 2 };

    for (ry, row) in rows.iter().enumerate() {
        for (rx, ch) in row.chars().enumerate() {
            let px = ox + (rx as i32) * 2;
            let py = oy + (ry as i32);
            match ch {
                '#' => {
                    canvas.set(px, py, pal.accent_secondary);
                    canvas.set(px + 1, py, pal.accent_secondary);
                }
                '.' => {
                    canvas.set(px, py, pal.fg_bright);
                    canvas.set(px + 1, py, pal.fg_bright);
                }
                _ => {}
            }
        }
    }

    canvas.render(
        canvas_rect,
        frame.buffer_mut(),
        pal.bg,
        pal.bg_scanline,
        scanlines,
    );
}
