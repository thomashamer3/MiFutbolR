//! Flujo del módulo Carrera por la TUI (§16.7).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

mod common;
use common::*;

fn open_carrera(app: &mut App) {
    for _ in 0..8 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

#[test]
fn hub_resumen_and_identity_form() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    open_carrera(&mut app);
    assert_eq!(app.current_screen(), Screen::CarreraMenu);

    let text = render_con(&mut app, 120, 36);
    assert!(text.contains("Resumen"));
    assert!(text.contains("Hall of Fame"));

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraResumen);
    let text = render_con(&mut app, 120, 36);
    assert!(text.contains("Partidos"));
    assert!(text.contains("Mejor racha"));
    press(&mut app, KeyCode::Esc);

    // Ficha de identidad.
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraIdentidadForm);
    type_text(&mut app, "Jugador");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::CarreraMenu);
    let nombre: String = app
        .db
        .conn()
        .query_row(
            "SELECT nombre FROM carrera_identidad WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(nombre, "Jugador");
}

#[test]
fn create_and_delete_hito() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    open_carrera(&mut app);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraHitos);

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::CarreraHitoForm(_)));
    press(&mut app, KeyCode::Tab); // del partido al tipo
    type_text(&mut app, "Primer gol");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::CarreraHitos);
    let hitos: i64 = app
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM carrera_partido_hito", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(hitos, 1);

    let text = render_con(&mut app, 120, 36);
    assert!(text.contains("Primer gol"));

    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    let hitos: i64 = app
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM carrera_partido_hito", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(hitos, 0);
}

#[test]
fn timeline_and_logros_render() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    open_carrera(&mut app);
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter); // Timeline
    assert_eq!(app.current_screen(), Screen::CarreraTimeline);
    let text = render_con(&mut app, 120, 36);
    assert!(text.contains("timeline") || text.contains("Timeline"));
    assert!(text.contains("Partido"));
    press(&mut app, KeyCode::Esc);

    for _ in 0..4 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter); // Logros
    assert_eq!(app.current_screen(), Screen::CarreraLogros);
    let text = render_con(&mut app, 120, 36);
    assert!(text.contains("Primer partido"));
    assert!(text.contains("Hat-trick"));
}

#[test]
fn perfil_y_modo_retro_render() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    app.menu_state.select(Some(8));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraMenu);

    app.carrera_hub.select(Some(5));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraPerfil);
    let text = render_con(&mut app, 120, 36);
    assert!(text.contains("Etiqueta"));
    press(&mut app, KeyCode::Esc);

    app.carrera_hub.select(Some(6));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraRetro);
    let text = render_con(&mut app, 120, 36);
    assert!(text.contains("Modo retro") || text.contains("Sin recuerdos"));
}
