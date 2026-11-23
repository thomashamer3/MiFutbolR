//! Pruebas de humo de la TUI con `TestBackend` (§22.1).

use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use mifutbol_rs::app::Screen;

mod common;
use common::*;

#[test]
fn main_menu_lists_all_modules() {
    let (mut app, _dir) = test_app();
    // 22 módulos: con 33 filas entran todos sin desplazar la lista.
    let text = render_con(&mut app, 100, 33);
    assert!(text.contains("MiFutbolR"), "falta el nombre de la app");
    for label in [
        "Camisetas",
        "Canchas",
        "Botines",
        "Equipos",
        "Partidos",
        "Lesiones",
        "Estadísticas",
        "Financiamiento",
        "Carrera futbolística",
        "Resumen compartible",
        "Ajustes",
        "Torneos",
        "Temporadas",
        "Tiendas",
        "Amigos",
        "Multimedia",
        "Recordatorios",
        "Progresión",
        "Bienestar",
        "Colecciones",
        "Entrenador IA",
        "Usuario",
    ] {
        assert!(text.contains(label), "falta el módulo {label}");
    }
}

#[test]
fn enter_opens_camisetas_and_esc_returns() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CamisetasList);
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("No hay camisetas"), "estado vacío");

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn arrow_keys_move_the_selection_with_wrap() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Up);
    assert_eq!(app.selected_module(), mifutbol_rs::app::Module::Usuario);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.selected_module(), mifutbol_rs::app::Module::Camisetas);
    press(&mut app, KeyCode::Char('3'));
    assert_eq!(app.selected_module(), mifutbol_rs::app::Module::Botines);
}

#[test]
fn quit_requires_confirmation() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('q'));
    assert!(app.modal.is_some());
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("¿Cerrar MiFutbolR?"), "el modal no se dibuja");
    assert!(!app.should_quit);

    press(&mut app, KeyCode::Char('s'));
    assert!(app.should_quit);
}

#[test]
fn help_screen_renders_shortcuts() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('?'));
    assert_eq!(app.current_screen(), Screen::Ayuda);
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Atajos globales"));
    assert!(text.contains("Guardar formulario"));
}

#[test]
fn renders_in_narrow_and_short_terminals() {
    let (mut app, _dir) = test_app();
    for (width, height) in [(60, 20), (40, 15), (120, 40)] {
        let text = render_con(&mut app, width, height);
        assert!(!text.is_empty(), "sin contenido en {width}x{height}");
    }
}

#[test]
fn mouse_wheel_moves_menu_selection() {
    let (mut app, _dir) = test_app();
    let abajo = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 5,
        row: 5,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(abajo).expect("mouse");
    assert_eq!(app.menu_state.selected(), Some(1));
    let arriba = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        ..abajo
    };
    app.handle_mouse(arriba).expect("mouse");
    assert_eq!(app.menu_state.selected(), Some(0));
}

#[test]
fn tiny_terminal_with_modal_does_not_panic() {
    let (mut app, _dir) = test_app();
    app.modal = Some(mifutbol_rs::app::Modal::Error(
        "Un mensaje largo de error que no entra en una terminal angosta".to_string(),
    ));
    let texto = render_con(&mut app, 30, 10);
    assert!(texto.contains("Terminal demasiado"), "{texto}");
    // En el mínimo exacto el modal se dibuja sin pánico.
    render_con(&mut app, 40, 12);
}
