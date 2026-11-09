//! Calendario mensual (IMP-02).

use chrono::{Datelike, Local, NaiveDate};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::app::App;
use crate::domain::calendario::{EventoCalendario, EventoTipo};
use crate::ui::theme::palette;

const MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];
const DIAS: [&str; 7] = ["Lu", "Ma", "Mi", "Ju", "Vi", "Sá", "Do"];

/// Dibuja el calendario.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let [cabecera, cuerpo] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);

    let titulo = format!(
        " {} {} ",
        MESES[(app.calendario.month as usize).saturating_sub(1) % 12],
        app.calendario.year
    );
    let hoy = Local::now().date_naive();
    let seleccionada = app.calendario.seleccionada();
    let cabecera_texto = Line::from(vec![
        Span::styled(
            titulo,
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  seleccionado {} · hoy {} ",
                seleccionada.format("%d/%m/%Y"),
                hoy.format("%d/%m/%Y")
            ),
            Style::default().fg(colors.muted),
        ),
    ]);
    frame.render_widget(Paragraph::new(cabecera_texto), cabecera);

    if area.width >= 110 {
        let [grilla, panel] = Layout::horizontal([Constraint::Percentage(68), Constraint::Min(30)])
            .spacing(1)
            .areas(cuerpo);
        draw_grilla(frame, grilla, app);
        draw_panel(frame, panel, app);
    } else {
        draw_grilla(frame, cuerpo, app);
    }
}

fn draw_grilla(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let primer = app.calendario.primer_dia();
    let offset = primer.weekday().num_days_from_monday();
    let dias = app.calendario.dias_del_mes();
    let hoy = Local::now().date_naive();

    let mut restricciones = vec![Constraint::Length(1)];
    for _ in 0..6 {
        restricciones.push(Constraint::Fill(1));
    }
    let filas = Layout::vertical(restricciones).split(area);

    let etiquetas: Vec<Line> = DIAS
        .iter()
        .map(|dia| {
            Line::from(Span::styled(
                format!(" {dia:^6} "),
                Style::default()
                    .fg(colors.muted)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    let columnas = Layout::horizontal([Constraint::Ratio(1, 7); 7]).split(filas[0]);
    for (indice, etiqueta) in etiquetas.into_iter().enumerate() {
        frame.render_widget(Paragraph::new(etiqueta), columnas[indice]);
    }

    for semana in 0..6 {
        let columnas = Layout::horizontal([Constraint::Ratio(1, 7); 7]).split(filas[semana + 1]);
        for dia_semana in 0..7u32 {
            let indice = semana as u32 * 7 + dia_semana;
            let Some(dia) = indice.checked_sub(offset).map(|d| d + 1) else {
                continue;
            };
            if dia > dias {
                continue;
            }
            let fecha = NaiveDate::from_ymd_opt(app.calendario.year, app.calendario.month, dia)
                .unwrap_or(primer);
            let eventos: Vec<&EventoCalendario> = app
                .calendario
                .eventos
                .iter()
                .filter(|evento| evento.fecha == fecha)
                .collect();
            let mut marcas = String::new();
            if eventos
                .iter()
                .any(|evento| evento.tipo == EventoTipo::Partido)
            {
                marcas.push('P');
            }
            if eventos
                .iter()
                .any(|evento| evento.tipo == EventoTipo::Financiamiento)
            {
                marcas.push('$');
            }
            let seleccionado = fecha == app.calendario.seleccionada();
            let es_hoy = fecha == hoy;
            let estilo = if seleccionado {
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else if es_hoy {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg)
            };
            let texto = if columnas[dia_semana as usize].height <= 1 {
                Line::from(Span::styled(format!(" {dia:>2}{marcas}"), estilo))
            } else {
                Line::from(vec![
                    Span::styled(format!(" {dia:>2} "), estilo),
                    Span::styled(marcas, Style::default().fg(colors.warning)),
                ])
            };
            frame.render_widget(
                Paragraph::new(texto).style(estilo),
                columnas[dia_semana as usize],
            );
        }
    }
}

fn draw_panel(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let fecha = app.calendario.seleccionada();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", fecha.format("%d/%m/%Y")))
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    let interior = block.inner(area);
    frame.render_widget(block, area);

    let eventos = app.calendario.eventos_del_dia();
    if eventos.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "Sin eventos.

c: calendario · Esc: volver",
            )
            .style(Style::default().fg(colors.muted))
            .wrap(Wrap { trim: true }),
            interior,
        );
        return;
    }
    let mut lineas = Vec::new();
    for evento in eventos {
        lineas.push(Line::from(vec![
            Span::styled(
                format!("[{}] ", evento.tipo.label()),
                Style::default().fg(match evento.tipo {
                    EventoTipo::Partido => colors.accent,
                    EventoTipo::Financiamiento => colors.warning,
                    EventoTipo::Recordatorio => colors.muted,
                }),
            ),
            Span::styled(evento.titulo.clone(), Style::default().fg(colors.fg)),
        ]));
        if !evento.detalle.is_empty() {
            lineas.push(Line::from(Span::styled(
                format!("    {}", evento.detalle),
                Style::default().fg(colors.muted),
            )));
        }
        lineas.push(Line::from(""));
    }
    frame.render_widget(Paragraph::new(lineas).wrap(Wrap { trim: false }), interior);
}
