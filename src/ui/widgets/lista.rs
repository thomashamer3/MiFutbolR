//! Tabla de listado estándar para las pantallas de módulos.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{
    Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Table, TableState, Wrap,
};

use crate::ui::theme::Palette;

/// Dibuja el listado con el estilo común de la aplicación.
#[allow(clippy::too_many_arguments)]
pub fn render_tabla(
    frame: &mut Frame,
    area: Rect,
    colors: &Palette,
    titulo: impl Into<String>,
    encabezados: Vec<&str>,
    anchos: Vec<Constraint>,
    filas: Vec<Row>,
    state: &mut TableState,
) {
    let header = Row::new(encabezados).style(
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    );
    let table = Table::new(filas, anchos)
        .header(header)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(titulo.into())
                .border_style(Style::default().fg(colors.border))
                .style(Style::default().bg(colors.bg)),
        )
        .column_spacing(1)
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(
            Style::default()
                .bg(colors.highlight_bg)
                .fg(colors.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(table, area, state);
}

/// Dibuja el estado vacío de un listado.
pub fn render_vacio(
    frame: &mut Frame,
    area: Rect,
    colors: &Palette,
    titulo: impl Into<String>,
    mensaje: impl Into<String>,
) {
    frame.render_widget(
        Paragraph::new(mensaje.into())
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(titulo.into())
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(colors.fg)),
        area,
    );
}

/// Celda de estado activo/inactivo con el estilo común.
#[must_use]
pub fn celda_activa(
    activa: bool,
    colors: &Palette,
    texto_activo: &str,
    texto_inactivo: &str,
) -> Cell<'static> {
    Cell::from(if activa {
        texto_activo.to_string()
    } else {
        texto_inactivo.to_string()
    })
    .style(
        Style::default()
            .fg(if activa { colors.success } else { colors.muted })
            .add_modifier(Modifier::BOLD),
    )
}
