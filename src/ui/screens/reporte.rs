//! Pantalla genérica de reporte (texto con barras opcionales).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Dibuja `app.reporte`.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let reporte = &app.reporte;
    let alto_grafico = if !reporte.barras.is_empty() && area.height >= 20 {
        12
    } else {
        0
    };
    let [grafico_area, texto_area] =
        Layout::vertical([Constraint::Length(alto_grafico), Constraint::Min(3)]).areas(area);
    if alto_grafico > 0 {
        let barras: Vec<Bar> = reporte
            .barras
            .iter()
            .map(|(etiqueta, valor)| {
                Bar::default()
                    .label(etiqueta.chars().take(8).collect::<String>())
                    .value(*valor)
            })
            .collect();
        frame.render_widget(
            BarChart::new(barras)
                .bar_width(8)
                .bar_gap(1)
                .bar_style(Style::default().fg(colors.accent))
                .value_style(Style::default().fg(colors.fg))
                .label_style(Style::default().fg(colors.muted))
                .block(colors.panel(" Gráfico ")),
            grafico_area,
        );
    }
    let lineas: Vec<Line> = reporte
        .lineas
        .iter()
        .map(|linea| match linea.strip_prefix("== ") {
            Some(titulo) => Line::from(Span::styled(
                titulo.to_string(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            None => Line::from(Span::styled(linea.clone(), Style::default().fg(colors.fg))),
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lineas)
            .block(colors.panel(format!(" {} ", reporte.titulo)))
            .wrap(Wrap { trim: false })
            .scroll((app.reporte_scroll, 0)),
        texto_area,
    );
}

/// Formulario de una elección (`Screen::Eleccion`).
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
