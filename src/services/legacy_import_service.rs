//! Importación de una base legacy de MiFutbolC (§24.2).
//!
//! Estrategia: se adjunta la base legacy como `legacy`, se copian las tablas
//! presentes en ambas por intersección de columnas, se normalizan fechas y se
//! convierten los importes legacy (`monto`, `precio`) a centavos.
//! La importación exige una base destino vacía para no duplicar datos.

use std::path::Path;

use rusqlite::{Connection, params};

/// Error de importación.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// Archivo inexistente.
    #[error("No existe el archivo: {0}")]
    NoExiste(String),
    /// Archivo sin tablas reconocibles.
    #[error("El archivo no parece una base de MiFutbolC (faltan tablas camiseta/partido).")]
    EsquemaInvalido,
    /// La base destino ya tiene datos.
    #[error("La base actual ya tiene datos; importe sobre una base vacía.")]
    DestinoConDatos,
    /// Error de SQLite.
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Error de E/S.
    #[error("E/S: {0}")]
    Io(String),
}

/// Resultado de la importación.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportResumen {
    /// Filas copiadas por tabla.
    pub tablas: Vec<(String, usize)>,
    /// Advertencias (por ejemplo, claves foráneas inválidas).
    pub advertencias: Vec<String>,
}

impl ImportResumen {
    /// Total de filas importadas.
    #[must_use]
    pub fn total(&self) -> usize {
        self.tablas.iter().map(|(_, filas)| filas).sum()
    }
}

/// Tablas que se importan, en un orden seguro con FKs desactivadas.
const TABLAS: &[&str] = &[
    "camiseta",
    "botin",
    "cancha",
    "cancha_tipo",
    "cancha_superficie",
    "partido",
    "partido_meta",
    "partido_tag",
    "equipo",
    "jugador",
    "lesion",
    "financiamiento",
    "presupuesto_mensual",
    "carrera_identidad",
    "carrera_partido_hito",
    "carrera_resumen_narrativo",
    "entrenamiento_plan",
    "progresion_jugador",
    "torneo",
    "partido_torneo",
    "equipo_historial",
    "temporada",
    // Módulos portados de MiFutbolC (mismas tablas y columnas); después de
    // camiseta y partido por sus claves foráneas.
    "bienestar_habito",
    "bienestar_entrenamiento",
    "bienestar_comida",
    "bienestar_sesion_mental",
    "bienestar_salud",
    "bienestar_recomendacion",
    "bienestar_ejercicio",
    "bienestar_entrenamiento_ejercicio",
    "bienestar_dia_nutricional",
    "bienestar_objetivo",
    "bienestar_plan_entrenamiento",
    "bienestar_control_medico",
    "bienestar_estudio_archivo",
    "bienestar_menu_imagen",
    "coleccion",
    "inventario_item",
    "coleccion_inventario",
    "consejos_historial",
    "ia_config",
];

/// Tablas con filas sembradas por las migraciones: se vacían antes de copiar
/// para que los datos legacy las reemplacen en vez de chocar (UNIQUE / id = 1).
const SEMBRADAS: &[&str] = &[
    "carrera_identidad",
    "bienestar_salud",
    "coleccion",
    "ia_config",
];

/// Importa una base legacy sobre una conexión destino vacía.
pub fn importar(conn: &mut Connection, ruta: &Path) -> Result<ImportResumen, ImportError> {
    if !ruta.is_file() {
        return Err(ImportError::NoExiste(ruta.display().to_string()));
    }

    let destino_vacio: i64 = conn.query_row(
        "SELECT (SELECT COUNT(*) FROM camiseta) + (SELECT COUNT(*) FROM partido)
              + (SELECT COUNT(*) FROM cancha) + (SELECT COUNT(*) FROM botin)",
        [],
        |row| row.get(0),
    )?;
    if destino_vacio > 0 {
        return Err(ImportError::DestinoConDatos);
    }

    // Solo lectura: la base de MiFutbolC nunca se modifica al importar.
    conn.execute(
        "ATTACH DATABASE ?1 AS legacy",
        params![uri_solo_lectura(ruta)],
    )?;

    let resultado = importar_adjunta(conn);
    let _ = conn.execute_batch("DETACH DATABASE legacy");
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    let mut resumen = resultado?;
    let fk_invalidas: i64 =
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    if fk_invalidas > 0 {
        resumen.advertencias.push(format!(
            "{fk_invalidas} claves foráneas inválidas tras importar"
        ));
    }
    Ok(resumen)
}

