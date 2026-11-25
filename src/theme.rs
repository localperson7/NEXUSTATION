use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ThemeId {
    #[default]
    NeonArcade,
    RetroGreenCrt,
    ClassicMonochrome,
    PurpleCyber,
}

impl ThemeId {
    pub const ALL: [Self; 4] = [
        Self::RetroGreenCrt,
        Self::NeonArcade,
        Self::ClassicMonochrome,
        Self::PurpleCyber,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::RetroGreenCrt => "Retro Green CRT",
            Self::NeonArcade => "Neon Arcade",
            Self::ClassicMonochrome => "Classic Monochrome",
            Self::PurpleCyber => "Purple Cyber",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::RetroGreenCrt => "P1 Emerald Phosphor Monitor (1981)",
            Self::NeonArcade => "NEXUS Graphite, Electric Violet, Blue & Cyan",
            Self::ClassicMonochrome => "Crisp High-Contrast Ivory & Obsidian",
            Self::PurpleCyber => "Synthwave Dusk Plum & Electric Orchid",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&t| t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&t| t == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::RetroGreenCrt => Palette {
                id: self,
                bg: Color::Rgb(5, 16, 9),
                bg_scanline: Color::Rgb(8, 24, 13),
                panel_bg: Color::Rgb(9, 27, 15),
                bezel: Color::Rgb(28, 92, 48),
                border_active: Color::Rgb(57, 255, 110),
                fg: Color::Rgb(82, 242, 122),
                fg_bright: Color::Rgb(190, 255, 205),
                fg_muted: Color::Rgb(36, 122, 60),
                accent: Color::Rgb(57, 255, 110),
                accent_secondary: Color::Rgb(152, 255, 112),
                highlight_bg: Color::Rgb(22, 74, 38),
                success: Color::Rgb(120, 255, 148),
                warning: Color::Rgb(198, 255, 94),
                danger: Color::Rgb(255, 118, 92),
                series: [
                    Color::Rgb(46, 196, 88),
                    Color::Rgb(72, 232, 115),
                    Color::Rgb(118, 250, 142),
                    Color::Rgb(162, 255, 130),
                    Color::Rgb(38, 166, 118),
                    Color::Rgb(96, 224, 168),
                    Color::Rgb(196, 255, 176),
                ],
            },
            Self::NeonArcade => Palette {
                id: self,
                bg: Color::Rgb(10, 11, 16),
                bg_scanline: Color::Rgb(14, 16, 24),
                panel_bg: Color::Rgb(16, 18, 28),
                bezel: Color::Rgb(58, 76, 138),
                border_active: Color::Rgb(0, 228, 255),
                fg: Color::Rgb(226, 234, 252),
                fg_bright: Color::Rgb(255, 255, 255),
                fg_muted: Color::Rgb(106, 120, 164),
                accent: Color::Rgb(0, 228, 255),
                accent_secondary: Color::Rgb(168, 92, 255),
                highlight_bg: Color::Rgb(30, 34, 62),
                success: Color::Rgb(0, 242, 168),
                warning: Color::Rgb(255, 214, 64),
                danger: Color::Rgb(255, 78, 112),
                series: [
                    Color::Rgb(0, 228, 255),
                    Color::Rgb(84, 148, 255),
                    Color::Rgb(168, 92, 255),
                    Color::Rgb(0, 242, 168),
                    Color::Rgb(255, 78, 112),
                    Color::Rgb(255, 214, 64),
                    Color::Rgb(255, 142, 68),
                ],
            },
            Self::ClassicMonochrome => Palette {
                id: self,
                bg: Color::Rgb(12, 12, 14),
                bg_scanline: Color::Rgb(18, 18, 22),
                panel_bg: Color::Rgb(20, 20, 24),
                bezel: Color::Rgb(92, 92, 98),
                border_active: Color::Rgb(242, 242, 236),
                fg: Color::Rgb(214, 214, 208),
                fg_bright: Color::Rgb(252, 252, 248),
                fg_muted: Color::Rgb(112, 112, 116),
                accent: Color::Rgb(245, 245, 240),
                accent_secondary: Color::Rgb(176, 176, 172),
                highlight_bg: Color::Rgb(46, 46, 52),
                success: Color::Rgb(236, 236, 232),
                warning: Color::Rgb(200, 200, 194),
                danger: Color::Rgb(255, 255, 255),
                series: [
                    Color::Rgb(245, 245, 240),
                    Color::Rgb(216, 216, 210),
                    Color::Rgb(190, 190, 184),
                    Color::Rgb(166, 166, 160),
                    Color::Rgb(142, 142, 138),
                    Color::Rgb(204, 204, 198),
                    Color::Rgb(230, 230, 224),
                ],
            },
            Self::PurpleCyber => Palette {
                id: self,
                bg: Color::Rgb(17, 9, 31),
                bg_scanline: Color::Rgb(23, 13, 41),
                panel_bg: Color::Rgb(27, 15, 48),
                bezel: Color::Rgb(98, 56, 152),
                border_active: Color::Rgb(206, 132, 255),
                fg: Color::Rgb(234, 214, 255),
                fg_bright: Color::Rgb(255, 245, 255),
                fg_muted: Color::Rgb(132, 96, 176),
                accent: Color::Rgb(206, 132, 255),
                accent_secondary: Color::Rgb(255, 102, 168),
                highlight_bg: Color::Rgb(50, 26, 86),
                success: Color::Rgb(100, 238, 224),
                warning: Color::Rgb(255, 209, 102),
                danger: Color::Rgb(255, 88, 132),
                series: [
                    Color::Rgb(100, 238, 224),
                    Color::Rgb(255, 209, 102),
                    Color::Rgb(206, 132, 255),
                    Color::Rgb(136, 242, 168),
                    Color::Rgb(255, 96, 152),
                    Color::Rgb(132, 158, 255),
                    Color::Rgb(255, 156, 110),
                ],
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub id: ThemeId,
    pub bg: Color,
    pub bg_scanline: Color,
    pub panel_bg: Color,
    pub bezel: Color,
    pub border_active: Color,
    pub fg: Color,
    pub fg_bright: Color,
    pub fg_muted: Color,
    pub accent: Color,
    pub accent_secondary: Color,
    pub highlight_bg: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub series: [Color; 7],
}

impl Palette {
    pub fn base_style(&self) -> Style {
        Style::default().bg(self.bg).fg(self.fg)
    }

    pub fn panel_style(&self) -> Style {
        Style::default().bg(self.panel_bg).fg(self.fg)
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn row_bg(&self, y: u16, scanlines: bool) -> Color {
        if scanlines && y % 2 == 1 {
            self.bg_scanline
        } else {
            self.bg
        }
    }

    /// Returns `(tile_bg, tile_fg)` for 2048 tiles adapted to the active theme.
    pub fn tile_2048_colors(&self, value: u32) -> (Color, Color) {
        if value == 0 {
            return (self.panel_bg, self.fg_muted);
        }
        match self.id {
            ThemeId::RetroGreenCrt => match value {
                2 => (Color::Rgb(14, 42, 22), Color::Rgb(120, 245, 146)),
                4 => (Color::Rgb(20, 58, 30), Color::Rgb(152, 255, 170)),
                8 => (Color::Rgb(28, 82, 42), Color::Rgb(198, 255, 206)),
                16 => (Color::Rgb(38, 108, 54), Color::Rgb(226, 255, 230)),
                32 => (Color::Rgb(48, 138, 68), Color::Rgb(8, 22, 12)),
                64 => (Color::Rgb(64, 174, 86), Color::Rgb(8, 22, 12)),
                128 => (Color::Rgb(88, 208, 104), Color::Rgb(6, 18, 10)),
                256 => (Color::Rgb(118, 232, 118), Color::Rgb(6, 18, 10)),
                512 => (Color::Rgb(154, 246, 112), Color::Rgb(6, 18, 10)),
                1024 => (Color::Rgb(192, 255, 118), Color::Rgb(5, 16, 9)),
                _ => (Color::Rgb(226, 255, 168), Color::Rgb(5, 16, 9)),
            },
            ThemeId::ClassicMonochrome => match value {
                2 => (Color::Rgb(32, 32, 36), Color::Rgb(218, 218, 212)),
                4 => (Color::Rgb(48, 48, 54), Color::Rgb(236, 236, 230)),
                8 => (Color::Rgb(68, 68, 76), Color::Rgb(250, 250, 246)),
                16 => (Color::Rgb(92, 92, 100), Color::Rgb(255, 255, 255)),
                32 => (Color::Rgb(124, 124, 132), Color::Rgb(12, 12, 14)),
                64 => (Color::Rgb(152, 152, 158), Color::Rgb(12, 12, 14)),
                128 => (Color::Rgb(178, 178, 182), Color::Rgb(10, 10, 12)),
                256 => (Color::Rgb(200, 200, 202), Color::Rgb(10, 10, 12)),
                512 => (Color::Rgb(220, 220, 218), Color::Rgb(8, 8, 10)),
                1024 => (Color::Rgb(238, 238, 234), Color::Rgb(8, 8, 10)),
                _ => (Color::Rgb(255, 255, 250), Color::Rgb(8, 8, 10)),
            },
            ThemeId::NeonArcade => match value {
                2 => (Color::Rgb(24, 36, 68), Color::Rgb(210, 232, 255)),
                4 => (Color::Rgb(32, 54, 96), Color::Rgb(130, 244, 255)),
                8 => (Color::Rgb(0, 146, 186), Color::Rgb(255, 255, 255)),
                16 => (Color::Rgb(0, 184, 148), Color::Rgb(10, 16, 26)),
                32 => (Color::Rgb(122, 76, 224), Color::Rgb(255, 255, 255)),
                64 => (Color::Rgb(214, 44, 128), Color::Rgb(255, 255, 255)),
                128 => (Color::Rgb(242, 114, 44), Color::Rgb(14, 12, 20)),
                256 => (Color::Rgb(255, 176, 28), Color::Rgb(14, 12, 20)),
                512 => (Color::Rgb(255, 218, 42), Color::Rgb(12, 12, 20)),
                1024 => (Color::Rgb(0, 240, 255), Color::Rgb(10, 13, 26)),
                _ => (Color::Rgb(255, 72, 176), Color::Rgb(255, 255, 255)),
            },
            ThemeId::PurpleCyber => match value {
                2 => (Color::Rgb(38, 22, 68), Color::Rgb(232, 212, 255)),
                4 => (Color::Rgb(56, 32, 98), Color::Rgb(244, 224, 255)),
                8 => (Color::Rgb(88, 46, 148), Color::Rgb(255, 255, 255)),
                16 => (Color::Rgb(132, 66, 198), Color::Rgb(255, 255, 255)),
                32 => (Color::Rgb(186, 72, 176), Color::Rgb(255, 255, 255)),
                64 => (Color::Rgb(232, 76, 142), Color::Rgb(255, 255, 255)),
                128 => (Color::Rgb(78, 196, 192), Color::Rgb(17, 9, 31)),
                256 => (Color::Rgb(112, 232, 212), Color::Rgb(17, 9, 31)),
                512 => (Color::Rgb(255, 178, 92), Color::Rgb(17, 9, 31)),
                1024 => (Color::Rgb(255, 216, 108), Color::Rgb(17, 9, 31)),
                _ => (Color::Rgb(236, 168, 255), Color::Rgb(17, 9, 31)),
            },
        }
    }

    pub fn piece_color(&self, index: usize) -> Color {
        self.series[index % self.series.len()]
    }
}
