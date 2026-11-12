//! Flujo del módulo Ajustes por la TUI (§18.7).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};
use mifutbol_rs::domain::models::Theme;

mod common;
use common::*;

fn open_ajustes(app: &mut App) {
    app.menu_state.select(Some(10));
    press(app, KeyCode::Enter);
}

fn archivos_export(app: &App) -> Vec<String> {
    std::fs::read_dir(&app.paths.export_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn change_theme_and_persist() {
    let (mut app, _dir) = test_app();
    open_ajustes(&mut app);
    assert_eq!(app.current_screen(), Screen::AjustesForm);

    let text = render_con(&mut app, 120, 34);
    assert!(text.contains("Tema"));

    // El tema arranca en Light (seed); Right lo pasa a Dark.
    press(&mut app, KeyCode::Right);
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert_eq!(app.settings.theme, Theme::Dark);
    let guardado = app.db.settings().unwrap();
    assert_eq!(guardado.theme, Theme::Dark);
}

#[test]
fn backup_export_and_integrity() {
    let (mut app, _dir) = test_app();
    open_ajustes(&mut app);

    press_ctrl(&mut app, 'b');
    assert!(matches!(app.modal, Some(mifutbol_rs::app::Modal::Info(_))));
    let archivos = archivos_export(&app);
    assert!(
        archivos.iter().any(|nombre| nombre.starts_with("backup_")),
        "{archivos:?}"
    );
    press(&mut app, KeyCode::Enter);

    press_ctrl(&mut app, 'i');
    match &app.modal {
        Some(mifutbol_rs::app::Modal::Info(texto)) => {
            assert!(texto.contains("Integridad: ok"));
        }
        otro => panic!("modal inesperado: {otro:?}"),
    }
    press(&mut app, KeyCode::Esc);

    press_ctrl(&mut app, 'e');
    let archivos = archivos_export(&app);
    assert!(
        archivos
            .iter()
            .any(|nombre| nombre.starts_with("mifutbol_export_")),
        "{archivos:?}"
    );
}

#[test]
fn restore_latest_backup() {
    let (mut app, _dir) = test_app();

    // Guardar tema Dark y crear backup.
    open_ajustes(&mut app);
    press(&mut app, KeyCode::Right);
    press_ctrl(&mut app, 's');
    assert_eq!(app.settings.theme, Theme::Dark);
    open_ajustes(&mut app);
    press_ctrl(&mut app, 'b');
    press(&mut app, KeyCode::Enter);

    // Cambiar el tema a Blue y guardar.
    press(&mut app, KeyCode::Right);
    press_ctrl(&mut app, 's');
    assert_eq!(app.settings.theme, Theme::Blue);

    // Restaurar el backup (Dark).
    open_ajustes(&mut app);
    press_ctrl(&mut app, 'r');
    assert!(matches!(
        app.modal,
        Some(mifutbol_rs::app::Modal::Confirm { .. })
    ));
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.settings.theme, Theme::Dark);
    assert_eq!(app.db.settings().unwrap().theme, Theme::Dark);
}

#[test]
fn import_legacy_database_through_ui() {
    let (mut app, dir) = test_app();
    let ruta = dir.path().join("legacy.db");
    {
        let legacy = rusqlite::Connection::open(&ruta).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE camiseta (id INTEGER PRIMARY KEY, nombre TEXT, activa INTEGER, sorteada INTEGER);
                 CREATE TABLE cancha (id INTEGER PRIMARY KEY, nombre TEXT, localidad TEXT, activa INTEGER);
                 CREATE TABLE partido (id INTEGER PRIMARY KEY, cancha_id INTEGER, camiseta_id INTEGER, fecha_hora TEXT);
                 INSERT INTO camiseta (id, nombre, activa, sorteada) VALUES (1, 'Legacy FC', 1, 0);
                 INSERT INTO cancha (id, nombre, localidad, activa) VALUES (1, 'Sur', 'Quilmes', 1);
                 INSERT INTO partido (id, cancha_id, camiseta_id, fecha_hora) VALUES (1, 1, 1, '2026-02-01 19:00');",
            )
            .unwrap();
    }

    open_ajustes(&mut app);
    press_ctrl(&mut app, 'l');
    assert_eq!(app.current_screen(), Screen::LegacyImportForm);
    type_text(&mut app, &ruta.to_string_lossy());
    press_ctrl(&mut app, 's');

    assert!(matches!(app.modal, Some(mifutbol_rs::app::Modal::Info(_))));
    assert_eq!(app.current_screen(), Screen::AjustesForm);
    let camisetas: i64 = app
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM camiseta", [], |row| row.get(0))
        .unwrap();
    let partidos: i64 = app
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0))
        .unwrap();
    assert_eq!(camisetas, 1);
    assert_eq!(partidos, 1);
    let fecha: String = app
        .db
        .conn()
        .query_row("SELECT fecha_hora FROM partido WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(fecha, "2026-02-01 19:00:00");
}
