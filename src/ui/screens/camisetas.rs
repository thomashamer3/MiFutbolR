//! Pantallas del módulo Camisetas: lista, detalle y formulario (§8.5).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, HighlightSpacing, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::domain::models::Camiseta;
use crate::ui::theme::palette;
use crate::ui::widgets::{detail, form};

/// Listado con agregados y vista previa lateral.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let title = format!(
        " Camisetas · {} · {} ",
        app.camiseta_filter.estado.label(),
        app.camiseta_filter.orden.label()
    );

    let (tabla_area, preview_area) = match detail::split(area) {
        Some((tabla, panel)) => (tabla, Some(panel)),
        None => (area, None),
    };

    if app.camisetas.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay camisetas registradas.\n\nn: crear una nueva      /: buscar      Esc: volver",
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
        if let Some(panel) = preview_area {
            detail::panel(
                frame,
                panel,
                &colors,
                "Vista previa",
                vec![Line::from(Span::styled(
                    "Sin selección",
                    Style::default().fg(colors.muted),
                ))],
            );
        }
        return;
    }

    let header = Row::new(vec![
        "ID",
        "Nombre",
        "PARTIDOS",
        "GOLES",
        "ASISTENCIAS",
        "VICTORIAS",
        "EMPATES",
        "DERROTAS",
        "Estado",
    ])
    .style(
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    );

    let rows = app.camisetas.iter().map(|row| {
        let row_style = if row.activa {
            Style::default().fg(colors.fg)
        } else {
            Style::default().fg(colors.muted)
        };
        Row::new(vec![
            Cell::from(row.id.to_string()),
            Cell::from(row.nombre.clone()),
            Cell::from(row.partidos.to_string()),
            Cell::from(row.goles.to_string()),
            Cell::from(row.asistencias.to_string()),
            Cell::from(row.victorias.to_string()),
            Cell::from(row.empates.to_string()),
            Cell::from(row.derrotas.to_string()),
            Cell::from(if row.activa { "Activa" } else { "INACTIVA" }),
        ])
        .style(row_style)
    });

    let widths = [
        Constraint::Length(5),
        Constraint::Min(18),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(9),
    ];

    let table = Table::new(rows, widths)
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

    frame.render_stateful_widget(table, tabla_area, &mut app.camisetas_table);

    if let Some(panel) = preview_area {
        let seleccionada = app
            .camisetas_table
            .selected()
            .and_then(|indice| app.camisetas.get(indice));
        let mut lineas = Vec::new();
        if let Some(row) = seleccionada {
            let titulo = Line::from(Span::styled(
                row.nombre.clone(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lineas.push(titulo);
            lineas.push(Line::from(""));
            let mut push = |etiqueta: &str, valor: String| {
                lineas.push(Line::from(vec![
                    Span::styled(format!("{etiqueta:<14}"), Style::default().fg(colors.muted)),
                    Span::styled(valor, Style::default().fg(colors.fg)),
                ]));
            };
            push("ID", row.id.to_string());
            push("Partidos", row.partidos.to_string());
            push("Goles", row.goles.to_string());
            push("Asistencias", row.asistencias.to_string());
            push(
                "V / E / D",
                format!("{} / {} / {}", row.victorias, row.empates, row.derrotas),
            );
            push(
                "Estado",
                if row.activa { "Activa" } else { "INACTIVA" }.to_string(),
            );
            lineas.push(Line::from(""));
            lineas.push(Line::from(Span::styled(
                "Enter: ficha completa   e: editar   d: eliminar",
                Style::default().fg(colors.muted),
            )));
        }
        detail::panel(frame, panel, &colors, "Vista previa", lineas);
    }
}

/// Detalle de una camiseta con estadísticas derivadas.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &App, id: i64) {
    let colors = palette(app.settings.theme);
    let Some(camiseta) = &app.camiseta_detail else {
        let paragraph = Paragraph::new(format!("Cargando camiseta {id}…"))
            .block(Block::bordered().title(" Camiseta "))
            .style(Style::default().fg(colors.fg));
        frame.render_widget(paragraph, area);
        return;
    };

    let mut lines = Vec::new();
    let mut push = |label: &str, value: &str| {
        if !value.trim().is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("{label:<18}"), Style::default().fg(colors.muted)),
                Span::styled(value.to_string(), Style::default().fg(colors.fg)),
            ]));
        }
    };
    push("Nombre", &camiseta.nombre);
    push("Marca", &camiseta.marca);
    push("Modelo", &camiseta.modelo);
    push("Temporada", &camiseta.temporada);
    push("Color principal", &camiseta.color_principal);
    push("Color secundario", &camiseta.color_secundario);
    push("Estado físico", camiseta.estado_fisico.label());
    if let Some(fecha) = camiseta.fecha_compra {
        push(
            "Fecha de compra",
            &crate::domain::dates::display_date(fecha),
        );
    }
    push("Costo", &camiseta.costo.display_ars());
    push("Proveedor", &camiseta.proveedor);
    if camiseta.fue_regalo {
        push("Regalo de", &camiseta.regalo_de);
    }
    push("Imagen", &camiseta.imagen_ruta);
    push("Observaciones", &camiseta.observaciones);
    push(
        "Estado",
        if camiseta.activa {
            "Activa"
        } else {
            "INACTIVA"
        },
    );
    push("Sorteada", if camiseta.sorteada { "Sí" } else { "No" });

    let ficha = app.camiseta_ficha.unwrap_or_default();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Estadísticas",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled(
            format!("{:<18}", "Partidos"),
            Style::default().fg(colors.muted),
        ),
        Span::styled(ficha.partidos.to_string(), Style::default().fg(colors.fg)),
        Span::styled(
            format!("{:<12}", "  Goles"),
            Style::default().fg(colors.muted),
        ),
        Span::styled(ficha.goles.to_string(), Style::default().fg(colors.fg)),
        Span::styled(
            format!("{:<16}", "  Asistencias"),
            Style::default().fg(colors.muted),
        ),
        Span::styled(
            ficha.asistencias.to_string(),
            Style::default().fg(colors.fg),
        ),
        Span::styled(
            format!("{:<12}", "  Lesiones"),
            Style::default().fg(colors.muted),
        ),
        Span::styled(ficha.lesiones.to_string(), Style::default().fg(colors.fg)),
    ]));

    let block = Block::bordered()
        .title(format!(" Camiseta · {} ", camiseta.nombre))
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    let [_, body] = Layout::vertical([Constraint::Length(0), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        body,
    );
}

/// Formulario de alta/edición.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(form) = &app.form {
        form::draw(frame, area, app, form);
    }
}

/// `true` si la camiseta coincide con el filtro de texto actual.
#[must_use]
pub fn coincide(camiseta: &Camiseta, texto: &str) -> bool {
    texto.trim().is_empty()
        || crate::util::text::normalize(&camiseta.nombre)
            .contains(&crate::util::text::normalize(texto))
}
