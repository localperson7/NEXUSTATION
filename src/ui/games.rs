use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

use crate::games::{
    ActiveCartridge, ArcadeGame, GameStatus,
    blockfall::{BlockfallGame, WELL_H, WELL_W},
    breakout::{ARENA_H, ARENA_W, BreakoutGame, PADDLE_Y},
    dino::{DINO_X, DinoGame, ObstacleKind, WORLD_WIDTH},
    game_2048::{BOARD_SIZE, Game2048},
    pong::{COURT_H, COURT_W, CPU_X, PLAYER_X, PongGame},
    snake::{Direction as SnakeDir, GRID_H, GRID_W, GridPoint, SnakeGame},
};
use crate::state::AppState;
use crate::theme::Palette;
use crate::ui::{
    canvas::HalfBlockCanvas,
    overlays::{centered_rect, render_gameover_overlay, render_hud_sidebar, render_pause_overlay},
};

pub fn render_in_game(frame: &mut Frame, area: Rect, app: &AppState, pal: &Palette) {
    let Some(game) = &app.active_game else {
        return;
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(52), Constraint::Length(22)])
        .split(area);

    let stage_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(pal.border_active))
        .style(Style::default().bg(pal.bg))
        .title(Line::from(vec![
            Span::styled(format!(" ◆ {} ", game.id().title()), pal.title_style()),
            Span::styled(
                format!("// {} ", game.id().genre()),
                Style::default().fg(pal.fg_muted),
            ),
        ]));
    let stage_inner = stage_block.inner(cols[0]);
    frame.render_widget(stage_block, cols[0]);

    match game {
        ActiveCartridge::Game2048(g) => render_2048(frame, stage_inner, g, pal),
        ActiveCartridge::Snake(g) => render_snake(frame, stage_inner, g, app, pal),
        ActiveCartridge::Dino(g) => render_dino(frame, stage_inner, g, app, pal),
        ActiveCartridge::Blockfall(g) => render_blockfall(frame, stage_inner, g, pal),
        ActiveCartridge::Pong(g) => render_pong(frame, stage_inner, g, app, pal),
        ActiveCartridge::Breakout(g) => render_breakout(frame, stage_inner, g, app, pal),
    }

    render_hud_sidebar(frame, cols[1], game, app, pal);

    if app.paused {
        render_pause_overlay(frame, cols[0], app, pal);
    } else if game.status() != GameStatus::Playing {
        render_gameover_overlay(frame, cols[0], game, app, pal);
    }
}

fn render_2048(frame: &mut Frame, area: Rect, game: &Game2048, pal: &Palette) {
    let tile_w: u16 = 10;
    let tile_h: u16 = 3;
    let grid_w = tile_w * (BOARD_SIZE as u16) + 2;
    let grid_h = tile_h * (BOARD_SIZE as u16) + 2;
    let board_rect = centered_rect(grid_w, grid_h, area);

    let frame_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(pal.bezel))
        .style(Style::default().bg(pal.panel_bg));
    let inner = frame_block.inner(board_rect);
    frame.render_widget(frame_block, board_rect);

    if inner.width < tile_w * 4 || inner.height < tile_h * 4 {
        return;
    }

    let buf = frame.buffer_mut();
    for r in 0..BOARD_SIZE {
        for c in 0..BOARD_SIZE {
            let val = game.board()[r][c];
            let (tbg, tfg) = pal.tile_2048_colors(val);
            let is_merged = game.is_flashing() && game.merged_mask()[r][c];
            let is_spawn = game.is_flashing() && game.last_spawn() == Some((r, c));

            let rx = inner.x + (c as u16) * tile_w;
            let ry = inner.y + (r as u16) * tile_h;
            let tile_rect = Rect {
                x: rx,
                y: ry,
                width: tile_w,
                height: tile_h,
            };

            let border_fg = if is_merged {
                pal.warning
            } else if is_spawn {
                pal.accent
            } else if val == 0 {
                pal.bezel
            } else {
                tfg
            };

            let tile_block = Block::default()
                .borders(Borders::ALL)
                .border_type(if val == 0 {
                    BorderType::Plain
                } else {
                    BorderType::Rounded
                })
                .border_style(Style::default().fg(border_fg).bg(tbg))
                .style(Style::default().bg(tbg));
            tile_block.render_into_buf(tile_rect, buf);

            if val > 0 {
                let label = val.to_string();
                let lx = rx + (tile_w.saturating_sub(label.len() as u16)) / 2;
                let ly = ry + tile_h / 2;
                buf.set_string(
                    lx,
                    ly,
                    &label,
                    Style::default()
                        .fg(tfg)
                        .bg(tbg)
                        .add_modifier(Modifier::BOLD),
                );
            } else {
                buf.set_string(
                    rx + tile_w / 2 - 1,
                    ry + tile_h / 2,
                    "··",
                    Style::default().fg(pal.fg_muted).bg(tbg),
                );
            }
        }
    }
}

