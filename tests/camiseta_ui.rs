//! Flujo completo del módulo Camisetas por la TUI (§22.2).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

mod common;
use common::*;

fn count_rows(app: &App) -> i64 {
    app.db
        .conn()
        .query_row("SELECT COUNT(*) FROM camiseta", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn create_camiseta_through_the_ui() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, mifutbol_rs::app::Module::Camisetas);
    assert_eq!(app.current_screen(), Screen::CamisetasList);

    press(&mut app, KeyCode::Char('n'));
    assert_eq!(
        app.current_screen(),
        Screen::CamisetaForm(mifutbol_rs::domain::models::FormMode::Create)
    );

    type_text(&mut app, "Titular 10");
    press_ctrl(&mut app, 's');

    assert_eq!(app.current_screen(), Screen::CamisetasList);
    assert_eq!(count_rows(&app), 1);
    let text = render_con(&mut app, 110, 32);
    assert!(text.contains("Titular 10"), "la fila no aparece: {text}");
}

#[test]
fn empty_name_is_rejected_and_form_stays_open() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, mifutbol_rs::app::Module::Camisetas);
    press(&mut app, KeyCode::Char('n'));
    press_ctrl(&mut app, 's');

    assert!(matches!(app.current_screen(), Screen::CamisetaForm(_)));
    assert!(app.modal.is_some());
    assert_eq!(count_rows(&app), 0);

    // Cerrar el modal de error y volver.
    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_none());
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::CamisetasList);
}

#[test]
fn edit_and_delete_through_the_ui() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, mifutbol_rs::app::Module::Camisetas);
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Vieja");
    press_ctrl(&mut app, 's');
    assert_eq!(count_rows(&app), 1);

    // Editar: el formulario conserva el nombre; agregamos un sufijo.
    press(&mut app, KeyCode::Char('e'));
    assert!(matches!(
        app.current_screen(),
        Screen::CamisetaForm(mifutbol_rs::domain::models::FormMode::Edit)
    ));
    type_text(&mut app, " Nueva");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::CamisetasList);
    let name: String = app
        .db
        .conn()
        .query_row("SELECT nombre FROM camiseta LIMIT 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(name, "Vieja Nueva");

    // Eliminar con confirmación.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(count_rows(&app), 0);
    assert_eq!(app.current_screen(), Screen::CamisetasList);
}

#[test]
fn search_filters_the_list() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, mifutbol_rs::app::Module::Camisetas);
    for name in ["Roja", "Azul"] {
        press(&mut app, KeyCode::Char('n'));
        type_text(&mut app, name);
        press_ctrl(&mut app, 's');
    }
    assert_eq!(app.camisetas.len(), 2);

    press(&mut app, KeyCode::Char('/'));
    assert!(app.search.is_some());
    type_text(&mut app, "roj");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.camisetas.len(), 1);
    assert_eq!(app.camisetas[0].nombre, "Roja");

    // Limpiar el filtro volviendo a buscar vacío.
    press(&mut app, KeyCode::Char('/'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.camisetas.len(), 2);
}

#[test]
fn draw_awards_a_jersey_and_marks_it() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, mifutbol_rs::app::Module::Camisetas);
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Sorteable");
    press_ctrl(&mut app, 's');

    press(&mut app, KeyCode::Char('s'));
    assert!(matches!(app.modal, Some(mifutbol_rs::app::Modal::Info(_))));
    let sorteada: i64 = app
        .db
        .conn()
        .query_row("SELECT sorteada FROM camiseta LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(sorteada, 1);
}

#[test]
fn toggle_reactivates_jersey_from_list_and_detail() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, mifutbol_rs::app::Module::Camisetas);
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Vuelve");
    press_ctrl(&mut app, 's');
    let activa = |app: &App| -> i64 {
        app.db
            .conn()
            .query_row("SELECT activa FROM camiseta LIMIT 1", [], |row| row.get(0))
            .unwrap()
    };

    press(&mut app, KeyCode::Char('a'));
    assert_eq!(activa(&app), 0);
    let text = render_con(&mut app, 120, 32);
    assert!(text.contains("INACTIVA"), "falta el indicador: {text}");

    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::CamisetaDetail(_)));
    press(&mut app, KeyCode::Char('a'));
    assert_eq!(activa(&app), 1);
    assert!(app.camiseta_detail.as_ref().unwrap().activa);
}
