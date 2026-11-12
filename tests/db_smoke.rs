//! Pruebas de integración de la base de datos (§22.2).

use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::domain::models::Theme;

mod common;

#[test]
fn database_file_survives_reopen_and_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_root(dir.path());
    paths.ensure_dirs().expect("dirs");

    let db = Database::open(&paths.db_file).expect("open");
    assert!(paths.db_file.exists());

    let mut settings = db.settings().expect("settings");
    settings.theme = Theme::HighContrast;
    db.save_settings(&settings).expect("save");
    drop(db);

    let reopened = Database::open(&paths.db_file).expect("reopen");
    assert_eq!(reopened.settings().unwrap().theme, Theme::HighContrast);

    let backup_path = dir.path().join("backup.db");
    reopened.backup(&backup_path).expect("backup");
    let backup = Database::open(&backup_path).expect("open backup");
    assert_eq!(backup.settings().unwrap().theme, Theme::HighContrast);
}

#[test]
fn seeds_create_settings_and_menu_rows() {
    let db = Database::open_in_memory().expect("db");
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM custom_menus WHERE enabled = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 11);

    let identity: String = db
        .conn()
        .query_row(
            "SELECT pie_habil FROM carrera_identidad WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(identity, "Derecho");
}

#[test]
fn inserting_a_match_without_court_fails_by_foreign_key() {
    let db = Database::open_in_memory().expect("db");
    let result = db.conn().execute(
        "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora)
         VALUES (999, 999, '2026-11-06 21:00:00')",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn integrity_check_and_restore() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_root(dir.path());
    paths.ensure_dirs().expect("dirs");

    let mut db = Database::open(&paths.db_file).expect("open");
    let mut settings = db.settings().unwrap();
    settings.theme = Theme::HighContrast;
    db.save_settings(&settings).unwrap();

    let backup = dir.path().join("backup_test.db");
    db.backup(&backup).expect("backup");

    let mut settings = db.settings().unwrap();
    settings.theme = Theme::Light;
    db.save_settings(&settings).unwrap();
    assert_eq!(db.settings().unwrap().theme, Theme::Light);

    db.restore_from(&backup).expect("restore");
    assert_eq!(db.settings().unwrap().theme, Theme::HighContrast);

    let resumen = db.integrity().expect("integrity");
    assert!(resumen.contains("Integridad: ok"), "{resumen}");
    assert!(resumen.contains("partido="));
}