trait RenderIntoBuf {
    fn render_into_buf(self, area: Rect, buf: &mut ratatui::buffer::Buffer);
}

impl RenderIntoBuf for Block<'_> {
    fn render_into_buf(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        ratatui::widgets::Widget::render(self, area, buf);
    }
}

fn render_snake(frame: &mut Frame, area: Rect, game: &SnakeGame, app: &AppState, pal: &Palette) {
    let arena_w = (GRID_W as u16) * 2 + 2;
    let arena_h = (GRID_H as u16) + 2;
    let rect = centered_rect(arena_w, arena_h, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(pal.accent))
        .style(Style::default().bg(pal.panel_bg));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.width < (GRID_W as u16) * 2 || inner.height < GRID_H as u16 {
        return;
    }

    let buf = frame.buffer_mut();
    for y in 0..GRID_H {
        let sy = inner.y + y as u16;
        let row_bg = pal.row_bg(sy, app.settings.scanlines);
        for x in 0..GRID_W {
            let sx = inner.x + (x as u16) * 2;
            buf.set_string(sx, sy, "· ", Style::default().fg(pal.fg_muted).bg(row_bg));
        }
    }

    // Regular food
    let food = game.food();
    let fy = inner.y + food.y as u16;
    let fx = inner.x + (food.x as u16) * 2;
    let pulse = app.idle_tick.is_multiple_of(2);
    buf.set_string(
        fx,
        fy,
        if pulse { "◆◆" } else { "◈◈" },
        Style::default()
            .fg(pal.danger)
            .bg(pal.row_bg(fy, app.settings.scanlines))
            .add_modifier(Modifier::BOLD),
    );

    // Bonus food
    if let Some((bf, _ttl)) = game.bonus_food() {
        let by = inner.y + bf.y as u16;
        let bx = inner.x + (bf.x as u16) * 2;
        buf.set_string(
            bx,
            by,
            "★★",
            Style::default()
                .fg(pal.warning)
                .bg(pal.row_bg(by, app.settings.scanlines))
                .add_modifier(Modifier::BOLD),
        );
    }

    // Snake body & head
    for (idx, &GridPoint { x, y }) in game.body().iter().enumerate().rev() {
        if !(0..GRID_W).contains(&x) || !(0..GRID_H).contains(&y) {
            continue;
        }
        let sx = inner.x + (x as u16) * 2;
        let sy = inner.y + y as u16;
        let rbg = pal.row_bg(sy, app.settings.scanlines);
        if idx == 0 {
            let head_glyph = match game.direction() {
                SnakeDir::Up => "▀▀",
                SnakeDir::Down => "▄▄",
                SnakeDir::Left => "▌█",
                SnakeDir::Right => "█▐",
            };
            buf.set_string(
                sx,
                sy,
                head_glyph,
                Style::default()
                    .fg(pal.fg_bright)
                    .bg(pal.success)
                    .add_modifier(Modifier::BOLD),
            );
        } else {
            let glyph = if idx % 2 == 0 { "██" } else { "▓▓" };
            let col = if idx < 4 { pal.success } else { pal.accent };
            buf.set_string(sx, sy, glyph, Style::default().fg(col).bg(rbg));
        }
    }
}

