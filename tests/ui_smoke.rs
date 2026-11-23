//! Pruebas de humo de la TUI con `TestBackend` (§22.1).

use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use mifutbol_rs::app::{Module, Screen};
use mifutbol_rs::domain::models::Mode;

mod common;
use common::*;

#[test]
fn main_menu_lists_all_modules() {
    let (mut app, _dir) = test_app();
    // 27 opciones + "0 Salir": con 60 filas entran todas sin desplazar la lista.
    let text = render_con(&mut app, 100, 60);
    assert!(text.contains("MiFutbolR"), "falta el nombre de la app");
    for label in [
        " 1   Calendario",
        " 2   Camisetas",
        " 3   Canchas",
        " 4   Botines",
        " 5   Equipos",
        " 6   Partidos",
        " 7   Lesiones",
        " 8   Estadísticas",
        " 9   Logros",
        "10   Financiamiento",
        "11   Torneos",
        "12   Temporada",
        "13   Análisis",
        "14   Bienestar",
        "15   Carrera Futbolística",
        "16   Recordatorios",
        "17   Colecciones",
        "18   Records & Rankings",
        "19   Tiendas",
        "20   Referencias Multimedia",
        "21   Tutorial",
        "22   Resumen Compartible",
        "23   Ranking Amigos",
        "24   Ajustes",
        "25   Progresión",
        "26   Entrenador IA",
        "27   Usuario",
        " 0   Salir",
    ] {
        assert!(text.contains(label), "falta la opción {label}\n{text}");
    }
}

#[test]
fn modo_simple_filtra_el_menu_y_conserva_los_numeros() {
    let (mut app, _dir) = test_app();
    app.settings.mode = Mode::Simple;
    assert_eq!(
        app.menu_modulos(),
        vec![
            Module::Calendario,
            Module::Camisetas,
            Module::Canchas,
            Module::Botines,
            Module::Partidos,
            Module::Lesiones,
            Module::Tutorial,
            Module::Ajustes,
            Module::Usuario,
        ]
    );
    let text = render_con(&mut app, 100, 40);
    assert!(text.contains("24   Ajustes"), "{text}");
    assert!(!text.contains("Financiamiento"), "{text}");
    // 5 (Equipos) no está en Simple: no abre nada.
    press(&mut app, KeyCode::Char('5'));
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert!(app.status.contains("no está disponible"), "{}", app.status);
    // En Simple ningún número visible empieza con 1 salvo el 1: abre ya.
    press(&mut app, KeyCode::Char('1'));
    assert_eq!(app.current_screen(), Screen::Calendario);
}

#[test]
fn modo_por_defecto_es_simple() {
    let (app, _dir) = test_app_con_bienvenida();
    assert_eq!(app.settings.mode, Mode::Simple);
    assert_eq!(app.menu_modulos().len(), 9);
}

#[test]
fn modo_personalizado_oculta_los_menus_desactivados() {
    let (mut app, _dir) = test_app();
    app.settings.mode = Mode::Custom;
    app.menu_ocultos = vec!["logros".to_string(), "settings".to_string()];
    let modulos = app.menu_modulos();
    assert!(!modulos.contains(&Module::Logros));
    assert!(
        modulos.contains(&Module::Ajustes),
        "Ajustes siempre visible"
    );
    assert!(modulos.contains(&Module::Calendario));
}

#[test]
fn numeros_de_dos_cifras_abren_la_opcion() {
    let (mut app, _dir) = test_app();
    // "1" puede ser 1 o 10..19: espera; "5" completa el 15.
    press(&mut app, KeyCode::Char('1'));
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert_eq!(app.selected_module(), Module::Calendario);
    press(&mut app, KeyCode::Char('5'));
    assert_eq!(app.current_screen(), Screen::CarreraMenu);

    press(&mut app, KeyCode::Esc);
    // "3" no es prefijo de ninguna otra opción: abre en el acto.
    press(&mut app, KeyCode::Char('3'));
    assert_eq!(app.current_screen(), Screen::CanchasList);

    press(&mut app, KeyCode::Esc);
    // Enter abre el número tecleado.
    press(&mut app, KeyCode::Char('2'));
    assert_eq!(app.current_screen(), Screen::MainMenu);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CamisetasList);

    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('2'));
    press(&mut app, KeyCode::Char('4'));
    assert_eq!(app.current_screen(), Screen::AjustesForm);
}

#[test]
fn numero_ambiguo_se_abre_al_vencer_la_pausa() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('1'));
    let ahora = std::time::Instant::now();
    assert!(!app.vencer_numero_menu(ahora));
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert!(app.vencer_numero_menu(ahora + mifutbol_rs::app::ESPERA_NUMERO_MENU * 2));
    assert_eq!(app.current_screen(), Screen::Calendario);
    assert!(app.menu_numero.is_none());
}

#[test]
fn cero_y_esc_en_el_menu_piden_confirmar_la_salida() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('0'));
    assert!(app.modal.is_some(), "0 = Salir pide confirmación");
    press(&mut app, KeyCode::Char('n'));
    assert!(app.modal.is_none());
    assert!(!app.should_quit);

    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_some(), "Esc en el menú pide confirmación");
    press(&mut app, KeyCode::Char('s'));
    assert!(app.should_quit);
}

#[test]
fn nuevas_opciones_abren_sus_pantallas() {
    let (mut app, _dir) = test_app();
    for (module, pantalla) in [
        (Module::Calendario, Screen::Calendario),
        (Module::Logros, Screen::Logros),
        (Module::Records, Screen::Records),
        (Module::Tutorial, Screen::Tutorial),
        (Module::Analisis, Screen::AnalisisMenu),
    ] {
        abrir_opcion(&mut app, module);
        assert_eq!(app.current_screen(), pantalla, "{module:?}");
        let _ = render_con(&mut app, 100, 30);
        press(&mut app, KeyCode::Char('g'));
        assert_eq!(app.current_screen(), Screen::MainMenu);
    }
}

#[test]
fn analisis_abre_comparador_y_entrenador() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Analisis);
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Comparador A/B") && text.contains("Entrenador IA"));
    assert!(text.contains("Análisis básico") && text.contains("Química"));
    for (indice, pantalla) in [
        (0, Screen::AnalisisBasico),
        (1, Screen::Comparador),
        (2, Screen::EntrenadorIa),
        (3, Screen::Quimica),
        (4, Screen::AnalisisEvolucion),
    ] {
        app.analisis_hub.select(Some(indice));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.current_screen(), pantalla);
        let _ = render_con(&mut app, 100, 30);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.current_screen(), Screen::AnalisisMenu);
    }
}

#[test]
fn atajos_de_letra_no_saltan_desde_un_modulo() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Tiendas);
    for letra in [
        'c', 'l', 'L', 'R', 't', 'C', 'T', 'U', 'E', 'A', 'M', 'Y', 'P',
    ] {
        press(&mut app, KeyCode::Char(letra));
        assert_eq!(
            app.current_screen(),
            Screen::TiendasList,
            "'{letra}' saltó a otra pantalla"
        );
    }
    // Desde el menú siguen funcionando.
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('c'));
    assert_eq!(app.current_screen(), Screen::Calendario);
}

#[test]
fn enter_opens_camisetas_and_esc_returns() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Camisetas);
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
    assert_eq!(app.selected_module(), Module::Usuario);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.selected_module(), Module::Calendario);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.selected_module(), Module::Camisetas);
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
