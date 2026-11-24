//! Pantalla de Ajustes (§18.7).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Dibuja el formulario de ajustes con una muestra del tema.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mostrar_preview = app
        .form
        .as_ref()
        .is_some_and(|formulario| formulario.title == "Ajustes");
    let (form_area, preview_area) = if mostrar_preview && area.height > 12 {
        let [formulario, preview] =
            Layout::vertical([Constraint::Min(6), Constraint::Length(3)]).areas(area);
        (formulario, Some(preview))
    } else {
        (area, None)
    };

    if let Some(formulario) = &app.form {
        form::draw(frame, form_area, app, formulario);
    }

    if let Some(preview) = preview_area {
        let chips = vec![
            Span::styled(" Normal ", Style::default().fg(colors.bg).bg(colors.fg)),
            Span::raw(" "),
            Span::styled(
                " Acento ",
                Style::default()
                    .fg(colors.bg)
                    .bg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(" Éxito ", Style::default().fg(colors.bg).bg(colors.success)),
            Span::raw(" "),
            Span::styled(" Aviso ", Style::default().fg(colors.bg).bg(colors.warning)),
            Span::raw(" "),
            Span::styled(" Error ", Style::default().fg(colors.bg).bg(colors.error)),
            Span::raw(" "),
            Span::styled(
                " Selección ",
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(colors.highlight_bg),
            ),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(chips)).block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Vista previa del tema ")
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            ),
            preview,
        );
    }
}

/// Administración de backups (`menu_backup_restore` de MiFutbolC).
pub fn draw_backups(frame: &mut Frame, area: Rect, app: &mut App) {
    use crate::services::backup_service::tamano;
    use ratatui::widgets::{Cell, Row, Table};

    let colors = palette(app.settings.theme);
    if app.backups.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay backups en Exportaciones/. Presioná b para crear uno.")
                .block(colors.panel(" Backups "))
                .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .backups
        .iter()
        .map(|b| {
            Row::new(vec![
                Cell::from(b.nombre.clone()),
                Cell::from(b.fecha.clone()),
                Cell::from(tamano(b.bytes)),
                Cell::from(
                    b.partidos
                        .map_or_else(|| "?".to_string(), |n| n.to_string()),
                ),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Min(30),
            Constraint::Length(17),
            Constraint::Length(10),
            Constraint::Length(9),
        ],
    )
    .header(
        Row::new(vec!["Archivo", "Fecha", "Tamaño", "Partidos"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(colors.panel(format!(" Backups · {} ", app.backups.len())))
    .style(Style::default().fg(colors.fg))
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.backups_table);
}

/// Exportar / Importar (`menu_exportar` y `menu_importar` de MiFutbolC).
pub fn draw_exportar(frame: &mut Frame, area: Rect, app: &mut App) {
    use crate::app_exportar::EXPORT_OPCIONES;
    use ratatui::widgets::{List, ListItem};

    let colors = palette(app.settings.theme);
    let items: Vec<ListItem> = EXPORT_OPCIONES
        .iter()
        .enumerate()
        .map(|(indice, (titulo, descripcion))| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{:>2} {titulo}", indice + 1),
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("    {descripcion}"),
                    Style::default().fg(colors.muted),
                )),
            ])
        })
        .collect();
    let lista = List::new(items)
        .block(colors.panel(" Exportar / Importar "))
        .highlight_style(
            Style::default()
                .bg(colors.highlight_bg)
                .fg(colors.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(lista, area, &mut app.export_hub);
}

/// Editor de menús personalizados: `[X]` visible, `[ ]` oculto en Personalizado.
pub fn draw_menus(frame: &mut Frame, area: Rect, app: &mut App) {
    use ratatui::widgets::{List, ListItem};

    let colors = palette(app.settings.theme);
    let items: Vec<ListItem> = App::menus_editables()
        .into_iter()
        .map(|module| {
            let visible = !app.menu_ocultos.iter().any(|clave| clave == module.key());
            ListItem::new(Line::from(vec![
                Span::styled(
                    if visible { "[X] " } else { "[ ] " },
                    Style::default()
                        .fg(if visible {
                            colors.success
                        } else {
                            colors.muted
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "{:>2} {}",
                        module.numero(),
                        module.etiqueta(app.settings.language)
                    ),
                    Style::default().fg(colors.fg),
                ),
            ]))
        })
        .collect();
    let lista = List::new(items)
        .block(colors.panel(format!(
            " Menús personalizados · se aplican en modo Personalizado (ahora: {}) · Ajustes siempre visible ",
            app.settings.mode.label()
        )))
        .highlight_style(
            Style::default()
                .bg(colors.highlight_bg)
                .fg(colors.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(lista, area, &mut app.menus_hub);
}