/// Copia a `destino` (el `Imagenes/` de MiFutbolR) los archivos de las carpetas
/// `Imagenes/` e `images/` de MiFutbolC, buscadas junto a la base legacy y en la
/// carpeta de arriba. El origen solo se lee; un archivo que ya existe con el
/// mismo nombre se saltea (las rutas guardadas son `Imagenes/<nombre>`).
/// Devuelve cuántos archivos se copiaron.
pub fn copiar_imagenes(ruta_db: &Path, destino: &Path) -> Result<usize, ImportError> {
    let io = |error: std::io::Error| ImportError::Io(error.to_string());
    let base = ruta_db.parent().unwrap_or(Path::new(""));
    let mut copiados = 0;
    for dir in [Some(base), base.parent()].into_iter().flatten() {
        for carpeta in ["Imagenes", "images"] {
            let Ok(entradas) = std::fs::read_dir(dir.join(carpeta)) else {
                continue;
            };
            for entrada in entradas.flatten() {
                let origen = entrada.path();
                let objetivo = destino.join(entrada.file_name());
                if !origen.is_file() || objetivo.exists() {
                    continue;
                }
                std::fs::create_dir_all(destino).map_err(io)?;
                std::fs::copy(&origen, &objetivo).map_err(io)?;
                copiados += 1;
            }
        }
    }
    Ok(copiados)
}

/// URI `file:` de solo lectura (rusqlite abre las conexiones con URIs habilitadas).
fn uri_solo_lectura(ruta: &Path) -> String {
    let ruta = std::fs::canonicalize(ruta).unwrap_or_else(|_| ruta.to_path_buf());
    let texto = ruta.to_string_lossy().replace('\\', "/");
    // `canonicalize` en Windows devuelve `//?/C:/...`; SQLite espera `/C:/...`.
    let texto = texto.trim_start_matches("//?/");
    let mut uri = String::from("file:");
    if !texto.starts_with('/') {
        uri.push('/');
    }
    for caracter in texto.chars() {
        match caracter {
            '%' => uri.push_str("%25"),
            ' ' => uri.push_str("%20"),
            '?' => uri.push_str("%3f"),
            '#' => uri.push_str("%23"),
            otro => uri.push(otro),
        }
    }
    uri.push_str("?mode=ro");
    uri
}

