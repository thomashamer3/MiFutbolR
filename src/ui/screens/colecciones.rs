//! Colecciones e inventario (paridad con MiFutbolC).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, TableState, Tabs};

use crate::app::App;
use crate::domain::colecciones::{ESTADOS_ITEM, InventarioItem, TIPOS_ITEM, etiqueta};
use crate::domain::dates;
use crate::ui::theme::{Palette, palette};
use crate::ui::widgets::{form, lista};

fn tabla_items(
    frame: &mut Frame,
    area: Rect,
    colors: &Palette,
    titulo: String,
    items: &[InventarioItem],
    state: &mut TableState,
) {
    let filas = items
        .iter()
        .map(|item| {
            Row::new(vec![
                Cell::from(item.id.to_string()),
                Cell::from(etiqueta(&TIPOS_ITEM, item.tipo)),
                Cell::from(item.nombre_visible.clone()),
                Cell::from(etiqueta(&ESTADOS_ITEM, item.estado)),
                Cell::from(format!("{:.2}", item.valor)),
                Cell::from(
                    dates::parse_date(&item.fecha_compra)
                        .map_or_else(|| "—".to_string(), dates::display_date),
                ),
                Cell::from(item.colecciones.to_string()),
            ])
        })
        .collect();
    lista::render_tabla(
        frame,
        area,
        colors,
        titulo,
        vec!["ID", "Tipo", "Nombre", "Estado", "Valor", "Compra", "Col."],
        vec![
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Min(20),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(11),
            Constraint::Length(5),
        ],
        filas,
        state,
    );
}

/// Listado con pestañas Inventario / Colecciones.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let [pestanas, cuerpo] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).areas(area);
    frame.render_widget(
        Tabs::new(["Inventario", "Colecciones"])
            .select(usize::from(app.colecciones_ver_colecciones))
            .style(Style::default().fg(colors.muted))
            .highlight_style(
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        pestanas,
    );
    if app.colecciones_ver_colecciones {
        let filas = app
            .colecciones
            .iter()
            .map(|coleccion| {
                Row::new(vec![
                    Cell::from(coleccion.id.to_string()),
                    Cell::from(coleccion.nombre.clone()),
                    Cell::from(coleccion.items.to_string()),
                    Cell::from(coleccion.descripcion.clone()),
                ])
            })
            .collect();
        lista::render_tabla(
            frame,
            cuerpo,
            &colors,
            format!(" Colecciones · {} ", app.colecciones.len()),
            vec!["ID", "Nombre", "Ítems", "Descripción"],
            vec![
                Constraint::Length(4),
                Constraint::Min(20),
                Constraint::Length(6),
                Constraint::Min(24),
            ],
            filas,
            &mut app.colecciones_table,
        );
    } else if app.inventario.is_empty() {
        lista::render_vacio(
            frame,
            cuerpo,
            &colors,
            " Inventario ",
            "El inventario está vacío. Presioná n para agregar un ítem o s para sincronizar camisetas.",
        );
    } else {
        tabla_items(
            frame,
            cuerpo,
            &colors,
            format!(" Inventario · {} ", app.inventario.len()),
            &app.inventario,
            &mut app.inventario_table,
        );
    }
}

/// Ítems de una colección.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &mut App, id: i64) {
    let colors = palette(app.settings.theme);
    let nombre = app
        .colecciones
        .iter()
        .find(|coleccion| coleccion.id == id)
        .map_or("Colección", |coleccion| coleccion.nombre.as_str())
        .to_string();
    if app.coleccion_detalle.is_empty() {
        lista::render_vacio(
            frame,
            area,
            &colors,
            format!(" {nombre} "),
            "Esta colección está vacía. Agregá ítems desde el inventario con a.",
        );
        return;
    }
    tabla_items(
        frame,
        area,
        &colors,
        format!(" {nombre} · {} ", app.coleccion_detalle.len()),
        &app.coleccion_detalle,
        &mut app.coleccion_detalle_table,
    );
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
