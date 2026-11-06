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

    conn.execute(
        "ATTACH DATABASE ?1 AS legacy",
        params![ruta.to_string_lossy()],
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
        if *tabla == "carrera_identidad" {
            tx.execute("DELETE FROM main.carrera_identidad", [])?;
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
                    tipo_partido INTEGER, formato_partido TEXT
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
                 INSERT INTO partido VALUES (10, 1, 1, 1, '2026-03-01 20:00', 15000, 2, 1, 1, 8, 1, 2, 1, 'Futbol 7');
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
        let ruta = dir.path().join("legacy.db");
        crear_legacy(&ruta);

        let mut db = Database::open_in_memory().unwrap();
        let resumen = importar(db.conn_mut(), &ruta).unwrap();
        assert!(resumen.total() >= 6);
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
}
