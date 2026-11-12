//! Flujo de bienvenida, creación de cuenta e inicio de sesión.

mod common;

use common::*;
use crossterm::event::KeyCode;
use mifutbol_rs::app::Screen;

#[test]
fn welcome_creates_account_and_opens_menu() {
    let (mut app, _dir) = test_app_con_bienvenida();
    assert_eq!(app.current_screen(), Screen::Bienvenida);
    assert!(app.usuario_actual.is_none());

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::UsuarioForm(_)));
    type_text(&mut app, "Ana");
    press_ctrl(&mut app, 's');

    assert_eq!(app.current_screen(), Screen::Bienvenida);
    assert_eq!(app.usuario_actual.as_ref().unwrap().nombre, "Ana");
    let texto = render_con(&mut app, 100, 30);
    assert!(texto.contains("Hola, Ana"), "{texto}");

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert!(app.bienvenida_vista);
}

#[test]
fn login_rejects_wrong_password() {
    let (mut app, _dir) = test_app();
    abrir_usuario(&mut app);

    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Beto");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "clave");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "clave");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::Bienvenida);
    press(&mut app, KeyCode::Enter);

    abrir_usuario(&mut app);
    press(&mut app, KeyCode::Char('l'));
    assert_eq!(app.current_screen(), Screen::UsuarioLogin);
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "mala");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::UsuarioLogin);
    assert!(app.modal.is_some());
}

fn abrir_usuario(app: &mut mifutbol_rs::app::App) {
    let actual = app.menu_state.selected().unwrap_or(0);
    let pasos = (18 + 19 - actual) % 19;
    for _ in 0..pasos {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::UsuarioList);
}