fn importar_adjunta(conn: &mut Connection) -> Result<ImportResumen, ImportError> {
    let tablas_legacy: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM legacy.sqlite_master WHERE type = 'table' ORDER BY name")?;
        stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    if !tablas_legacy.iter().any(|tabla| tabla == "camiseta")
        || !tablas_legacy.iter().any(|tabla| tabla == "partido")
    {
        return Err(ImportError::EsquemaInvalido);
    }

    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let mut resumen = ImportResumen::default();

    let tx = conn.transaction()?;
    // Sin historial de deshacer durante la copia masiva: cada fila dispararía un
    // trigger con json_object de toda la fila (~5x más lento). Va dentro de la
    // transacción, así un error deja la bandera como estaba.
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])?;
    for tabla in TABLAS {
        if !tablas_legacy.iter().any(|legacy| legacy == tabla) {
            continue;
        }
        let destino = columnas(&tx, tabla, false)?;
        let origen = columnas(&tx, tabla, true)?;
        if destino.is_empty() || origen.is_empty() {
            continue;
        }
        let (columnas_destino, columnas_origen) = mapear(tabla, &destino, &origen);
        if columnas_destino.is_empty() {
            continue;
        }
        // Las tablas singleton sembradas se reemplazan con los datos legacy.
        if SEMBRADAS.contains(tabla) {
            let filas_legacy: i64 = tx.query_row(
                &format!("SELECT COUNT(*) FROM legacy.\"{tabla}\""),
                [],
                |row| row.get(0),
            )?;
            if filas_legacy > 0 {
                tx.execute(&format!("DELETE FROM main.\"{tabla}\""), [])?;
            }
        }
        let lista_destino = columnas_destino
            .iter()
            .map(|columna| format!("\"{columna}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let lista_origen = columnas_origen.to_vec().join(", ");
        let sql = format!(
            "INSERT INTO main.\"{tabla}\" ({lista_destino}) SELECT {lista_origen} FROM legacy.\"{tabla}\""
        );
        tx.execute(&sql, [])?;
        let filas = tx.changes() as usize;
        resumen.tablas.push((tabla.to_string(), filas));
    }

    // Normalizaciones posteriores.
    tx.execute(
        "UPDATE partido SET fecha_hora = fecha_hora || ':00'
          WHERE length(fecha_hora) = 16 AND substr(fecha_hora, 11, 1) = ' '",
        [],
    )?;
    tx.execute("UPDATE partido SET mes_anio = substr(fecha_hora, 1, 7)", [])?;
    tx.execute(
        "UPDATE lesion
            SET fecha = substr(fecha, 7, 4) || '-' || substr(fecha, 4, 2) || '-' || substr(fecha, 1, 2)
                        || substr(fecha, 11)
          WHERE fecha LIKE '__/__/____%'",
        [],
    )?;
    tx.execute(
        "UPDATE lesion SET fecha = fecha || ':00'
          WHERE length(fecha) = 16 AND substr(fecha, 11, 1) = ' '",
        [],
    )?;

    preferencias_legacy(&tx, &tablas_legacy)?;
    poblar_canchas_hijas(&tx)?;
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(resumen)
}

fn poblar_canchas_hijas(conn: &Connection) -> Result<(), rusqlite::Error> {
    use crate::domain::cancha_codes;

    let canchas: Vec<(i64, i64, i64)> = {
        let mut stmt = conn.prepare(
            "SELECT id, tipo_cancha_codigo, superficie_codigo FROM main.cancha ORDER BY id",
        )?;
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?
    };
    conn.execute("DELETE FROM main.cancha_tipo", [])?;
    conn.execute("DELETE FROM main.cancha_superficie", [])?;
    for (cancha_id, tipos_codigo, superficie_codigo) in canchas {
        for tipo in cancha_codes::decode_tipos(tipos_codigo) {
            conn.execute(
                "INSERT OR IGNORE INTO main.cancha_tipo (cancha_id, tipo_codigo) VALUES (?1, ?2)",
                params![cancha_id, tipo.as_i64()],
            )?;
        }
        for superficie in cancha_codes::decode_superficies(superficie_codigo) {
            conn.execute(
                "INSERT OR IGNORE INTO main.cancha_superficie (cancha_id, superficie_codigo) VALUES (?1, ?2)",
                params![cancha_id, superficie.as_i64()],
            )?;
        }
    }
    Ok(())
}

fn preferencias_legacy(conn: &Connection, tablas_legacy: &[String]) -> Result<(), rusqlite::Error> {
    if !tablas_legacy.iter().any(|tabla| tabla == "settings") {
        return Ok(());
    }
    let destino = columnas(conn, "settings", false)?;
    let origen = columnas(conn, "settings", true)?;
    let asignaciones: Vec<String> = [
        "theme",
        "language",
        "mode",
        "text_size",
        "image_viewer",
        "botin_predeterminado",
    ]
    .iter()
    .filter(|campo| {
        destino.iter().any(|columna| columna == *campo)
            && origen.iter().any(|columna| columna == *campo)
    })
    .map(|campo| format!("\"{campo}\" = (SELECT \"{campo}\" FROM legacy.settings WHERE id = 1)"))
    .collect();
    if asignaciones.is_empty() {
        return Ok(());
    }
    conn.execute(
        &format!(
            "UPDATE main.settings SET {} WHERE id = 1",
            asignaciones.join(", ")
        ),
        [],
    )?;
    Ok(())
}

fn columnas(conn: &Connection, tabla: &str, legacy: bool) -> Result<Vec<String>, rusqlite::Error> {
    let sql = if legacy {
        format!("PRAGMA legacy.table_info(\"{tabla}\")")
    } else {
        format!("PRAGMA main.table_info(\"{tabla}\")")
    };
    let mut stmt = conn.prepare(&sql)?;
    let columnas = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(columnas)
}

/// Devuelve las columnas destino y sus expresiones de origen.
fn mapear(tabla: &str, destino: &[String], origen: &[String]) -> (Vec<String>, Vec<String>) {
    let tiene = |nombre: &str| origen.iter().any(|columna| columna == nombre);
    let mut columnas_destino = Vec::new();
    let mut expresiones = Vec::new();

    for columna in destino {
        let expresion = match (tabla, columna.as_str()) {
            ("financiamiento", "monto_centavos") => {
                if tiene("monto_centavos") {
                    Some("\"monto_centavos\"".to_string())
                } else if tiene("monto") {
                    Some("CAST(ROUND(\"monto\" * 100) AS INTEGER)".to_string())
                } else {
                    Some("0".to_string())
                }
            }
            ("partido", "precio_centavos") => {
                if tiene("precio_centavos") {
                    Some("\"precio_centavos\"".to_string())
                } else if tiene("precio") {
                    Some("CAST(ROUND(\"precio\" * 100) AS INTEGER)".to_string())
                } else {
                    Some("0".to_string())
                }
            }
            ("equipo", "tipo_futbol") if tiene("tipo_futbol") => Some(
                "CASE \"tipo_futbol\" WHEN 5 THEN 0 WHEN 7 THEN 1 WHEN 8 THEN 2 WHEN 11 THEN 3 ELSE \"tipo_futbol\" END".to_string(),
            ),
            ("equipo", "partido_id") if tiene("partido_id") => Some(
                "CASE WHEN \"partido_id\" IS NULL OR \"partido_id\" <= 0 THEN NULL ELSE \"partido_id\" END"
                    .to_string(),
            ),
            ("partido", "formato_partido") if tiene("formato_partido") => Some(
                "replace(replace(\"formato_partido\", 'Fútbol ', 'F'), 'Futbol ', 'F')".to_string(),
            ),
            // MiFutbolC guarda 0/NULL como "sin tarjeta"; aquí solo valen 1..=3.
            ("partido", "tarjeta") if tiene("tarjeta") => Some(
                "CASE WHEN \"tarjeta\" IN (2, 3) THEN \"tarjeta\" ELSE 1 END".to_string(),
            ),
            ("cancha", "cantidad_canchas") if tiene("cantidad_canchas") => Some(
                "CASE WHEN \"cantidad_canchas\" IS NULL OR \"cantidad_canchas\" < 1 THEN 1 ELSE \"cantidad_canchas\" END"
                    .to_string(),
            ),
            ("lesion", "estado") if tiene("estado") => Some(
                "UPPER(\"estado\")".to_string(),
            ),
            ("lesion", "fecha") if tiene("fecha") => Some(
                "CASE WHEN \"fecha\" LIKE '__/__/____%'
                      THEN substr(\"fecha\", 7, 4) || '-' || substr(\"fecha\", 4, 2) || '-'
                           || substr(\"fecha\", 1, 2) || substr(\"fecha\", 11)
                      ELSE \"fecha\" END"
                    .to_string(),
            ),
            _ => {
                if tiene(columna) {
                    Some(format!("\"{columna}\""))
                } else {
                    None
                }
            }
        };
        if let Some(expresion) = expresion {
            columnas_destino.push(columna.clone());
            expresiones.push(expresion);
        }
    }
    (columnas_destino, expresiones)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn crear_legacy(ruta: &Path) {
        let legacy = Connection::open(ruta).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE camiseta (id INTEGER PRIMARY KEY, nombre TEXT, activa INTEGER, sorteada INTEGER);
                 CREATE TABLE botin (id INTEGER PRIMARY KEY, nombre TEXT, activa INTEGER, sorteada INTEGER);
                 CREATE TABLE cancha (id INTEGER PRIMARY KEY, nombre TEXT, localidad TEXT, activa INTEGER);
                 CREATE TABLE partido (
                    id INTEGER PRIMARY KEY, cancha_id INTEGER, camiseta_id INTEGER, botin_id INTEGER,
                    fecha_hora TEXT, precio INTEGER, goles INTEGER, asistencias INTEGER,
                    resultado INTEGER, rendimiento_general INTEGER, clima INTEGER, dia INTEGER,
                    tipo_partido INTEGER, formato_partido TEXT, tarjeta INTEGER
                 );
                 CREATE TABLE lesion (
                    id INTEGER PRIMARY KEY, jugador TEXT, tipo TEXT, descripcion TEXT, fecha TEXT,
                    camiseta_id INTEGER, estado TEXT, partido_id INTEGER
                 );
                 CREATE TABLE financiamiento (
                    id INTEGER PRIMARY KEY, fecha TEXT, tipo INTEGER, categoria INTEGER,
                    descripcion TEXT, monto REAL
                 );
                 INSERT INTO camiseta VALUES (1, 'Titular', 1, 0);
                 INSERT INTO botin VALUES (1, 'Predator', 1, 0);
                 INSERT INTO cancha VALUES (1, 'Sur', 'Quilmes', 1);
                 INSERT INTO partido VALUES (10, 1, 1, 1, '2026-03-01 20:00', 15000, 2, 1, 1, 8, 1, 2, 1, 'Futbol 7', 0);
                 INSERT INTO lesion VALUES (5, 'Ana', 'Esguince', 'Tobillo', '15/03/2026 18:30', 1, 'ACTIVA', 10);
                 CREATE TABLE partido_meta (partido_id INTEGER PRIMARY KEY, favorito INTEGER DEFAULT 0);
                 CREATE TABLE partido_tag (id INTEGER PRIMARY KEY AUTOINCREMENT, partido_id INTEGER NOT NULL, tag TEXT NOT NULL);
                 CREATE TABLE settings (id INTEGER PRIMARY KEY, theme INTEGER, language INTEGER, mode INTEGER, text_size INTEGER, image_viewer TEXT, botin_predeterminado INTEGER);
                 INSERT INTO partido_meta VALUES (10, 1);
                 INSERT INTO partido_tag (partido_id, tag) VALUES (10, 'Partidazo');
                 INSERT INTO settings VALUES (1, 4, 1, 2, 2, 'visor', 1);
                 INSERT INTO financiamiento VALUES (7, '2026-03-01', 1, 5, 'Alquiler', 15000.5);",
            )
            .unwrap();
    }

    #[test]
    fn copies_legacy_images_without_touching_the_source() {
        let dir = tempfile::tempdir().unwrap();
        let c = dir.path().join("MiFutbolC");
        std::fs::create_dir_all(c.join("images")).unwrap();
        std::fs::create_dir_all(c.join("Imagenes").join("sub")).unwrap();
        std::fs::write(c.join("images").join("camiseta_1.png"), b"png").unwrap();
        std::fs::write(c.join("Imagenes").join("estudio_5.pdf"), b"%PDF").unwrap();
        std::fs::write(c.join("Imagenes").join("repetida.jpg"), b"nueva").unwrap();
        let db = c.join("mifutbol.db");
        std::fs::write(&db, b"").unwrap();
        let destino = dir.path().join("Rust").join("Imagenes");
        std::fs::create_dir_all(&destino).unwrap();
        std::fs::write(destino.join("repetida.jpg"), b"vieja").unwrap();

        assert_eq!(copiar_imagenes(&db, &destino).unwrap(), 2);
        assert_eq!(
            std::fs::read(destino.join("camiseta_1.png")).unwrap(),
            b"png"
        );
        assert_eq!(
            std::fs::read(destino.join("estudio_5.pdf")).unwrap(),
            b"%PDF"
        );
        assert_eq!(
            std::fs::read(destino.join("repetida.jpg")).unwrap(),
            b"vieja"
        );
        assert!(!destino.join("sub").exists(), "solo archivos");
        // El origen queda intacto y repetir no copia nada.
        assert_eq!(
            std::fs::read(c.join("Imagenes").join("repetida.jpg")).unwrap(),
            b"nueva"
        );
        assert_eq!(copiar_imagenes(&db, &destino).unwrap(), 0);
    }

    #[test]
    fn rejects_empty_path_and_non_legacy() {
        let mut db = Database::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            importar(db.conn_mut(), &dir.path().join("nope.db")),
            Err(ImportError::NoExiste(_))
        ));

        let vacia = dir.path().join("vacia.db");
        Connection::open(&vacia).unwrap();
        assert!(matches!(
            importar(db.conn_mut(), &vacia),
            Err(ImportError::EsquemaInvalido)
        ));
    }

    #[test]
    fn imports_and_converts_legacy_data() {
        let dir = tempfile::tempdir().unwrap();
        // Espacios y % en la ruta, como "Lenguaje C" en la carpeta real.
        let carpeta = dir.path().join("Mi Futbol 100%");
        std::fs::create_dir_all(&carpeta).unwrap();
        let ruta = carpeta.join("legacy.db");
        crear_legacy(&ruta);
        let original = std::fs::read(&ruta).unwrap();

        let mut db = Database::open_in_memory().unwrap();
        let resumen = importar(db.conn_mut(), &ruta).unwrap();
        assert!(resumen.total() >= 6);
        // La copia masiva no llena el historial de deshacer y lo deja reactivado.
        let (historial, suspendido): (i64, i64) = db
            .conn()
            .query_row(
                "SELECT (SELECT COUNT(*) FROM undo_historial), suspendido FROM undo_control",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((historial, suspendido), (0, 0));
        assert_eq!(
            std::fs::read(&ruta).unwrap(),
            original,
            "el origen no cambia"
        );
        let solo_lectura = Connection::open_in_memory().unwrap();
        solo_lectura
            .execute("ATTACH DATABASE ?1 AS l", params![uri_solo_lectura(&ruta)])
            .unwrap();
        assert!(
            solo_lectura
                .execute("INSERT INTO l.camiseta (nombre) VALUES ('x')", [])
                .is_err(),
            "la base adjunta es de solo lectura"
        );
        assert!(
            resumen.advertencias.is_empty(),
            "{:?}",
            resumen.advertencias
        );

        let precio: i64 = db
            .conn()
            .query_row(
                "SELECT precio_centavos FROM partido WHERE id = 10",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(precio, 1_500_000);

        let tarjeta: i64 = db
            .conn()
            .query_row("SELECT tarjeta FROM partido WHERE id = 10", [], |row| row.get(0))
            .unwrap();
        assert_eq!(tarjeta, 1, "tarjeta 0 de MiFutbolC es 'sin tarjeta'");

        let fecha: String = db
            .conn()
            .query_row("SELECT fecha_hora FROM partido WHERE id = 10", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(fecha, "2026-03-01 20:00:00");

        let mes: String = db
            .conn()
            .query_row("SELECT mes_anio FROM partido WHERE id = 10", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(mes, "2026-03");

        let lesion: String = db
            .conn()
            .query_row("SELECT fecha FROM lesion WHERE id = 5", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(lesion, "2026-03-15 18:30:00");

        let monto: i64 = db
            .conn()
            .query_row(
                "SELECT monto_centavos FROM financiamiento WHERE id = 7",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(monto, 1_500_050);
    }

    #[test]
    fn rejects_non_empty_destination() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("legacy.db");
        crear_legacy(&ruta);
        let mut db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Local')", [])
            .unwrap();
        assert!(matches!(
            importar(db.conn_mut(), &ruta),
            Err(ImportError::DestinoConDatos)
        ));
    }

    #[test]
    fn imports_ported_modules_and_replaces_seeded_rows() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("legacy.db");
        crear_legacy(&ruta);
        // Tablas tal como las crea MiFutbolC (db.c).
        Connection::open(&ruta)
            .unwrap()
            .execute_batch(
                "CREATE TABLE bienestar_habito (id INTEGER PRIMARY KEY AUTOINCREMENT, fecha TEXT NOT NULL, dormi_bien INTEGER DEFAULT 0, hidratacion INTEGER DEFAULT 0, alcohol INTEGER DEFAULT 0, estado_animico TEXT DEFAULT '', nervios INTEGER DEFAULT 0, confianza INTEGER DEFAULT 0, motivacion INTEGER DEFAULT 0, notas TEXT DEFAULT '', tipo_diario TEXT DEFAULT '');
                 CREATE TABLE bienestar_salud (id INTEGER PRIMARY KEY CHECK (id = 1), altura_cm REAL DEFAULT NULL, peso_kg REAL DEFAULT NULL, tipo_sangre TEXT DEFAULT '', ultima_revision TEXT DEFAULT '', medidas TEXT DEFAULT '', notas TEXT DEFAULT '');
                 CREATE TABLE coleccion (id INTEGER PRIMARY KEY AUTOINCREMENT, nombre TEXT NOT NULL UNIQUE, descripcion TEXT DEFAULT '', fecha_creacion TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
                 CREATE TABLE inventario_item (id INTEGER PRIMARY KEY AUTOINCREMENT, tipo INTEGER NOT NULL, nombre TEXT DEFAULT '', estado INTEGER NOT NULL DEFAULT 0, valor REAL DEFAULT 0, fecha_compra TEXT DEFAULT '', camiseta_id INTEGER UNIQUE);
                 CREATE TABLE coleccion_inventario (coleccion_id INTEGER NOT NULL, inventario_id INTEGER NOT NULL, fecha_asociacion TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY(coleccion_id, inventario_id));
                 CREATE TABLE ia_config (id INTEGER PRIMARY KEY, nivel_intervencion INTEGER NOT NULL DEFAULT 2, ultima_alerta INTEGER NOT NULL DEFAULT 0, ultimo_riesgo REAL NOT NULL DEFAULT 0);
                 INSERT INTO bienestar_habito (fecha, dormi_bien, confianza) VALUES ('2026-03-02', 1, 8);
                 INSERT INTO bienestar_salud (id, altura_cm, peso_kg, tipo_sangre) VALUES (1, 178, 74.5, 'O+');
                 INSERT INTO coleccion (id, nombre) VALUES (1, 'Equipamiento actual'), (2, 'Historico'), (3, 'Vendidos'), (4, 'Retro');
                 INSERT INTO inventario_item (id, tipo, nombre, estado, valor, camiseta_id) VALUES (1, 1, '', 1, 0, 1), (2, 3, 'Canilleras', 0, 2500, NULL);
                 INSERT INTO coleccion_inventario (coleccion_id, inventario_id) VALUES (4, 1), (1, 2);
                 INSERT INTO ia_config (id, nivel_intervencion) VALUES (1, 3);",
            )
            .unwrap();

        let mut db = Database::open_in_memory().unwrap();
        let resumen = importar(db.conn_mut(), &ruta).unwrap();
        assert!(
            resumen.advertencias.is_empty(),
            "{:?}",
            resumen.advertencias
        );
        let consulta =
            |sql: &str| -> i64 { db.conn().query_row(sql, [], |row| row.get(0)).unwrap() };
        assert_eq!(
            consulta("SELECT COUNT(*) FROM bienestar_habito WHERE confianza = 8"),
            1
        );
        assert_eq!(
            consulta("SELECT COUNT(*) FROM bienestar_salud WHERE tipo_sangre = 'O+'"),
            1
        );
        // Las 3 colecciones sembradas se reemplazan por las 4 del origen, sin duplicar.
        assert_eq!(consulta("SELECT COUNT(*) FROM coleccion"), 4);
        assert_eq!(consulta("SELECT COUNT(*) FROM coleccion_inventario"), 2);
        assert_eq!(
            consulta("SELECT camiseta_id FROM inventario_item WHERE id = 1"),
            1
        );
        assert_eq!(
            consulta("SELECT nivel_intervencion FROM ia_config WHERE id = 1"),
            3
        );
    }

    #[test]
    fn imports_remaining_wellness_tables() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("legacy.db");
        crear_legacy(&ruta);
        // SQL exacto de db.c (MiFutbolC).
        Connection::open(&ruta)
            .unwrap()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS bienestar_objetivo ( id INTEGER PRIMARY KEY AUTOINCREMENT, nombre TEXT NOT NULL, fecha_inicio TEXT NOT NULL, fecha_fin TEXT NOT NULL, estado TEXT NOT NULL DEFAULT 'Activo', notas TEXT DEFAULT '');
                 CREATE TABLE IF NOT EXISTS bienestar_plan_entrenamiento ( id INTEGER PRIMARY KEY AUTOINCREMENT, objetivo_id INTEGER NOT NULL, frecuencia_semanal INTEGER NOT NULL, rutina_semanal TEXT NOT NULL, notas TEXT DEFAULT '', FOREIGN KEY(objetivo_id) REFERENCES bienestar_objetivo(id));
                 CREATE TABLE IF NOT EXISTS bienestar_entrenamiento ( id INTEGER PRIMARY KEY AUTOINCREMENT, fecha TEXT NOT NULL, tipo TEXT NOT NULL, duracion_min INTEGER NOT NULL, intensidad INTEGER NOT NULL, omitido INTEGER DEFAULT 0, notas TEXT DEFAULT '');
                 CREATE TABLE IF NOT EXISTS bienestar_ejercicio ( id INTEGER PRIMARY KEY AUTOINCREMENT, nombre TEXT NOT NULL, grupo_muscular TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS bienestar_entrenamiento_ejercicio ( id INTEGER PRIMARY KEY AUTOINCREMENT, entrenamiento_id INTEGER NOT NULL, ejercicio_id INTEGER NOT NULL, series INTEGER DEFAULT 0, repeticiones INTEGER DEFAULT 0, tiempo_min INTEGER DEFAULT 0, FOREIGN KEY(entrenamiento_id) REFERENCES bienestar_entrenamiento(id), FOREIGN KEY(ejercicio_id) REFERENCES bienestar_ejercicio(id));
                 CREATE TABLE IF NOT EXISTS bienestar_dia_nutricional ( id INTEGER PRIMARY KEY AUTOINCREMENT, fecha TEXT NOT NULL UNIQUE, hidratacion TEXT NOT NULL, alcohol INTEGER DEFAULT 0, peso_corporal REAL DEFAULT NULL, notas TEXT DEFAULT '');
                 CREATE TABLE IF NOT EXISTS bienestar_control_medico ( id INTEGER PRIMARY KEY AUTOINCREMENT, fecha TEXT NOT NULL, tipo TEXT NOT NULL, profesional TEXT DEFAULT '', resultado TEXT DEFAULT '', notas TEXT DEFAULT '');
                 CREATE TABLE IF NOT EXISTS bienestar_estudio_archivo ( id INTEGER PRIMARY KEY AUTOINCREMENT, control_id INTEGER NOT NULL, nombre_original TEXT NOT NULL, ruta_archivo TEXT NOT NULL, tipo_archivo TEXT DEFAULT '', fecha_subida TEXT NOT NULL, FOREIGN KEY(control_id) REFERENCES bienestar_control_medico(id));
                 CREATE TABLE IF NOT EXISTS bienestar_menu_imagen ( menu_key TEXT PRIMARY KEY, imagen_ruta TEXT DEFAULT '');
                 INSERT INTO bienestar_objetivo VALUES (3, 'Correr 10k', '2026-01-01', '2026-06-01', 'Activo', '');
                 INSERT INTO bienestar_plan_entrenamiento VALUES (1, 3, 4, 'Lunes fuerza', '');
                 INSERT INTO bienestar_entrenamiento VALUES (7, '2026-03-02', 'Fuerza', 60, 7, 0, '');
                 INSERT INTO bienestar_ejercicio VALUES (2, 'Sentadilla', 'Piernas');
                 INSERT INTO bienestar_entrenamiento_ejercicio VALUES (1, 7, 2, 4, 12, 0);
                 INSERT INTO bienestar_dia_nutricional VALUES (1, '2026-03-02', 'Alta', 0, 74.5, '');
                 INSERT INTO bienestar_control_medico VALUES (5, '2026-02-10', 'Clínico', 'Dra. Pérez', 'OK', '');
                 INSERT INTO bienestar_estudio_archivo VALUES (1, 5, 'sangre.pdf', 'Imagenes/estudio_5_20260210.pdf', 'documento', '2026-02-10');
                 INSERT INTO bienestar_menu_imagen VALUES ('salud', 'imagenes/bienestar_salud_20260210.png');",
            )
            .unwrap();

        let mut db = Database::open_in_memory().unwrap();
        let resumen = importar(db.conn_mut(), &ruta).unwrap();
        assert!(
            resumen.advertencias.is_empty(),
            "{:?}",
            resumen.advertencias
        );
        for tabla in [
            "bienestar_objetivo",
            "bienestar_plan_entrenamiento",
            "bienestar_entrenamiento",
            "bienestar_ejercicio",
            "bienestar_entrenamiento_ejercicio",
            "bienestar_dia_nutricional",
            "bienestar_control_medico",
            "bienestar_estudio_archivo",
            "bienestar_menu_imagen",
        ] {
            assert!(
                resumen.tablas.contains(&(tabla.to_string(), 1)),
                "{tabla}: {:?}",
                resumen.tablas
            );
        }
        let plan: String = db
            .conn()
            .query_row(
                "SELECT o.nombre FROM bienestar_plan_entrenamiento p
                   JOIN bienestar_objetivo o ON o.id = p.objetivo_id",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(plan, "Correr 10k");
    }

    #[test]
    fn keeps_seeded_rows_when_legacy_table_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("legacy.db");
        crear_legacy(&ruta);
        Connection::open(&ruta)
            .unwrap()
            .execute_batch("CREATE TABLE coleccion (id INTEGER PRIMARY KEY, nombre TEXT NOT NULL UNIQUE, descripcion TEXT DEFAULT '', fecha_creacion TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);")
            .unwrap();
        let mut db = Database::open_in_memory().unwrap();
        importar(db.conn_mut(), &ruta).unwrap();
        let total: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM coleccion", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 3, "las colecciones por defecto siguen");
    }
}
