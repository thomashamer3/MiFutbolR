//! Overlays de búsqueda global y paleta de comandos (IMP-03).

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Wrap};

use crate::app::{App, BusquedaState, Overlay, PaletaState};
use crate::ui::theme::Palette;

/// Dibuja el overlay activo.
pub fn draw(frame: &mut Frame, area: Rect, app: &App, colors: &Palette) {
    let Some(overlay) = &app.overlay else {
        return;
    };
    let ancho = (area.width * 3 / 4).clamp(48, 110);
    let alto = (area.height * 2 / 3).clamp(10, 28);
    let popup = Rect {
        x: area.x + area.width.saturating_sub(ancho) / 2,
        y: area.y + area.height.saturating_sub(alto) / 2,
        width: ancho,
        height: alto,
    };
    frame.render_widget(Clear, popup);
    match overlay {
        Overlay::Busqueda(estado) => draw_busqueda(frame, popup, estado, colors),
        Overlay::Paleta(estado) => draw_paleta(frame, popup, estado, colors),
    }
}

fn draw_busqueda(frame: &mut Frame, area: Rect, estado: &BusquedaState, colors: &Palette) {
    let [input_area, lista_area, pie_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(area);

    let bloque = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Búsqueda global ")
        .border_style(Style::default().fg(colors.accent))
        .style(Style::default().bg(colors.bg));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(colors.accent)),
            Span::styled(estado.query.clone(), Style::default().fg(colors.fg)),
            Span::styled("▌", Style::default().fg(colors.accent)),
        ]))
        .block(bloque)
        .style(Style::default().bg(colors.bg)),
        input_area,
    );

    if estado.resultados.is_empty() {
        frame.render_widget(
            Paragraph::new(if estado.query.is_empty() {
                "Escribí para buscar partidos, camisetas, canchas, equipos, jugadores, lesiones o finanzas."
            } else {
                "Sin resultados."
            })
            .style(Style::default().fg(colors.muted))
            .wrap(Wrap { trim: true }),
            lista_area,
        );
    } else {
        let alto = lista_area.height as usize;
        let inicio = estado.selected.saturating_sub(alto.saturating_sub(1));
        let lineas: Vec<Line> = estado
            .resultados
            .iter()
            .enumerate()
            .skip(inicio)
            .take(alto)
            .map(|(indice, resultado)| {
                let estilo = if indice == estado.selected {
                    Style::default()
                        .fg(colors.highlight_fg)
                        .bg(colors.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.fg)
                };
                Line::from(vec![
                    Span::styled(format!(" {:<12} ", resultado.tipo.label()), estilo),
                    Span::styled(resultado.titulo.clone(), estilo),
                    Span::styled(
                        format!("  {}", resultado.subtitulo),
                        Style::default().fg(colors.muted),
                    ),
                ])
            })
            .collect();
        frame.render_widget(Paragraph::new(lineas), lista_area);
    }

    frame.render_widget(
        Paragraph::new("Enter: abrir   ↑↓: mover   Esc: cerrar")
            .alignment(Alignment::Right)
            .style(Style::default().fg(colors.muted)),
        pie_area,
    );
}

fn draw_paleta(frame: &mut Frame, area: Rect, estado: &PaletaState, colors: &Palette) {
    let [input_area, lista_area, pie_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(area);

    let bloque = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Paleta de comandos ")
        .border_style(Style::default().fg(colors.accent))
        .style(Style::default().bg(colors.bg));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(colors.accent)),
            Span::styled(estado.query.clone(), Style::default().fg(colors.fg)),
            Span::styled("▌", Style::default().fg(colors.accent)),
        ]))
        .block(bloque)
        .style(Style::default().bg(colors.bg)),
        input_area,
    );

    let acciones = crate::app_inicio::acciones_paleta(estado);
    if acciones.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin acciones que coincidan.").style(Style::default().fg(colors.muted)),
            lista_area,
        );
    } else {
        let lineas: Vec<Line> = acciones
            .iter()
            .enumerate()
            .map(|(indice, (etiqueta, _))| {
                let estilo = if indice == estado.selected {
                    Style::default()
                        .fg(colors.highlight_fg)
                        .bg(colors.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.fg)
                };
                Line::from(Span::styled(format!(" ▸ {etiqueta}"), estilo))
            })
            .collect();
        frame.render_widget(Paragraph::new(lineas), lista_area);
    }

    frame.render_widget(
        Paragraph::new("Enter: ejecutar   ↑↓: mover   Esc: cerrar")
            .alignment(Alignment::Right)
            .style(Style::default().fg(colors.muted)),
        pie_area,
    );
}
