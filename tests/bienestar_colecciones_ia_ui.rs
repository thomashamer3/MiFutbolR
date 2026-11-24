//! Humo de los módulos portados de MiFutbolC: Bienestar, Colecciones y Entrenador IA.

mod common;

use common::*;
use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Modal, Screen};
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
    for _ in 0..6 {
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

fn ir_a(app: &mut App, seccion: Seccion) {
    for _ in 0..Seccion::ALL.len() {
        if app.bienestar_seccion == seccion {
            return;
        }
        press(app, KeyCode::Tab);
    }
    panic!("no se llegó a {seccion:?}");
}

/// Alta con valores por defecto (más `texto` en el primer campo y `ruta` en el
/// campo de archivo) y render de la pestaña.
fn alta(app: &mut App, seccion: Seccion, texto: &str, ruta: Option<&str>) -> String {
    ir_a(app, seccion);
    press(app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::BienestarForm(_)));
    type_text(app, texto);
    if let (Some(ruta), Some(indice)) = (ruta, seccion.columna_archivo()) {
        for _ in 0..indice {
            press(app, KeyCode::Tab);
        }
        type_text(app, ruta);
    }
    render_con(app, 100, 30);
    press_ctrl(app, 's');
    assert_eq!(
        app.current_screen(),
        Screen::BienestarList,
        "{:?}",
        app.modal
    );
    render_con(app, 100, 30)
}

#[test]
fn bienestar_new_sections_link_records_and_copy_files() {
    let (mut app, dir) = test_app();
    abrir_por_paleta(&mut app, "bienestar");

    let texto = alta(&mut app, Seccion::Objetivos, "Correr 10k", None);
    assert!(
        texto.contains("Correr 10k") && texto.contains("Activo"),
        "{texto}"
    );
    // El plan elige el objetivo de una lista y el listado muestra su nombre.
    let texto = alta(&mut app, Seccion::Planes, "", None);
    assert!(texto.contains("Correr 10k"), "{texto}");

    alta(&mut app, Seccion::Entrenamientos, "", None);
    alta(&mut app, Seccion::Ejercicios, "Sentadilla", None);
    let texto = alta(&mut app, Seccion::Series, "", None);
    assert!(
        texto.contains("Sentadilla") && texto.contains("· Fuerza"),
        "{texto}"
    );

    // Un día nutricional por fecha: el segundo alta del día lo actualiza.
    alta(&mut app, Seccion::Nutricion, "", None);
    alta(&mut app, Seccion::Nutricion, "", None);
    assert_eq!(app.bienestar_registros.len(), 1);

    alta(&mut app, Seccion::Controles, "", None);
    let informe = dir.path().join("informe.txt");
    std::fs::write(&informe, "ok").unwrap();
    let texto = alta(
        &mut app,
        Seccion::Estudios,
        "",
        Some(informe.to_str().unwrap()),
    );
    assert!(
        texto.contains("informe.txt") && texto.contains("documento"),
        "{texto}"
    );
    let ruta = app.bienestar_registros[0].valores[4].clone();
    assert!(app.paths.data_dir.join(&ruta).is_file(), "{ruta}");
    press(&mut app, KeyCode::Char('v'));
    assert!(app.status.starts_with("Archivo:"), "{}", app.status);

    let foto = dir.path().join("menu.png");
    std::fs::write(&foto, "png").unwrap();
    let texto = alta(
        &mut app,
        Seccion::Imagenes,
        "",
        Some(foto.to_str().unwrap()),
    );
    assert!(texto.contains("entrenamiento"), "{texto}");
    assert!(app.bienestar_registros[0].valores[1].starts_with("Imagenes/bienestar_entrenamiento_"));

    // Borrar el control borra sus estudios en cascada (y queda en el historial de deshacer).
    ir_a(&mut app, Seccion::Controles);
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Enter);
    ir_a(&mut app, Seccion::Estudios);
    assert!(app.bienestar_registros.is_empty());
}

