//! Flujo del Resumen Compartible por la TUI (§17.5).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mifutbol_rs::app::{App, Screen};
use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn test_app() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_root(dir.path());
    paths.ensure_dirs().expect("dirs");
    let db = Database::open(&paths.db_file).expect("db");
    let app = App::new(db, paths).expect("app");
    (app, dir)
}

fn preparar(app: &App) {
    app.db
        .conn()
        .execute("INSERT INTO cancha (nombre) VALUES ('Sur')", [])
        .unwrap();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
        .unwrap();
    for (fecha, goles, resultado, rendimiento) in [
        ("2026-03-01 20:00:00", 3, 1, 9),
        ("2026-03-08 20:00:00", 1, 1, 7),
        ("2026-03-15 20:00:00", 2, 3, 6),
    ] {
        app.db
            .conn()
            .execute(
                "INSERT INTO partido
                    (cancha_id, camiseta_id, fecha_hora, mes_anio, goles, asistencias,
                     resultado, rendimiento_general, clima, dia, tipo_partido)
                 VALUES (1, 1, ?1, ?2, ?3, 1, ?4, ?5, 1, 2, 1)",
                rusqlite::params![fecha, &fecha[..7], goles, resultado, rendimiento],
            )
            .unwrap();
    }
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("handle_key");
}

fn render(app: &mut App) -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| ui::draw(frame, app)).expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

fn open_resumen(app: &mut App) {
    for _ in 0..9 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

#[test]
fn preview_switches_format_and_exports() {
    let (mut app, dir) = test_app();
    preparar(&app);
    open_resumen(&mut app);
    assert_eq!(app.current_screen(), Screen::ResumenPreview);

    let text = render(&mut app);
    assert!(
        text.contains("Resumen de carrera"),
        "falta el resumen: {text}"
    );
    assert!(text.contains("Partidos"));

    press(&mut app, KeyCode::Char('h'));
    let text = render(&mut app);
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
    let text = render(&mut app);
    assert!(text.contains("Sin datos suficientes"));

    press(&mut app, KeyCode::Char('g'));
    let archivos = std::fs::read_dir(&app.paths.export_dir)
        .unwrap()
        .filter_map(Result::ok)
        .count();
    assert_eq!(archivos, 0);
}
