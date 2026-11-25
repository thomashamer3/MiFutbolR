//! Importación de perfiles de MiFutbolC (users.db + mifutbol_<usuario>.db).

use std::path::Path;

use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::services::{perfiles_import_service, usuario_service};
use rusqlite::{Connection, params};

/// Implementación independiente del hash de `utils.c` (FNV-1a 64, dos rondas).
fn hash_c(password: &str, salt: &str) -> String {
    let fnv = |texto: &str| {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in texto.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
        hash
    };
    let h1 = fnv(&format!("{salt}:{password}"));
    format!("{:016x}", fnv(&format!("{password}:{h1:016x}:{salt}")))
}

fn crear_directorio_c(dir: &Path) {
    let users = Connection::open(dir.join("users.db")).unwrap();
    users
        .execute_batch(
            "CREATE TABLE local_users (id INTEGER PRIMARY KEY AUTOINCREMENT,
                 username TEXT NOT NULL UNIQUE, password_salt TEXT DEFAULT '',
                 password_hash TEXT DEFAULT '', created_at TEXT DEFAULT CURRENT_TIMESTAMP);
             INSERT INTO local_users (username) VALUES ('Ana');",
        )
        .unwrap();
    users
        .execute(
            "INSERT INTO local_users (username, password_salt, password_hash) VALUES ('Beto', ?1, ?2)",
            params!["00ff", hash_c("clave", "00ff")],
        )
        .unwrap();

    let datos = Connection::open(dir.join("mifutbol_Ana.db")).unwrap();
    datos
        .execute_batch(
            "CREATE TABLE camiseta (id INTEGER PRIMARY KEY, nombre TEXT, activa INTEGER, sorteada INTEGER);
             CREATE TABLE cancha (id INTEGER PRIMARY KEY, nombre TEXT, localidad TEXT, activa INTEGER);
             CREATE TABLE partido (id INTEGER PRIMARY KEY, cancha_id INTEGER, camiseta_id INTEGER,
                 fecha_hora TEXT, goles INTEGER, asistencias INTEGER, resultado INTEGER,
                 rendimiento_general INTEGER, clima INTEGER, dia INTEGER, tipo_partido INTEGER);
             INSERT INTO camiseta VALUES (1, 'Titular', 1, 0);
             INSERT INTO cancha VALUES (1, 'Sur', 'Quilmes', 1);
             INSERT INTO partido VALUES (10, 1, 1, '2026-03-01 20:00', 2, 1, 1, 8, 1, 2, 1);",
        )
        .unwrap();
}

#[test]
fn imports_profiles_with_own_database_and_passwords() {
    let origen = tempfile::tempdir().unwrap();
    crear_directorio_c(origen.path());
    let users_antes = std::fs::read(origen.path().join("users.db")).unwrap();
    let ana_antes = std::fs::read(origen.path().join("mifutbol_Ana.db")).unwrap();

    let destino = tempfile::tempdir().unwrap();
    let paths = AppPaths::with_root(destino.path());
    paths.ensure_dirs().unwrap();
    let cuentas = Database::open_cuentas(&paths.cuentas_file).unwrap();

    let resumen =
        perfiles_import_service::importar_perfiles(cuentas.conn(), &paths, origen.path()).unwrap();
    assert_eq!(resumen.perfiles.len(), 2, "{resumen:?}");
    assert!(
        resumen.advertencias.is_empty(),
        "{:?}",
        resumen.advertencias
    );
    let ana = &resumen.perfiles[0];
    assert_eq!((ana.nombre.as_str(), ana.con_password), ("Ana", false));
    assert!(ana.filas >= 3);
    assert!(resumen.perfiles[1].con_password);

    // Ana tiene su propia base con el partido; Beto no tenía datos.
    let cuentas_lista = usuario_service::listar(cuentas.conn()).unwrap();
    assert!(cuentas_lista.iter().any(|u| u.nombre == "Ana"));
    let db_ana = Database::open(&paths.db_usuario("Ana")).unwrap();
    let partidos: i64 = db_ana
        .conn()
        .query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0))
        .unwrap();
    assert_eq!(partidos, 1);

    // La contraseña de MiFutbolC sigue sirviendo.
    assert!(usuario_service::autenticar(cuentas.conn(), "Beto", Some("mala")).is_err());
    assert!(usuario_service::autenticar(cuentas.conn(), "Beto", Some("clave")).is_ok());
    assert!(usuario_service::autenticar(cuentas.conn(), "Ana", None).is_ok());

    // Repetir no duplica: avisa y omite.
    let otra =
        perfiles_import_service::importar_perfiles(cuentas.conn(), &paths, origen.path()).unwrap();
    assert!(otra.perfiles.is_empty());
    assert_eq!(otra.advertencias.len(), 2, "{:?}", otra.advertencias);
    assert_eq!(usuario_service::listar(cuentas.conn()).unwrap().len(), 2);

    // Los archivos de MiFutbolC no se tocaron.
    assert_eq!(
        std::fs::read(origen.path().join("users.db")).unwrap(),
        users_antes
    );
    assert_eq!(
        std::fs::read(origen.path().join("mifutbol_Ana.db")).unwrap(),
        ana_antes
    );
}

#[test]
fn missing_users_db_suggests_legacy_import() {
    let origen = tempfile::tempdir().unwrap();
    let destino = tempfile::tempdir().unwrap();
    let paths = AppPaths::with_root(destino.path());
    let cuentas = Database::open_cuentas(&paths.cuentas_file).unwrap();

    assert!(
        perfiles_import_service::importar_perfiles(cuentas.conn(), &paths, origen.path()).is_err()
    );
    Connection::open(origen.path().join("mifutbol.db")).unwrap();
    let resumen =
        perfiles_import_service::importar_perfiles(cuentas.conn(), &paths, origen.path()).unwrap();
    assert!(resumen.perfiles.is_empty());
    assert!(resumen.advertencias[0].contains("--importar-legacy"));
}
