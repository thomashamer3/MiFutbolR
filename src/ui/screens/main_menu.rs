//! Menú principal con los 11 módulos (§19.2).
//!
//! En pantallas anchas muestra una tarjeta de vista previa del módulo
//! seleccionado; en pantallas angostas apila la lista y el detalle.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Module};
use crate::ui::theme::palette;

/// Dibuja el menú y la ficha del módulo seleccionado.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let ancho = area.width >= 100;

    let (list_area, detail_area) = if ancho {
        let [izquierda, derecha] =
            Layout::horizontal([Constraint::Percentage(44), Constraint::Min(40)])
                .spacing(1)
                .areas(area);
        (izquierda, derecha)
    } else {
        let [arriba, abajo] =
            Layout::vertical([Constraint::Min(8), Constraint::Length(7)]).areas(area);
        (arriba, abajo)
    };

    let items: Vec<ListItem> = Module::ALL
        .iter()
        .enumerate()
        .map(|(index, module)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {:>2} ", index + 1),
                    Style::default()
                        .fg(colors.highlight_fg)
                        .bg(colors.border)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(module.label(), Style::default().fg(colors.fg)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" Módulos · 1-9 acceso rápido ")
                .border_style(Style::default().fg(colors.border))
                .style(Style::default().bg(colors.bg)),
        )
        .highlight_style(
            Style::default()
                .bg(colors.highlight_bg)
                .fg(colors.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, list_area, &mut app.menu_state);

    let selected = app.selected_module();
    let indice = Module::ALL
        .iter()
        .position(|module| *module == selected)
        .map_or(1, |posicion| posicion + 1);

    let mut lineas = vec![
        Line::from(Span::styled(
            selected.label(),
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            selected.description(),
            Style::default().fg(colors.fg),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Módulo ", Style::default().fg(colors.muted)),
            Span::styled(
                format!("{indice} de {}", Module::ALL.len()),
                Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   ·   tema ", Style::default().fg(colors.muted)),
            Span::styled(
                app.settings.theme.label(),
                Style::default().fg(colors.accent),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Atajos",
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  ↑↓/jk  mover      Enter  abrir      ?  ayuda",
            Style::default().fg(colors.fg),
        )),
        Line::from(Span::styled(
            "  1-9   ir directo    q  salir",
            Style::default().fg(colors.fg),
        )),
    ];
    if area.width < 60 {
        lineas.retain(|linea| linea.width() <= area.width as usize);
    }

    let detail = Paragraph::new(lineas).wrap(Wrap { trim: true }).block(
        Block::bordered()
            .title(" Vista previa ")
            .border_style(Style::default().fg(colors.border))
            .style(Style::default().bg(colors.bg)),
    );
    frame.render_widget(detail, detail_area);
}
