//! Paletas de color por tema (§19.7).

use ratatui::style::Color;

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

/// Devuelve la paleta del tema configurado.
#[must_use]
pub const fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Light => Palette {
            bg: Color::White,
            fg: Color::Black,
            muted: Color::DarkGray,
            accent: Color::Blue,
            success: Color::Green,
            warning: Color::LightYellow,
            error: Color::Red,
            border: Color::Gray,
            highlight_bg: Color::Blue,
            highlight_fg: Color::White,
        },
        Theme::Dark => Palette {
            bg: Color::Black,
            fg: Color::White,
            muted: Color::DarkGray,
            accent: Color::Cyan,
            success: Color::LightGreen,
            warning: Color::LightYellow,
            error: Color::LightRed,
            border: Color::DarkGray,
            highlight_bg: Color::LightCyan,
            highlight_fg: Color::Black,
        },
        Theme::Blue => Palette {
            bg: Color::Black,
            fg: Color::LightCyan,
            muted: Color::Blue,
            accent: Color::LightBlue,
            success: Color::LightGreen,
            warning: Color::LightYellow,
            error: Color::LightRed,
            border: Color::Blue,
            highlight_bg: Color::Blue,
            highlight_fg: Color::White,
        },
        Theme::Green => Palette {
            bg: Color::Black,
            fg: Color::LightGreen,
            muted: Color::DarkGray,
            accent: Color::Green,
            success: Color::LightGreen,
            warning: Color::Yellow,
            error: Color::LightRed,
            border: Color::Green,
            highlight_bg: Color::Green,
            highlight_fg: Color::Black,
        },
        Theme::Red => Palette {
            bg: Color::Black,
            fg: Color::LightRed,
            muted: Color::DarkGray,
            accent: Color::Red,
            success: Color::LightGreen,
            warning: Color::Yellow,
            error: Color::LightRed,
            border: Color::Red,
            highlight_bg: Color::Red,
            highlight_fg: Color::White,
        },
        Theme::Purple => Palette {
            bg: Color::Black,
            fg: Color::LightMagenta,
            muted: Color::DarkGray,
            accent: Color::Magenta,
            success: Color::LightGreen,
            warning: Color::LightYellow,
            error: Color::LightRed,
            border: Color::Magenta,
            highlight_bg: Color::Magenta,
            highlight_fg: Color::Black,
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