fn render_dino(frame: &mut Frame, area: Rect, game: &DinoGame, app: &AppState, pal: &Palette) {
    if area.width < 20 || area.height < 10 {
        return;
    }

    let viewport_h = area.height.min(18);
    let viewport = Rect {
        x: area.x,
        y: area.y + (area.height.saturating_sub(viewport_h)) / 2,
        width: area.width,
        height: viewport_h,
    };

    let mut canvas = HalfBlockCanvas::new(viewport.width, viewport.height);
    let h_px = canvas.height_px() as i32;
    let w_px = canvas.width() as i32;
    let ground_py = h_px - 5;
    let x_scale = (w_px as f32) / WORLD_WIDTH;
    let y_scale = 1.6_f32;

    // Parallax clouds / stars
    let cloud_shift = ((game.ground_offset() / 3) as i32) % w_px.max(1);
    for idx in 0..3 {
        let cx = ((idx * 37 + w_px - cloud_shift) % w_px.max(1)) - 4;
        let cy = 3 + (idx % 2) * 3;
        canvas.draw_sprite(cx, cy, &["  ####  ", "########"], pal.fg_muted);
    }

    // Horizon line + scrolling pebbles
    canvas.fill_rect(0, ground_py, w_px, 1, pal.bezel);
    let scroll = game.ground_offset() as i32;
    for px in 0..w_px {
        if (px + scroll) % 7 == 0 {
            canvas.set(px, ground_py + 2, pal.fg_muted);
        } else if (px + scroll) % 13 == 0 {
            canvas.set(px, ground_py + 3, pal.fg_muted);
        }
    }

    // Obstacles
    for obs in game.obstacles() {
        let ox = (obs.x * x_scale).round() as i32;
        let bottom_py = ground_py - (obs.y * y_scale).round() as i32;
        match obs.kind {
            ObstacleKind::SmallCactus => {
                let sp = [" # ", "###", "###", " # ", " # ", " # "];
                canvas.draw_sprite(ox, bottom_py - 6, &sp, pal.success);
            }
            ObstacleKind::TallCactus => {
                let sp = [
                    "  #  ", "# #  ", "### #", "#####", "  # #", "  #  ", "  #  ", "  #  ", "  #  ",
                ];
                canvas.draw_sprite(ox, bottom_py - 9, &sp, pal.success);
            }
            ObstacleKind::CactusCluster => {
                let sp = [
                    " #   #  ", "### ### ", "####### ", " # # #  ", " # # #  ", " # # #  ",
                    " # # #  ",
                ];
                canvas.draw_sprite(ox, bottom_py - 7, &sp, pal.success);
            }
            ObstacleKind::PteroMid | ObstacleKind::PteroHigh => {
                let flap = if game.anim_frame() == 0 {
                    ["  ##    ", " ####   ", "########", "   ###  "]
                } else {
                    ["   ##   ", "########", " ####   ", "  #     "]
                };
                canvas.draw_sprite(ox, bottom_py - 4, &flap, pal.warning);
            }
        }
    }

    // Dino pixel sprite
    let dx = (DINO_X * x_scale).round() as i32;
    let dino_bottom = ground_py - (game.dino_y() * y_scale).round() as i32;
    if game.is_ducking() {
        let crouch = [
            "    #######",
            "#####.#####",
            "  #######  ",
            "  #  #     ",
            "  #   #    ",
        ];
        canvas.draw_sprite(dx, dino_bottom - 5, &crouch, pal.accent);
    } else {
        let run_a = [
            "    #####",
            "    #.###",
            "    #####",
            "#  ##### ",
            "#######  ",
            " #####   ",
            "  #  #   ",
            "  ##     ",
        ];
        let run_b = [
            "    #####",
            "    #.###",
            "    #####",
            "#  ##### ",
            "#######  ",
            " #####   ",
            "   # #   ",
            "     ##  ",
        ];
        let sprite = if game.dino_y() > 0.2 || game.anim_frame() == 0 {
            &run_a
        } else {
            &run_b
        };
        canvas.draw_sprite(dx, dino_bottom - 8, sprite, pal.accent);
    }

    canvas.render(
        viewport,
        frame.buffer_mut(),
        pal.bg,
        pal.bg_scanline,
        app.settings.scanlines,
    );

    // Classic arcade runner odometer in top-right of viewport
    let odo = format!(
        " HI {:05}   SCORE {:05} ",
        app.highscores.best_score(game.id()).max(game.score()),
        game.score()
    );
    let ox = viewport.x + viewport.width.saturating_sub(odo.len() as u16 + 1);
    frame.buffer_mut().set_string(
        ox,
        viewport.y,
        &odo,
        Style::default()
            .fg(pal.fg_bright)
            .bg(pal.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
}

fn render_blockfall(frame: &mut Frame, area: Rect, game: &BlockfallGame, pal: &Palette) {
    if area.width < (WELL_W as u16) * 2 + 2 || area.height < 8 {
        return;
    }

    // Open-top matrix rails (║ ... ║ and ╚═══╝) fit all 20 rows even in compact 27-row terminals
    let visible_rows = WELL_H.min(usize::from(area.height.saturating_sub(1)));
    let y_start = WELL_H - visible_rows;
    let rail_w = (WELL_W as u16) * 2 + 2;
    let rail_h = (visible_rows as u16) + 1;
    let well_rect = centered_rect(rail_w, rail_h, area);
    let inner_x = well_rect.x + 1;
    let inner_y = well_rect.y;

    let buf = frame.buffer_mut();
    let wall_style = Style::default().fg(pal.border_active).bg(pal.bg);

    // 1. Rails + background grid + locked blocks
    for row_idx in 0..visible_rows {
        let wy = y_start + row_idx;
        let sy = inner_y + row_idx as u16;
        buf.set_string(well_rect.x, sy, "║", wall_style);
        buf.set_string(well_rect.x + rail_w - 1, sy, "║", wall_style);

        for x in 0..WELL_W {
            let sx = inner_x + (x as u16) * 2;
            if let Some(color_idx) = game.well()[wy][x] {
                let c = pal.piece_color(usize::from(color_idx));
                buf.set_string(sx, sy, "▓▓", Style::default().fg(c).bg(pal.panel_bg));
            } else {
                buf.set_string(
                    sx,
                    sy,
                    "· ",
                    Style::default().fg(pal.fg_muted).bg(pal.panel_bg),
                );
            }
        }
    }

    let bot_y = inner_y + visible_rows as u16;
    let floor = format!("╚{}╝", "═".repeat(WELL_W * 2));
    buf.set_string(well_rect.x, bot_y, &floor, wall_style);

    // 2. Ghost landing silhouette
    let active = game.active();
    let active_col = pal.piece_color(usize::from(active.kind.color_index()));
    let gy = game.ghost_y();
    let ghost = crate::games::blockfall::ActivePiece { y: gy, ..active };
    for (wx, wy) in ghost.world_cells() {
        if (0..WELL_W as i32).contains(&wx) && wy >= y_start as i32 && wy < WELL_H as i32 {
            let sx = inner_x + (wx as u16) * 2;
            let sy = inner_y + (wy - y_start as i32) as u16;
            buf.set_string(
                sx,
                sy,
                "░░",
                Style::default().fg(active_col).bg(pal.panel_bg),
            );
        }
    }

    // 3. Active falling polyomino
    for (wx, wy) in active.world_cells() {
        if (0..WELL_W as i32).contains(&wx) && wy >= y_start as i32 && wy < WELL_H as i32 {
            let sx = inner_x + (wx as u16) * 2;
            let sy = inner_y + (wy - y_start as i32) as u16;
            buf.set_string(
                sx,
                sy,
                "██",
                Style::default()
                    .fg(active_col)
                    .bg(pal.panel_bg)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }
}

fn render_pong(frame: &mut Frame, area: Rect, game: &PongGame, app: &AppState, pal: &Palette) {
    let court_rect = centered_rect(area.width.min(82), area.height.min(24), area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(pal.bezel))
        .style(Style::default().bg(pal.bg))
        .title(Line::from(vec![
            Span::styled(
                " YOU ",
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("VS", Style::default().fg(pal.fg_muted)),
            Span::styled(
                " CPU ",
                Style::default()
                    .fg(pal.accent_secondary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    let inner = block.inner(court_rect);
    frame.render_widget(block, court_rect);

    if inner.width < 20 || inner.height < 8 {
        return;
    }

    let mut canvas = HalfBlockCanvas::new(inner.width, inner.height);
    let w_px = canvas.width() as f32;
    let h_px = canvas.height_px() as f32;
    let sx = w_px / COURT_W;
    let sy = h_px / COURT_H;

    // Dashed center net
    let net_x = (w_px * 0.5).round() as i32;
    for py in (1..canvas.height_px() as i32 - 1).step_by(3) {
        canvas.fill_rect(net_x, py, 1, 2, pal.bezel);
    }

    // Large pixel scoreboard digits at top
    canvas.draw_digit(net_x - 8, 2, game.player_goals(), pal.accent);
    canvas.draw_digit(net_x + 5, 2, game.cpu_goals(), pal.accent_secondary);

    // Paddles
    let pad_h_px = (game.paddle_height() * sy).round().max(3.0) as i32;
    let half_px = pad_h_px / 2;
    let p_x = (PLAYER_X * sx).round() as i32;
    let p_y = (game.player_y() * sy).round() as i32 - half_px;
    canvas.fill_rect(p_x, p_y, 2, pad_h_px, pal.accent);

    let c_x = (CPU_X * sx).round() as i32 - 1;
    let c_y = (game.cpu_y() * sy).round() as i32 - half_px;
    canvas.fill_rect(c_x, c_y, 2, pad_h_px, pal.accent_secondary);

    // Ball motion trail
    for (idx, &(tx, ty)) in game.trail().iter().enumerate().rev() {
        let px = (tx * sx).round() as i32;
        let py = (ty * sy).round() as i32;
        let col = if idx < 2 { pal.warning } else { pal.fg_muted };
        canvas.set(px, py, col);
    }

    // Ball
    let (bx, by) = game.ball_pos();
    let bpx = (bx * sx).round() as i32;
    let bpy = (by * sy).round() as i32;
    canvas.fill_rect(bpx, bpy, 2, 2, pal.fg_bright);

    canvas.render(
        inner,
        frame.buffer_mut(),
        pal.bg,
        pal.bg_scanline,
        app.settings.scanlines,
    );
}

fn render_breakout(
    frame: &mut Frame,
    area: Rect,
    game: &BreakoutGame,
    app: &AppState,
    pal: &Palette,
) {
    let arena_rect = centered_rect(area.width.min(82), area.height.min(25), area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(pal.border_active))
        .style(Style::default().bg(pal.bg))
        .title(Line::from(vec![
            Span::styled(
                format!(" LEVEL {} ", game.level()),
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(pal.bezel)),
            Span::styled(
                format!("LIVES {} ", "♥".repeat(game.lives() as usize)),
                Style::default().fg(pal.danger).add_modifier(Modifier::BOLD),
            ),
        ]));
    let inner = block.inner(arena_rect);
    frame.render_widget(block, arena_rect);

    if inner.width < 20 || inner.height < 10 {
        return;
    }

    let mut canvas = HalfBlockCanvas::new(inner.width, inner.height);
    let w_px = canvas.width() as f32;
    let h_px = canvas.height_px() as f32;
    let sx = w_px / ARENA_W;
    let sy = h_px / ARENA_H;

    // Bricks
    for brick in game.bricks() {
        if brick.hp == 0 {
            continue;
        }
        let x0 = (brick.x * sx).round() as i32;
        let y0 = (brick.y * sy).round() as i32;
        let bw = (brick.w * sx).round().max(2.0) as i32;
        let bh = (brick.h * sy).round().max(2.0) as i32;
        let base_col = pal.piece_color(brick.row);
        canvas.fill_rect(x0, y0, bw, bh, base_col);
        // Armor sheen on 2-HP bricks
        if brick.hp > 1 && bh >= 2 {
            canvas.fill_rect(x0, y0, bw, 1, pal.fg_bright);
        }
    }

    // Paddle
    let pw = (game.paddle_width() * sx).round().max(6.0) as i32;
    let px = (game.paddle_x() * sx).round() as i32 - pw / 2;
    let py = (PADDLE_Y * sy).round() as i32;
    canvas.fill_rect(px, py, pw, 2, pal.accent);
    canvas.fill_rect(px + 2, py, (pw - 4).max(2), 1, pal.fg_bright);

    // Ball
    let (bx, by) = game.ball_pos();
    let bpx = (bx * sx).round() as i32;
    let bpy = (by * sy).round() as i32;
    canvas.fill_rect(bpx, bpy, 2, 2, pal.warning);

    canvas.render(
        inner,
        frame.buffer_mut(),
        pal.bg,
        pal.bg_scanline,
        app.settings.scanlines,
    );

    if game.is_ball_attached() && game.status() == GameStatus::Playing {
        let prompt = " PRESS [SPACE] TO LAUNCH BALL ";
        let tx = inner.x + inner.width.saturating_sub(prompt.len() as u16) / 2;
        let ty = inner.y + inner.height / 2 + 2;
        if ty < inner.y + inner.height {
            frame.buffer_mut().set_string(
                tx,
                ty,
                prompt,
                Style::default()
                    .fg(pal.bg)
                    .bg(pal.warning)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }
}
