//! B8: tareas de arranque de C (backup de inicio, Modo Retro, actualizaciones),
//! menús personalizados, ver/restablecer configuración e idioma.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Modal, Module, Screen};
use mifutbol_rs::domain::models::{Language, Mode, Theme};

mod common;
use common::*;

fn archivos_export(app: &App) -> Vec<String> {
    std::fs::read_dir(&app.paths.export_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Servidor falso en 127.0.0.1:0 (nunca la red real) que responde `cuerpo`.
fn github_falso(cuerpo: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut lector = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut linea = String::new();
                if lector.read_line(&mut linea).unwrap_or(0) == 0 || linea == "\r\n" {
                    break;
                }
            }
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
                cuerpo.len()
            );
        }
    });
    url
}

#[test]
fn backup_de_inicio_solo_si_el_binario_lo_activa_y_con_datos() {
    let (mut app, _dir) = test_app();
    app.tareas_de_inicio();
    preparar_partidos(&app);
    app.tareas_de_inicio();
    assert!(
        archivos_export(&app).is_empty(),
        "sin tareas_inicio no hay backup"
    );

    app.tareas_inicio = true;
    app.tareas_de_inicio();
    let archivos = archivos_export(&app);
    assert!(
        archivos
            .iter()
            .any(|n| n.starts_with("backup_") && n.ends_with("_inicio.db")),
        "{archivos:?}"
    );
    // Lo ve la administración de backups (B4).
    open_backups(&mut app);
    assert!(app.backups.iter().any(|b| b.nombre.ends_with("_inicio.db")));
    // Una sola vez por base.
    app.tareas_de_inicio();
    assert_eq!(archivos_export(&app).len(), archivos.len());
}

fn open_backups(app: &mut App) {
    abrir_opcion(app, Module::Ajustes);
    press_ctrl(app, 'g');
    assert_eq!(app.current_screen(), Screen::BackupsList);
}

#[test]
fn aviso_modo_retro_abre_carrera_retro() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    // Mismo día y mes, cuatro años atrás (válido también un 29 de febrero).
    let hoy = mifutbol_rs::domain::dates::today();
    let fecha = format!(
        "{:04}-{} 18:00:00",
        i32::from(hoy.year()) - 4,
        hoy.strftime("%m-%d")
    );
    app.db
        .conn()
        .execute(
            "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora) VALUES (1, 1, ?1)",
            [fecha],
        )
        .unwrap();
    app.tareas_inicio = true;
    app.tareas_de_inicio();
    match &app.modal {
        Some(Modal::Confirm { title, message, .. }) => {
            assert_eq!(title, "Modo Retro");
            assert!(message.contains("1 recuerdo"), "{message}");
        }
        otro => panic!("modal inesperado: {otro:?}"),
    }
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.current_screen(), Screen::CarreraRetro);
}

#[test]
fn buscar_actualizaciones_manual_y_al_inicio_contra_servidor_falso() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Ajustes);
    // Sin URL (como en todos los tests) no se consulta nada.
    press_ctrl(&mut app, 'u');
    assert!(!app.esperar_actualizacion(Duration::from_millis(50)));

    app.url_actualizaciones = Some(github_falso(
        r#"{"tag_name":"v99.0.0","html_url":"https://example.invalid/release"}"#,
    ));
    press_ctrl(&mut app, 'u');
    assert!(app.esperar_actualizacion(Duration::from_secs(10)));
    match &app.modal {
        Some(Modal::Info(texto)) => {
            assert!(texto.contains("NUEVA VERSIÓN"), "{texto}");
            assert!(texto.contains("v99.0.0") && texto.contains("example.invalid"));
        }
        otro => panic!("modal inesperado: {otro:?}"),
    }
    press(&mut app, KeyCode::Enter);

    // Al inicio: una vez por día (la marca queda en data_dir).
    app.tareas_inicio = true;
    app.tareas_de_inicio();
    assert!(app.esperar_actualizacion(Duration::from_secs(10)));
    assert!(app.status.contains("v99.0.0"), "{}", app.status);
    assert!(app.paths.data_dir.join("update_check.stamp").exists());
}

