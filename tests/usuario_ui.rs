//! Portada de inicio de sesión, registro y bases de datos por cuenta.

mod common;

use common::*;
use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

#[test]
fn welcome_creates_account_and_opens_its_own_db() {
    let (mut app, _dir) = test_app_con_bienvenida();
    assert_eq!(app.current_screen(), Screen::Bienvenida);
    assert!(app.usuario_actual.is_none());
    let texto = render_con(&mut app, 100, 30);
    assert!(texto.contains("Todavía no hay cuentas"), "{texto}");

    crear_cuenta(&mut app, "Ana", "");

    assert_eq!(app.current_screen(), Screen::MainMenu);
    let ana = app.usuario_actual.clone().expect("sesión");
    assert_eq!(ana.nombre, "Ana");
    assert_eq!(app.db.path(), Some(app.paths.db_usuario(ana.id).as_path()));
}

#[test]
fn login_rejects_wrong_password() {
    let (mut app, _dir) = test_app_con_bienvenida();
    crear_cuenta(&mut app, "Beto", "clave");
    cerrar_sesion(&mut app);

    // Beto tiene contraseña: Enter abre el login con el nombre precargado.
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::UsuarioLogin);
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "mala");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::UsuarioLogin);
    assert!(app.modal.is_some());
    assert!(app.usuario_actual.is_none());
}

#[test]
fn each_account_keeps_its_own_data() {
    let (mut app, _dir) = test_app_con_bienvenida();
    crear_cuenta(&mut app, "Ana", "");
    let base = canchas(&app);
    app.db
        .conn()
        .execute("INSERT INTO cancha (nombre) VALUES ('Solo de Ana')", [])
        .unwrap();
    assert_eq!(canchas(&app), base + 1);

    cerrar_sesion(&mut app);
    crear_cuenta(&mut app, "Beto", "");
    assert_eq!(app.usuario_actual.as_ref().unwrap().nombre, "Beto");
    assert_eq!(canchas(&app), base);

    cerrar_sesion(&mut app);
    // Las cuentas se listan por nombre: Ana es la primera.
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert_eq!(app.usuario_actual.as_ref().unwrap().nombre, "Ana");
    assert_eq!(canchas(&app), base + 1);
}

#[test]
fn portada_renders_at_any_size() {
    let (mut app, _dir) = test_app_con_bienvenida();
    for (ancho, alto) in [(40, 12), (80, 24), (120, 40)] {
        render_con(&mut app, ancho, alto);
    }
    crear_cuenta(&mut app, "Ana", "");
    cerrar_sesion(&mut app);
    for (ancho, alto) in [(40, 12), (80, 24), (120, 40)] {
        let texto = render_con(&mut app, ancho, alto);
        assert!(texto.contains("Ana"), "{ancho}x{alto}: {texto}");
    }
    press(&mut app, KeyCode::Char('n'));
    for (ancho, alto) in [(40, 12), (80, 24), (120, 40)] {
        render_con(&mut app, ancho, alto);
    }
}

fn crear_cuenta(app: &mut App, nombre: &str, password: &str) {
    press(app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::UsuarioForm(_)));
    type_text(app, nombre);
    press(app, KeyCode::Tab);
    type_text(app, password);
    press(app, KeyCode::Tab);
    type_text(app, password);
    press_ctrl(app, 's');
    assert!(app.modal.is_none(), "{:?}", app.modal);
}

fn cerrar_sesion(app: &mut App) {
    abrir_usuario(app);
    press(app, KeyCode::Char('c'));
    assert_eq!(app.current_screen(), Screen::Bienvenida);
    assert!(app.usuario_actual.is_none());
}

fn canchas(app: &App) -> i64 {
    app.db
        .conn()
        .query_row("SELECT COUNT(*) FROM cancha", [], |row| row.get(0))
        .unwrap()
}

fn abrir_usuario(app: &mut App) {
    let actual = app.menu_state.selected().unwrap_or(0);
    let pasos = (18 + 19 - actual) % 19;
    for _ in 0..pasos {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::UsuarioList);
}
