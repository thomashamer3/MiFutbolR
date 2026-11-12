//! Flujos de Canchas y Botines por la TUI.

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

mod common;
use common::*;

fn open_module(app: &mut App, downs: usize) {
    for _ in 0..downs {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

fn count(app: &App, table: &str) -> i64 {
    app.db
        .conn()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn create_and_delete_cancha_through_the_ui() {
    let (mut app, _dir) = test_app();
    open_module(&mut app, 1); // Camisetas -> Canchas
    assert_eq!(app.current_screen(), Screen::CanchasList);

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::CanchaForm(_)));
    type_text(&mut app, "Complejo Sur");
    press_ctrl(&mut app, 's');

    assert_eq!(app.current_screen(), Screen::CanchasList);
    assert_eq!(count(&app, "cancha"), 1);
    let text = render_con(&mut app, 120, 34);
    assert!(
        text.contains("Complejo Sur"),
        "la cancha no aparece: {text}"
    );

    // Desactivar y reactivar con 'a'.
    press(&mut app, KeyCode::Char('a'));
    let activa: i64 = app
        .db
        .conn()
        .query_row("SELECT activa FROM cancha LIMIT 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(activa, 0);
    press(&mut app, KeyCode::Char('a'));

    // Eliminar con confirmación.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(count(&app, "cancha"), 0);
}

#[test]
fn cancha_search_and_detail() {
    let (mut app, _dir) = test_app();
    open_module(&mut app, 1);
    for nombre in ["Sur", "Norte"] {
        press(&mut app, KeyCode::Char('n'));
        type_text(&mut app, nombre);
        press_ctrl(&mut app, 's');
    }
    assert_eq!(app.canchas.len(), 2);

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "nor");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.canchas.len(), 1);
    assert_eq!(app.canchas[0].nombre, "Norte");

    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::CanchaDetail(_)));
    let text = render_con(&mut app, 120, 34);
    assert!(text.contains("Norte"));
    assert!(text.contains("Estadísticas"));
}

#[test]
fn create_set_default_and_draw_botin() {
    let (mut app, _dir) = test_app();
    open_module(&mut app, 2); // Camisetas -> Canchas -> Botines
    assert_eq!(app.current_screen(), Screen::BotinesList);

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::BotinForm(_)));
    type_text(&mut app, "Predator");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::BotinesList);
    assert_eq!(count(&app, "botin"), 1);

    // Fijar predeterminado.
    press(&mut app, KeyCode::Char('p'));
    let predeterminado: i64 = app
        .db
        .conn()
        .query_row(
            "SELECT botin_predeterminado FROM settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(predeterminado > 0);

    // Sortear.
    press(&mut app, KeyCode::Char('s'));
    assert!(matches!(app.modal, Some(mifutbol_rs::app::Modal::Info(_))));
    let sorteada: i64 = app
        .db
        .conn()
        .query_row("SELECT sorteada FROM botin LIMIT 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(sorteada, 1);
    press(&mut app, KeyCode::Enter); // cierra el modal

    // El listado muestra la estrella del predeterminado.
    let text = render_con(&mut app, 120, 34);
    assert!(text.contains("★"));
    assert!(text.contains("Predator"));
}
