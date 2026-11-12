//! Flujo completo del módulo Equipos por la TUI (§11.5).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

mod common;
use common::*;

fn count(app: &App, sql: &str) -> i64 {
    app.db.conn().query_row(sql, [], |row| row.get(0)).unwrap()
}

#[test]
fn create_team_add_players_and_delete_one() {
    let (mut app, _dir) = test_app();
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::EquiposList);

    // Alta del equipo con los valores por defecto (Fijo, F5, 5 jugadores).
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::EquipoForm(_)));
    type_text(&mut app, "Los Pibes");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::EquiposList);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM equipo"), 1);

    // Detalle.
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::EquipoDetail(_)));
    let text = render_con(&mut app, 120, 34);
    assert!(text.contains("Los Pibes"));
    assert!(text.contains("Sin jugadores"));

    // Primer jugador: nombre + dorsal.
    press(&mut app, KeyCode::Char('a'));
    assert!(matches!(app.current_screen(), Screen::JugadorForm(_)));
    type_text(&mut app, "Uno");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "10");
    press_ctrl(&mut app, 's');
    assert!(matches!(app.current_screen(), Screen::EquipoDetail(_)));
    assert_eq!(count(&app, "SELECT COUNT(*) FROM jugador"), 1);

    // Segundo jugador, marcado como capitán.
    press(&mut app, KeyCode::Char('a'));
    type_text(&mut app, "Dos");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "7");
    press(&mut app, KeyCode::Tab); // posición
    press(&mut app, KeyCode::Tab); // capitán
    press(&mut app, KeyCode::Char(' '));
    press_ctrl(&mut app, 's');
    assert_eq!(count(&app, "SELECT COUNT(*) FROM jugador"), 2);
    assert_eq!(
        count(&app, "SELECT COUNT(*) FROM jugador WHERE es_capitan = 1"),
        1
    );

    let text = render_con(&mut app, 120, 34);
    assert!(text.contains("Dos"));
    assert!(text.contains("★"));

    // Borrar el jugador seleccionado.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(count(&app, "SELECT COUNT(*) FROM jugador"), 1);
}

#[test]
fn duplicate_dorsal_is_reported_and_form_stays() {
    let (mut app, _dir) = test_app();
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Equipo");
    press_ctrl(&mut app, 's');
    press(&mut app, KeyCode::Enter);

    for (nombre, dorsal) in [("Uno", "5"), ("Dos", "5")] {
        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, nombre);
        press(&mut app, KeyCode::Tab);
        type_text(&mut app, dorsal);
        press_ctrl(&mut app, 's');
        if nombre == "Dos" {
            assert!(matches!(app.current_screen(), Screen::JugadorForm(_)));
            assert!(app.modal.is_some());
            assert_eq!(count(&app, "SELECT COUNT(*) FROM jugador"), 1);
        }
    }
}

#[test]
fn team_search_and_toggle_active() {
    let (mut app, _dir) = test_app();
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter);
    for nombre in ["Alfa", "Beta"] {
        press(&mut app, KeyCode::Char('n'));
        type_text(&mut app, nombre);
        press_ctrl(&mut app, 's');
    }
    assert_eq!(app.equipos.len(), 2);

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "bet");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.equipos.len(), 1);
    assert_eq!(app.equipos[0].nombre, "Beta");

    press(&mut app, KeyCode::Char('a'));
    let activa: i64 = app
        .db
        .conn()
        .query_row(
            "SELECT activa FROM equipo WHERE nombre = 'Beta'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(activa, 0);
}