#[test]
fn desactivar_la_comprobacion_al_iniciar() {
    let (mut app, _dir) = test_app();
    app.settings.buscar_actualizaciones = false;
    app.url_actualizaciones = Some(github_falso(r#"{"tag_name":"v99.0.0"}"#));
    app.tareas_inicio = true;
    app.tareas_de_inicio();
    assert!(!app.esperar_actualizacion(Duration::from_millis(200)));
    assert!(!app.paths.data_dir.join("update_check.stamp").exists());

    // El campo del formulario se guarda.
    abrir_opcion(&mut app, Module::Ajustes);
    let campo = app
        .form
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .find(|f| f.key == "actualizaciones")
        .unwrap();
    assert_eq!(campo.value, "No");
    press_ctrl(&mut app, 's');
    assert!(!app.db.settings().unwrap().buscar_actualizaciones);
}

#[test]
fn editor_de_menus_oculta_en_personalizado() {
    let (mut app, _dir) = test_app();
    app.settings.mode = Mode::Custom;
    abrir_opcion(&mut app, Module::Ajustes);
    press_ctrl(&mut app, 'n');
    assert_eq!(app.current_screen(), Screen::MenusPersonalizados);
    let text = render_con(&mut app, 120, 40);
    assert!(text.contains("[X]") && text.contains("Camisetas"), "{text}");
    assert!(
        !App::menus_editables().contains(&Module::Ajustes),
        "Ajustes no se puede ocultar"
    );

    // Calendario es la primera; abajo = Camisetas.
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(!app.menu_modulos().contains(&Module::Camisetas));
    assert!(app.menu_modulos().contains(&Module::Ajustes));
    assert!(render_con(&mut app, 120, 40).contains("[ ]"));
    // Persistido en custom_menus.
    let enabled: i64 = app
        .db
        .conn()
        .query_row(
            "SELECT enabled FROM custom_menus WHERE menu_name = 'camisetas'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(enabled, 0);
    // En Avanzado se ve igual; Espacio la vuelve a mostrar.
    app.settings.mode = Mode::Advanced;
    assert!(app.menu_modulos().contains(&Module::Camisetas));
    app.settings.mode = Mode::Custom;
    press(&mut app, KeyCode::Char(' '));
    assert!(app.menu_modulos().contains(&Module::Camisetas));
}

#[test]
fn ver_configuracion_sin_secretos_y_restablecer() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute(
            "INSERT OR REPLACE INTO sync_config (clave, valor) VALUES ('token', 'tok-secreto-123')",
            [],
        )
        .unwrap();
    abrir_opcion(&mut app, Module::Ajustes);
    press(&mut app, KeyCode::Right); // Tema → Oscuro
    press_ctrl(&mut app, 's');
    assert_eq!(app.settings.theme, Theme::Dark);

    abrir_opcion(&mut app, Module::Ajustes);
    press_ctrl(&mut app, 'f');
    match &app.modal {
        Some(Modal::Info(texto)) => {
            assert!(texto.contains("Tema: Oscuro"), "{texto}");
            assert!(texto.contains("Usuario: Invitado"), "{texto}");
            assert!(!texto.contains("tok-secreto"), "{texto}");
        }
        otro => panic!("modal inesperado: {otro:?}"),
    }
    press(&mut app, KeyCode::Enter);

    press_ctrl(&mut app, 'q');
    assert!(matches!(app.modal, Some(Modal::Confirm { .. })));
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.settings.theme, Theme::Dark);
    press_ctrl(&mut app, 'q');
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.settings.theme, Theme::Light);
    assert_eq!(app.settings.mode, Mode::Simple);
    assert_eq!(app.db.settings().unwrap().theme, Theme::Light);
    // El formulario abierto muestra los valores nuevos.
    assert_eq!(app.form.as_ref().unwrap().value("theme"), Some("0"));
}

#[test]
fn idioma_ingles_traduce_el_menu() {
    let (mut app, _dir) = test_app();
    // Alto suficiente para ver la lista entera hasta "0 Salir".
    let text = render_con(&mut app, 120, 70);
    assert!(text.contains("Camisetas") && text.contains(" 0   Salir"));
    app.settings.language = Language::English;
    let text = render_con(&mut app, 120, 70);
    assert!(text.contains("Shirts"), "{text}");
    assert!(text.contains(" 0   Exit"), "{text}");
}
