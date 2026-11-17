//! Humo de los módulos portados de MiFutbolC: Bienestar, Colecciones y Entrenador IA.

mod common;

use common::*;
use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};
use mifutbol_rs::domain::bienestar::Seccion;

fn abrir_por_paleta(app: &mut App, texto: &str) {
    press_ctrl_code(app, KeyCode::Char('k'));
    type_text(app, texto);
    press(app, KeyCode::Enter);
}

#[test]
fn bienestar_creates_records_per_section_and_runs_assistant() {
    let (mut app, _dir) = test_app();
    abrir_por_paleta(&mut app, "bienestar");
    assert_eq!(app.current_screen(), Screen::BienestarList);
    let texto = render_con(&mut app, 100, 30);
    assert!(
        texto.contains("Hábitos") && texto.contains("Salud"),
        "{texto}"
    );

    // Hábito con los valores por defecto (fecha de hoy).
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::BienestarForm(_)));
    render_con(&mut app, 100, 30);
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::BienestarList);
    assert_eq!(app.bienestar_registros.len(), 1);

    press(&mut app, KeyCode::Tab);
    assert_eq!(app.bienestar_seccion, Seccion::Entrenamientos);
    press(&mut app, KeyCode::Char('n'));
    press_ctrl(&mut app, 's');
    assert_eq!(app.bienestar_registros.len(), 1);
    press(&mut app, KeyCode::Char('o'));
    assert_eq!(app.bienestar_registros[0].valores[4], "Sí");

    press(&mut app, KeyCode::Char('a'));
    assert_eq!(app.current_screen(), Screen::BienestarAsistente);
    let texto = render_con(&mut app, 100, 30);
    assert!(texto.contains("Preparación"), "{texto}");
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.bienestar_recomendaciones.len(), 1);
    press(&mut app, KeyCode::Esc);

    // El perfil de salud existe siempre y no se puede borrar.
    for _ in 0..3 {
        press(&mut app, KeyCode::Tab);
    }
    assert_eq!(app.bienestar_seccion, Seccion::Salud);
    assert_eq!(app.bienestar_registros.len(), 1);
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_none());
}

#[test]
fn colecciones_sync_assign_and_remove() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    abrir_por_paleta(&mut app, "colecciones");
    assert_eq!(app.current_screen(), Screen::ColeccionesList);

    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.inventario.len(), 1);
    let texto = render_con(&mut app, 100, 30);
    assert!(texto.contains("Titular"), "{texto}");

    press(&mut app, KeyCode::Char('a'));
    assert!(matches!(app.current_screen(), Screen::ColeccionForm(_)));
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::ColeccionesList);
    assert_eq!(app.inventario[0].colecciones, 1);

    press(&mut app, KeyCode::Tab);
    assert!(app.colecciones_ver_colecciones);
    render_con(&mut app, 100, 30);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::ColeccionDetail(_)));
    assert_eq!(app.coleccion_detalle.len(), 1);
    render_con(&mut app, 100, 30);
    press(&mut app, KeyCode::Char('u'));
    assert!(app.coleccion_detalle.is_empty());
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Retro");
    press_ctrl(&mut app, 's');
    assert_eq!(app.colecciones.len(), 4);
}

#[test]
fn entrenador_shows_analysis_and_records_feedback() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    abrir_por_paleta(&mut app, "entrenador");
    assert_eq!(app.current_screen(), Screen::EntrenadorIa);
    let texto = render_con(&mut app, 100, 30);
    assert!(
        texto.contains("Consejos") && texto.contains("Predicción"),
        "{texto}"
    );
    assert_eq!(app.entrenador.as_ref().unwrap().estado.partidos, 3);

    press(&mut app, KeyCode::Char('s'));
    assert!(app.entrenador.as_ref().unwrap().historial.0 > 0);
    press(&mut app, KeyCode::Char('v'));
    assert_eq!(app.entrenador.as_ref().unwrap().nivel, 3);
    render_con(&mut app, 80, 24);
}