#[test]
fn coach_warns_before_loading_a_risky_match() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    let hoy = mifutbol_rs::domain::dates::to_iso_date(mifutbol_rs::domain::dates::today());
    for _ in 0..4 {
        app.db
            .conn()
            .execute(
                "INSERT INTO partido
                    (cancha_id, camiseta_id, fecha_hora, mes_anio, cansancio, resultado,
                     clima, dia, tipo_partido)
                 VALUES (1, 1, ?1 || ' 08:00:00', substr(?1, 1, 7), 9, 3, 1, 2, 1)",
                [&hoy],
            )
            .unwrap();
    }
    abrir_por_paleta(&mut app, "nuevo partido");
    assert_eq!(app.current_screen(), Screen::PartidoWizard);
    let Some(Modal::Info(aviso)) = &app.modal else {
        panic!("sin aviso: {:?}", app.modal);
    };
    assert!(aviso.contains("Entrenador IA"), "{aviso}");
    render_con(&mut app, 100, 30);
    let (ultima, riesgo): (i64, f64) = app
        .db
        .conn()
        .query_row(
            "SELECT ultima_alerta, ultimo_riesgo FROM ia_config WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(ultima > 0 && riesgo > 3.0, "{ultima} {riesgo}");
    // Se acepta y se sigue cargando el partido; no se repite dentro de las 24 h.
    press(&mut app, KeyCode::Enter);
    assert!(app.modal.is_none());
    assert_eq!(app.current_screen(), Screen::PartidoWizard);
}

#[test]
fn colecciones_backup_json_exporta_e_importa() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    abrir_por_paleta(&mut app, "colecciones");
    press(&mut app, KeyCode::Char('s'));
    press(&mut app, KeyCode::Char('x'));
    assert!(matches!(app.modal, Some(Modal::Info(_))));
    press(&mut app, KeyCode::Enter);
    let exportado = app.paths.export_dir.join("colecciones_inventario.json");
    assert!(exportado.exists());
    // Sin archivo en Importaciones/: error claro.
    press(&mut app, KeyCode::Char('i'));
    assert!(matches!(app.modal, Some(Modal::Error(_))));
    press(&mut app, KeyCode::Enter);
    std::fs::create_dir_all(&app.paths.import_dir).unwrap();
    std::fs::copy(
        &exportado,
        app.paths.import_dir.join("colecciones_inventario.json"),
    )
    .unwrap();
    press(&mut app, KeyCode::Char('i'));
    match &app.modal {
        Some(Modal::Info(texto)) => assert!(texto.contains("Relaciones nuevas: 0"), "{texto}"),
        otro => panic!("modal inesperado: {otro:?}"),
    }
    assert_eq!(app.inventario.len(), 1, "no duplica la camiseta");
}

#[test]
fn bienestar_analisis_e_informe_mensual_pdf() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute_batch(
            "INSERT INTO bienestar_comida (fecha, tipo, calidad) VALUES ('2026-03-01', 'Cena', 'Buena');",
        )
        .unwrap();
    abrir_por_paleta(&mut app, "bienestar");
    press(&mut app, KeyCode::Char('A'));
    assert_eq!(app.current_screen(), Screen::Reporte);
    let texto = render_con(&mut app, 110, 40);
    assert!(texto.contains("Entrenamiento vs rendimiento"), "{texto}");
    assert!(texto.contains("Estadísticas de alimentación"), "{texto}");
    assert!(
        app.reporte
            .texto()
            .contains("Porcentaje de días con buena alimentación: 100.0%")
    );
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::BienestarList);

    press(&mut app, KeyCode::Char('p'));
    render_con(&mut app, 110, 30);
    press_ctrl(&mut app, 's');
    assert_eq!(
        app.current_screen(),
        Screen::BienestarList,
        "{:?}",
        app.modal
    );
    assert!(app.status.contains("Informe Personal"), "{}", app.status);
    let pdf = std::fs::read_dir(&app.paths.export_dir)
        .unwrap()
        .flatten()
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("Informe Personal")
        })
        .expect("PDF generado");
    assert_eq!(&std::fs::read(pdf.path()).unwrap()[..4], b"%PDF");
}
