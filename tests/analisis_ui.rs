//! Análisis básico, evolución temporal y química por la TUI (B4a).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Module, Screen};

mod common;
use common::*;

fn abrir_hub(app: &mut App, indice: usize) {
    abrir_opcion(app, Module::Analisis);
    app.analisis_hub.select(Some(indice));
    press(app, KeyCode::Enter);
}

#[test]
fn analisis_basico_muestra_rachas_y_comparacion() {
    let (mut app, _dir) = test_app();
    abrir_hub(&mut app, 0);
    assert_eq!(app.current_screen(), Screen::AnalisisBasico);
    assert!(render_con(&mut app, 100, 30).contains("No hay suficientes datos"));
    press(&mut app, KeyCode::Esc);

    preparar_partidos(&app);
    app.analisis_hub.select(Some(0));
    press(&mut app, KeyCode::Enter);
    let texto = render_con(&mut app, 110, 40);
    assert!(texto.contains("Mejor racha de victorias"), "{texto}");
    assert!(texto.contains("2 partidos"), "{texto}");
    assert!(texto.contains("Últimos 5 vs promedio general"), "{texto}");
}

#[test]
fn evolucion_cambia_de_metrica() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    abrir_hub(&mut app, 4);
    assert_eq!(app.current_screen(), Screen::AnalisisEvolucion);
    let texto = render_con(&mut app, 110, 40);
    assert!(texto.contains("Marzo 2026"), "{texto}");
    assert!(texto.contains("Goles promedio por mes"), "{texto}");
    press(&mut app, KeyCode::Char('m'));
    let texto = render_con(&mut app, 110, 40);
    assert!(texto.contains("Asistencias promedio por mes"), "{texto}");
    assert!(texto.contains("Mejor mes histórico"), "{texto}");
}

#[test]
fn quimica_crea_y_borra_una_dupla() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute_batch(
            "INSERT INTO equipo (nombre) VALUES ('Amigos');
             INSERT INTO jugador (equipo_id, nombre, numero, posicion)
                 VALUES (1, 'Ana', 1, 1), (1, 'Beto', 2, 2);",
        )
        .unwrap();
    abrir_hub(&mut app, 3);
    assert_eq!(app.current_screen(), Screen::Quimica);
    assert!(render_con(&mut app, 110, 30).contains("Sin duplas cargadas"));

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::QuimicaForm(_)));
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::Quimica);
    assert_eq!(app.quimica_duplas.len(), 1);
    let texto = render_con(&mut app, 110, 30);
    assert!(texto.contains("Ana + Beto"), "{texto}");

    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('s'));
    assert!(app.quimica_duplas.is_empty());
}
