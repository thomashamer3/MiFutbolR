//! Paletas de color y estilos reutilizables (§19.7).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Padding};

use crate::domain::models::Theme;

/// Colores de la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Fondo general.
    pub bg: Color,
    /// Texto normal.
    pub fg: Color,
    /// Texto secundario.
    pub muted: Color,
    /// Acento (títulos, selección de foco).
    pub accent: Color,
    /// Éxito.
    pub success: Color,
    /// Advertencia.
    pub warning: Color,
    /// Error.
    pub error: Color,
    /// Bordes de bloques.
    pub border: Color,
    /// Fondo del elemento seleccionado.
    pub highlight_bg: Color,
    /// Texto del elemento seleccionado.
    pub highlight_fg: Color,
}

impl Palette {
    /// Panel estándar: borde redondeado, padding interno y título.
    pub fn panel<'a>(&self, title: impl Into<Line<'a>>) -> Block<'a> {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.border))
            .title(title)
            .padding(Padding::horizontal(1))
            .style(Style::default().bg(self.bg))
    }

    /// Estilo de títulos y encabezados.
    #[must_use]
    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Estilo de texto secundario.
    #[must_use]
    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.muted)
    }
}

/// Devuelve la paleta del tema configurado.
#[must_use]
pub const fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Light => Palette {
            bg: Color::Rgb(250, 247, 240),
            fg: Color::Rgb(31, 41, 51),
            muted: Color::Rgb(107, 114, 128),
            accent: Color::Rgb(15, 118, 110),
            success: Color::Rgb(21, 128, 61),
            warning: Color::Rgb(180, 83, 9),
            error: Color::Rgb(185, 28, 28),
            border: Color::Rgb(214, 211, 209),
            highlight_bg: Color::Rgb(15, 118, 110),
            highlight_fg: Color::Rgb(255, 255, 255),
        },
        Theme::Dark => Palette {
            bg: Color::Rgb(15, 17, 21),
            fg: Color::Rgb(230, 225, 214),
            muted: Color::Rgb(139, 148, 158),
            accent: Color::Rgb(94, 234, 212),
            success: Color::Rgb(52, 211, 153),
            warning: Color::Rgb(251, 191, 36),
            error: Color::Rgb(248, 113, 113),
            border: Color::Rgb(48, 54, 61),
            highlight_bg: Color::Rgb(94, 234, 212),
            highlight_fg: Color::Rgb(15, 17, 21),
        },
        Theme::Blue => Palette {
            bg: Color::Rgb(11, 18, 32),
            fg: Color::Rgb(220, 231, 255),
            muted: Color::Rgb(124, 141, 181),
            accent: Color::Rgb(96, 165, 250),
            success: Color::Rgb(74, 222, 128),
            warning: Color::Rgb(250, 204, 21),
            error: Color::Rgb(248, 113, 113),
            border: Color::Rgb(30, 58, 95),
            highlight_bg: Color::Rgb(96, 165, 250),
            highlight_fg: Color::Rgb(11, 18, 32),
        },
        Theme::Green => Palette {
            bg: Color::Rgb(11, 21, 18),
            fg: Color::Rgb(209, 250, 229),
            muted: Color::Rgb(107, 144, 128),
            accent: Color::Rgb(74, 222, 128),
            success: Color::Rgb(52, 211, 153),
            warning: Color::Rgb(253, 230, 138),
            error: Color::Rgb(248, 113, 113),
            border: Color::Rgb(31, 61, 51),
            highlight_bg: Color::Rgb(74, 222, 128),
            highlight_fg: Color::Rgb(11, 21, 18),
        },
        Theme::Red => Palette {
            bg: Color::Rgb(26, 15, 18),
            fg: Color::Rgb(255, 228, 230),
            muted: Color::Rgb(176, 138, 144),
            accent: Color::Rgb(251, 113, 133),
            success: Color::Rgb(74, 222, 128),
            warning: Color::Rgb(253, 230, 138),
            error: Color::Rgb(248, 113, 113),
            border: Color::Rgb(76, 29, 36),
            highlight_bg: Color::Rgb(251, 113, 133),
            highlight_fg: Color::Rgb(26, 15, 18),
        },
        Theme::Purple => Palette {
            bg: Color::Rgb(20, 15, 31),
            fg: Color::Rgb(237, 233, 254),
            muted: Color::Rgb(154, 143, 184),
            accent: Color::Rgb(192, 132, 252),
            success: Color::Rgb(74, 222, 128),
            warning: Color::Rgb(253, 230, 138),
            error: Color::Rgb(251, 113, 133),
            border: Color::Rgb(59, 45, 84),
            highlight_bg: Color::Rgb(192, 132, 252),
            highlight_fg: Color::Rgb(20, 15, 31),
        },
        Theme::Classic => Palette {
            bg: Color::Black,
            fg: Color::White,
            muted: Color::Gray,
            accent: Color::White,
            success: Color::White,
            warning: Color::White,
            error: Color::White,
            border: Color::White,
            highlight_bg: Color::White,
            highlight_fg: Color::Black,
        },
        Theme::HighContrast => Palette {
            bg: Color::Black,
            fg: Color::White,
            muted: Color::White,
            accent: Color::Yellow,
            success: Color::White,
            warning: Color::Yellow,
            error: Color::White,
            border: Color::White,
            highlight_bg: Color::White,
            highlight_fg: Color::Black,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_has_contrasting_highlight() {
        for theme in Theme::ALL {
            let palette = palette(*theme);
            assert_ne!(palette.highlight_bg, palette.highlight_fg);
        }
    }
}
