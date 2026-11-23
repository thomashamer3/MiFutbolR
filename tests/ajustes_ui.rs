//! Flujo del módulo Ajustes por la TUI (§18.7).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};
use mifutbol_rs::domain::models::Theme;

mod common;
use common::*;

fn open_ajustes(app: &mut App) {
    abrir_opcion(app, mifutbol_rs::app::Module::Ajustes);
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

#[test]
fn ofrece_restaurar_backup_si_la_base_esta_vacia() {
    let (mut app, dir) = test_app();
    // Sin backups no se ofrece nada.
    app.ofrecer_restaurar_backup_inicio();
    assert!(app.modal.is_none());

    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
        .unwrap();
    app.backup_ajustes().unwrap();

    // Base nueva y vacía con las mismas carpetas: se ofrece una sola vez.
    let db = mifutbol_rs::db::Database::open(&dir.path().join("vacia.db")).unwrap();
    let _ = mifutbol_rs::services::usuario_service::marcar_bienvenida(db.conn());
    let mut nueva = App::new(db, app.paths.clone()).unwrap();
    assert!(nueva.modal.is_none(), "App::new no abre modales");
    nueva.ofrecer_restaurar_backup_inicio();
    let text = render_con(&mut nueva, 120, 34);
    assert!(text.contains("¿Restaurar backup?"), "{text}");
    press(&mut nueva, KeyCode::Char('s'));
    let camisetas: i64 = nueva
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM camiseta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(camisetas, 1);
    nueva.modal = None;
    nueva.restauracion_ofrecida = false;
    // Con datos ya no se ofrece.
    nueva.ofrecer_restaurar_backup_inicio();
    assert!(nueva.modal.is_none());
}

#[test]
fn administrar_backups_restaura_y_elimina_el_elegido() {
    let (mut app, _dir) = test_app();
    open_ajustes(&mut app);
    press(&mut app, KeyCode::Right);
    press_ctrl(&mut app, 's');
    assert_eq!(app.settings.theme, Theme::Dark);
    open_ajustes(&mut app);
    press_ctrl(&mut app, 'b');
    press(&mut app, KeyCode::Enter);
    // Renombrarlo como más antiguo para que no lo pise el segundo backup.
    let viejo = app.paths.export_dir.join("backup_20000101_000000.db");
    let nombre = archivos_export(&app)
        .into_iter()
        .find(|n| n.starts_with("backup_"))
        .unwrap();
    std::fs::rename(app.paths.export_dir.join(nombre), &viejo).unwrap();

    press(&mut app, KeyCode::Right);
    press_ctrl(&mut app, 's');
    assert_eq!(app.settings.theme, Theme::Blue);
    open_ajustes(&mut app);
    press_ctrl(&mut app, 'g');
    assert_eq!(app.current_screen(), Screen::BackupsList);
    press(&mut app, KeyCode::Char('b'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.backups.len(), 2);
    let texto = render_con(&mut app, 110, 30);
    assert!(texto.contains("backup_20000101_000000.db"), "{texto}");

    // El más reciente va primero: bajar al viejo (Dark) y restaurarlo.
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(
        app.modal,
        Some(mifutbol_rs::app::Modal::Confirm { .. })
    ));
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.db.settings().unwrap().theme, Theme::Dark);
    press(&mut app, KeyCode::Enter);

    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('s'));
    assert!(!viejo.exists());
    assert_eq!(app.backups.len(), 1);
    assert_eq!(app.current_screen(), Screen::BackupsList);
}

#[test]
fn mantenimiento_y_exportar_todo() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    open_ajustes(&mut app);

    press_ctrl(&mut app, 't');
    assert!(matches!(
        app.modal,
        Some(mifutbol_rs::app::Modal::Confirm { .. })
    ));
    press(&mut app, KeyCode::Char('s'));
    match &app.modal {
        Some(mifutbol_rs::app::Modal::Info(texto)) => {
            assert!(texto.contains("VACUUM y ANALYZE completados"), "{texto}");
            assert!(texto.contains("Integridad: ok"), "{texto}");
        }
        otro => panic!("modal inesperado: {otro:?}"),
    }
    assert!(
        archivos_export(&app)
            .iter()
            .any(|n| n.starts_with("backup_") && n.ends_with("_pre_vacuum.db"))
    );
    press(&mut app, KeyCode::Enter);

    press_ctrl(&mut app, 'x');
    assert!(matches!(app.modal, Some(mifutbol_rs::app::Modal::Info(_))));
    let archivos = archivos_export(&app);
    let json = archivos.iter().find(|n| n.ends_with(".json")).unwrap();
    let valor: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(app.paths.export_dir.join(json)).unwrap())
            .unwrap();
    assert_eq!(valor["partido"].as_array().unwrap().len(), 3);
    assert!(archivos.iter().any(|n| n.ends_with(".md")), "{archivos:?}");
}
