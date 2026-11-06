//! Benchmarks de consultas críticas (Performance.md §7).
//!
//! Mide listado paginado, búsqueda, conteo y carga completa de estadísticas
//! sobre una base en memoria con 10 000 partidos.

use std::time::Duration;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use mifutbol_rs::db::Database;
use mifutbol_rs::db::helpers::Page;
use mifutbol_rs::domain::models::PartidoFilter;
use mifutbol_rs::services::{estadisticas_service, partido_service};

fn base_datos() -> Database {
    let mut db = Database::open_in_memory().expect("base en memoria");
    db.conn()
        .execute("INSERT INTO cancha (nombre) VALUES ('Cancha')", [])
        .unwrap();
    db.conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Camiseta')", [])
        .unwrap();
    let tx = db.conn_mut().transaction().unwrap();
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO partido
                    (cancha_id, camiseta_id, fecha_hora, mes_anio, goles, asistencias,
                     resultado, rendimiento_general, rival_nombre, clima, dia, tipo_partido)
                 VALUES (1, 1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 2, 1)",
            )
            .unwrap();
        for indice in 0..10_000u32 {
            let dia = 1 + (indice % 28);
            let mes = 1 + ((indice / 28) % 12);
            let anio = 2020 + (indice / 336) % 7;
            let fecha = format!("{anio:04}-{mes:02}-{dia:02} 20:00:00");
            stmt.execute(rusqlite::params![
                fecha,
                &fecha[..7],
                indice % 4,
                indice % 3,
                (indice % 3) + 1,
                5 + (indice % 6),
                format!("Rival {}", indice % 50),
            ])
            .unwrap();
        }
    }
    tx.commit().unwrap();
    db
}

fn benchmarks(c: &mut Criterion) {
    let mut db = base_datos();
    let mut grupo = c.benchmark_group("partidos");
    grupo.sample_size(10);
    grupo.measurement_time(Duration::from_secs(3));

    grupo.bench_function("page_20", |b| {
        b.iter(|| {
            let filas = partido_service::listar(
                db.conn(),
                black_box(&PartidoFilter::default()),
                Page::numbered(1, 20),
            )
            .unwrap();
            black_box(filas)
        });
    });

    grupo.bench_function("busqueda_texto", |b| {
        b.iter(|| {
            let filtro = PartidoFilter {
                texto: "Rival 7".to_string(),
                ..PartidoFilter::default()
            };
            let filas =
                partido_service::listar(db.conn(), black_box(&filtro), Page::numbered(1, 20))
                    .unwrap();
            black_box(filas)
        });
    });

    grupo.bench_function("conteo", |b| {
        b.iter(|| {
            let total =
                partido_service::contar(db.conn(), black_box(&PartidoFilter::default())).unwrap();
            black_box(total)
        });
    });

    grupo.bench_function("estadisticas_completas", |b| {
        b.iter(|| {
            let estadisticas = estadisticas_service::cargar(db.conn()).unwrap();
            black_box(estadisticas)
        });
    });

    grupo.finish();
    let _ = &mut db;
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
