//! Flujo del Resumen Compartible por la TUI (§17.5).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

mod common;
use common::*;

fn open_resumen(app: &mut App) {
    for _ in 0..9 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

#[test]
fn preview_switches_format_and_exports() {
    let (mut app, dir) = test_app();
    preparar_partidos(&app);
    open_resumen(&mut app);
    assert_eq!(app.current_screen(), Screen::ResumenPreview);

    let text = render_con(&mut app, 120, 40);
    assert!(
        text.contains("Resumen de carrera"),
        "falta el resumen: {text}"
    );
    assert!(text.contains("Partidos"));

    press(&mut app, KeyCode::Char('h'));
    let text = render_con(&mut app, 120, 40);
    assert!(text.contains("<!doctype html>"));

    press(&mut app, KeyCode::Char('g'));
    assert!(app.modal.is_some());
    let archivos = std::fs::read_dir(&app.paths.export_dir)
        .unwrap()
        .filter_map(Result::ok)
        .count();
    assert_eq!(archivos, 1);
    let _ = dir;
}

#[test]
fn empty_state_blocks_export() {
    let (mut app, _dir) = test_app();
    open_resumen(&mut app);
    assert_eq!(app.current_screen(), Screen::ResumenPreview);
    assert!(app.resumen_datos.is_none());
    let text = render_con(&mut app, 120, 40);
    assert!(text.contains("Sin datos suficientes"));

    press(&mut app, KeyCode::Char('g'));
    let archivos = std::fs::read_dir(&app.paths.export_dir)
        .unwrap()
        .filter_map(Result::ok)
        .count();
    assert_eq!(archivos, 0);
}
