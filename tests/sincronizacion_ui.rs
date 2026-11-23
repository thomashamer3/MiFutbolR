//! 25 Sincronización (PocketBase) y MiFutbolMobile desde la TUI. Sin red: la
//! URL apunta a un puerto cerrado y nunca se lanza la sincronización.

use crossterm::event::KeyCode;
use mifutbol_rs::app::{Modal, Module, Screen};
use mifutbol_rs::services::sync_service::Config;

mod common;
use common::*;

#[test]
fn menu_25_configura_el_servidor_y_no_muestra_el_token() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Sincronizacion);
    assert_eq!(app.current_screen(), Screen::Sincronizacion);
    assert_eq!(Module::Sincronizacion.numero(), 25);

    press(&mut app, KeyCode::Char('1'));
    assert_eq!(app.current_screen(), Screen::SyncConfigForm);
    type_text(&mut app, "192.168.0.10:8090/");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::Sincronizacion);
    let cfg = Config::cargar(app.db.conn()).unwrap();
    assert_eq!(cfg.url_local, "http://192.168.0.10:8090");

    app.db
        .conn()
        .execute(
            "INSERT INTO sync_config (clave, valor) VALUES ('token', 'tok-secreto'), ('user_id', 'u1')",
            [],
        )
        .unwrap();
    press(&mut app, KeyCode::Char('5'));
    let text = render_con(&mut app, 120, 40);
    assert!(text.contains("http://192.168.0.10:8090"), "{text}");
    assert!(text.contains("iniciada"), "{text}");
    assert!(!text.contains("tok-secreto"), "el token nunca se muestra");

    press(&mut app, KeyCode::Char('7'));
    assert!(Config::cargar(app.db.conn()).unwrap().auto_sync);
    press(&mut app, KeyCode::Char('8'));
    assert!(!Config::cargar(app.db.conn()).unwrap().con_sesion());

    press(&mut app, KeyCode::Char('6'));
    assert!(app.status.contains("No hay conflictos"), "{}", app.status);
}

#[test]
fn url_invalida_muestra_error() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Sincronizacion);
    press(&mut app, KeyCode::Char('1'));
    type_text(&mut app, "ftp://x");
    press_ctrl(&mut app, 's');
    assert!(matches!(app.modal, Some(Modal::Error(_))));
    assert_eq!(app.current_screen(), Screen::SyncConfigForm);
}

#[test]
fn ajustes_exporta_e_importa_para_mifutbolmobile() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    abrir_opcion(&mut app, Module::Ajustes);

    press_ctrl(&mut app, 'w');
    let ruta = app.paths.export_dir.join("mifutbolmobile_import.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&ruta).unwrap()).unwrap();
    assert_eq!(json["formato"], "mifutbolmobile-import");
    assert_eq!(json["partidos"].as_array().unwrap().len(), 3);
    press(&mut app, KeyCode::Enter);

    let camiseta: String = app
        .db
        .conn()
        .query_row("SELECT nombre FROM camiseta LIMIT 1", [], |r| r.get(0))
        .unwrap();
    std::fs::write(
        app.paths.import_dir.join("partidos.json"),
        format!(
            r#"[{{"cancha": "Movil", "fecha": "2026-07-01 19:00", "goles": 1, "asistencias": 2, "camiseta": "{camiseta}"}}]"#
        ),
    )
    .unwrap();
    press_ctrl(&mut app, 'y');
    match &app.modal {
        Some(Modal::Info(texto)) => assert!(texto.contains("1 de 1"), "{texto}"),
        otro => panic!("modal inesperado: {otro:?}"),
    }
    press(&mut app, KeyCode::Enter);

    press_ctrl(&mut app, 'a');
    assert!(app.paths.import_dir.join(".sync_partidos_auto").exists());
    assert!(app.status.contains("ACTIVADA"), "{}", app.status);
}

#[test]
fn al_entrar_importa_los_partidos_del_movil_una_vez() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    let camiseta: String = app
        .db
        .conn()
        .query_row("SELECT nombre FROM camiseta LIMIT 1", [], |r| r.get(0))
        .unwrap();
    std::fs::write(app.paths.import_dir.join(".sync_partidos_auto"), "activo\n").unwrap();
    std::fs::write(
        app.paths.import_dir.join("partidos.json"),
        format!(
            r#"[{{"cancha": "Movil", "fecha": "2026-07-02 19:00", "goles": 3, "asistencias": 0, "camiseta": "{camiseta}"}}]"#
        ),
    )
    .unwrap();
    app.sincronizar_al_iniciar();
    assert!(
        app.status.contains("1 partidos importados"),
        "{}",
        app.status
    );
    assert!(!app.paths.import_dir.join("partidos.json").exists());
    let renombrado = std::fs::read_dir(&app.paths.import_dir)
        .unwrap()
        .filter_map(Result::ok)
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("partidos_importados_")
        });
    assert!(renombrado);
    // Con auto-sync activado pero sin sesión no se lanza nada (y nunca hay red).
    assert!(!app.sync.ocupado());
}
