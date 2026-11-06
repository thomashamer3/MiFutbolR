//! Pantallas del módulo Canchas (§9.5).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, HighlightSpacing, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::{detail, form};

/// Listado de canchas con vista previa lateral.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let title = format!(
        " Canchas · {} · {} ",
        app.cancha_filter.estado.label(),
        app.cancha_filter.orden.label()
    );

    let (tabla_area, preview_area) = match detail::split(area) {
        Some((tabla, panel)) => (tabla, Some(panel)),
        None => (area, None),
    };

    if app.canchas.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay canchas registradas.\n\nn: crear una nueva      /: buscar      Esc: volver",
            )
            .block(
                Block::bordered()
                    .title(title)
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(colors.fg)),
            tabla_area,
        );
        return;
    }

    let header = Row::new(vec!["ID", "Nombre", "Localidad", "P", "Rend.", "Estado"]).style(
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    );
    let rows = app.canchas.iter().map(|row| {
        let style = if row.activa {
            Style::default().fg(colors.fg)
        } else {
            Style::default().fg(colors.muted)
        };
        Row::new(vec![
            Cell::from(row.id.to_string()),
            Cell::from(row.nombre.clone()),
            Cell::from(row.localidad.clone()),
            Cell::from(row.partidos.to_string()),
            Cell::from(format!("{:.1}", row.rendimiento_promedio)),
            Cell::from(if row.activa { "Activa" } else { "INACTIVA" }),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Min(18),
            Constraint::Min(12),
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Length(9),
        ],
    )
    .header(header)
    .block(
        Block::bordered()
            .title(title)
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

    frame.render_stateful_widget(table, tabla_area, &mut app.canchas_table);

    if let Some(panel) = preview_area {
        let seleccionada = app
            .canchas_table
            .selected()
            .and_then(|indice| app.canchas.get(indice));
        let mut lineas = Vec::new();
        if let Some(row) = seleccionada {
            lineas.push(Line::from(Span::styled(
                row.nombre.clone(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lineas.push(Line::from(""));
            for (etiqueta, valor) in [
                ("ID", row.id.to_string()),
                ("Localidad", row.localidad.clone()),
                ("Partidos", row.partidos.to_string()),
                ("Goles", row.goles.to_string()),
                ("Asistencias", row.asistencias.to_string()),
                (
                    "V / E / D",
                    format!("{} / {} / {}", row.victorias, row.empates, row.derrotas),
                ),
                ("Rendimiento", format!("{:.2}", row.rendimiento_promedio)),
                (
                    "Estado",
                    if row.activa { "Activa" } else { "INACTIVA" }.to_string(),
                ),
            ] {
                lineas.push(Line::from(vec![
                    Span::styled(format!("{etiqueta:<14}"), Style::default().fg(colors.muted)),
                    Span::styled(valor, Style::default().fg(colors.fg)),
                ]));
            }
            lineas.push(Line::from(""));
            lineas.push(Line::from(Span::styled(
                "Enter: detalle   e: editar   d: eliminar",
                Style::default().fg(colors.muted),
            )));
        }
        detail::panel(frame, panel, &colors, "Vista previa", lineas);
    }
}

/// Detalle de una cancha.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &App, id: i64) {
    let colors = palette(app.settings.theme);
    let Some(cancha) = &app.cancha_detail else {
        frame.render_widget(
            Paragraph::new(format!("Cargando cancha {id}…"))
                .block(Block::bordered().title(" Cancha "))
                .style(Style::default().fg(colors.fg)),
            area,
        );
        return;
    };

    let mut lines = Vec::new();
    let mut push = |label: &str, value: &str| {
        if !value.trim().is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("{label:<22}"), Style::default().fg(colors.muted)),
                Span::styled(value.to_string(), Style::default().fg(colors.fg)),
            ]));
        }
    };
    push("Nombre", &cancha.nombre);
    push("Teléfono", &cancha.telefono);
    push("Dirección", &cancha.direccion);
    push("Localidad", &cancha.localidad);
    push(
        "Tipos",
        &cancha
            .tipos
            .iter()
            .map(|tipo| tipo.label())
            .collect::<Vec<_>>()
            .join(", "),
    );
    push(
        "Superficies",
        &cancha
            .superficies
            .iter()
            .map(|superficie| superficie.label())
            .collect::<Vec<_>>()
            .join(", "),
    );
    push("Techada", cancha.techada.label());
    push("Iluminación", yes_no(cancha.tiene_iluminacion));
    push("Apertura", &horario(cancha.horario_apertura_min));
    push("Cierre", &horario(cancha.horario_cierre_min));
    push("Precio día", &cancha.precio_dia.display_ars());
    push("Precio noche", &cancha.precio_noche.display_ars());
    push("Vestuarios", yes_no(cancha.servicios.vestuarios));
    push("Duchas", yes_no(cancha.servicios.duchas));
    push("Buffet", yes_no(cancha.servicios.buffet));
    push("Estacionamiento", yes_no(cancha.servicios.estacionamiento));
    push("Cantidad", &cancha.cantidad_canchas.to_string());
    push("Estado", &cancha.estado);
    push("Descripción", &cancha.descripcion);
    push("Contacto alternativo", &cancha.contacto_alt);
    if let (Some(lat), Some(lon)) = (cancha.latitud, cancha.longitud) {
        push("Coordenadas", &format!("{lat:.5}, {lon:.5}"));
    }
    push("Grabación", yes_no(cancha.tiene_grabacion));
    push("Marcador", cancha.tiene_marcador.label());
    push("Imagen", &cancha.imagen_ruta);
    push("Activa", yes_no(cancha.activa));

    if let Some(row) = app.canchas.iter().find(|row| row.id == cancha.id) {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Estadísticas",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<16}", "Partidos"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(row.partidos.to_string(), Style::default().fg(colors.fg)),
            Span::styled(
                format!("{:<12}", "  Goles"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(row.goles.to_string(), Style::default().fg(colors.fg)),
            Span::styled(
                format!("{:<16}", "  Victorias"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(row.victorias.to_string(), Style::default().fg(colors.fg)),
            Span::styled(
                format!("{:<18}", "  Rendimiento"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                format!("{:.1}", row.rendimiento_promedio),
                Style::default().fg(colors.fg),
            ),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title(format!(" Cancha · {} ", cancha.nombre))
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        area,
    );
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(form) = &app.form {
        form::draw(frame, area, app, form);
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "✓ Sí" } else { "✗ No" }
}

fn horario(minutos: Option<i64>) -> String {
    match minutos {
        Some(minutos) => format!("{:02}:{:02}", minutos / 60, minutos % 60),
        None => "No definido".to_string(),
    }
}
