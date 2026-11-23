//! Importar camisetas, partidos y lesiones desde JSON, TXT, CSV y HTML (port
//! de `import.c`). Lee `Importaciones/<entidad>.<ext>` con los formatos que
//! escriben los exportadores de MiFutbolC.
//!
//! Todo va en una transacción con el deshacer suspendido. Una fila mal formada
//! se informa con su línea y no frena al resto; un error de SQLite deshace todo.
//! Las estadísticas por camiseta de C no se importan: iban a una tabla
//! `estadistica` que nada lee (MiFutbolR las calcula de los partidos).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::db::DbError;
use crate::services::mobile_service;

/// Tamaño máximo de un archivo de importación (el mismo límite que C).
const MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Formato del archivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    /// Array JSON de objetos.
    Json,
    /// Texto con separadores `|`.
    Txt,
    /// Valores separados por coma.
    Csv,
    /// Tabla HTML (`<tr><td>`).
    Html,
}

impl Formato {
    /// Extensión del archivo.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Txt => "txt",
            Self::Csv => "csv",
            Self::Html => "html",
        }
    }
}

/// Qué se importa, en orden: las camisetas antes que lo que las referencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entidad {
    /// `camisetas.<ext>`
    Camisetas,
    /// `partidos.<ext>`
    Partidos,
    /// `lesiones.<ext>`
    Lesiones,
}

impl Entidad {
    const ALL: [Self; 3] = [Self::Camisetas, Self::Partidos, Self::Lesiones];

    /// Nombre del archivo sin extensión.
    #[must_use]
    pub const fn nombre(self) -> &'static str {
        match self {
            Self::Camisetas => "camisetas",
            Self::Partidos => "partidos",
            Self::Lesiones => "lesiones",
        }
    }
}

/// Resultado de un archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resumen {
    /// Entidad importada.
    pub entidad: Entidad,
    /// Archivo leído.
    pub archivo: PathBuf,
    /// Filas nuevas.
    pub importados: usize,
    /// Filas válidas que no entraron: ya existían, la camiseta no existe o un
    /// valor está fuera de rango.
    pub omitidos: usize,
    /// Filas mal formadas ("línea 3: ...") o el archivo entero ilegible.
    pub errores: Vec<String>,
}

type Fila = Result<Value, String>;

/// Archivos de `dir` que existen para `formato`, en orden de importación.
#[must_use]
pub fn pendientes(dir: &Path, formato: Formato) -> Vec<(Entidad, PathBuf)> {
    Entidad::ALL
        .into_iter()
        .map(|entidad| {
            let archivo = dir.join(format!("{}.{}", entidad.nombre(), formato.extension()));
            (entidad, archivo)
        })
        .filter(|(_, archivo)| archivo.exists())
        .collect()
}

/// Importa los archivos de `dir` que existan para `formato`.
pub fn importar(
    conn: &mut Connection,
    dir: &Path,
    formato: Formato,
) -> Result<Vec<Resumen>, String> {
    let lecturas: Vec<_> = pendientes(dir, formato)
        .into_iter()
        .map(|(entidad, archivo)| {
            let filas = leer(&archivo).and_then(|texto| filas(entidad, formato, &texto));
            (entidad, archivo, filas)
        })
        .collect();
    if lecturas.is_empty() {
        return Err(format!(
            "No hay camisetas.{ext}, partidos.{ext} ni lesiones.{ext} en {}",
            dir.display(),
            ext = formato.extension()
        ));
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])
        .map_err(|e| e.to_string())?;
    let mut resumenes = Vec::new();
    for (entidad, archivo, filas) in lecturas {
        let mut resumen = Resumen {
            entidad,
            archivo,
            importados: 0,
            omitidos: 0,
            errores: Vec::new(),
        };
        match filas {
            Err(error) => resumen.errores.push(error),
            Ok(filas) => {
                for fila in filas {
                    match fila {
                        Ok(item) => {
                            if insertar(&tx, entidad, &item).map_err(|e| e.to_string())? {
                                resumen.importados += 1;
                            } else {
                                resumen.omitidos += 1;
                            }
                        }
                        Err(error) => resumen.errores.push(error),
                    }
                }
            }
        }
        resumenes.push(resumen);
    }
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(resumenes)
}

fn leer(archivo: &Path) -> Result<String, String> {
    let error = |e: std::io::Error| format!("{}: {e}", archivo.display());
    if std::fs::metadata(archivo).map_err(error)?.len() > MAX_BYTES {
        return Err("el archivo supera los 10 MB".to_string());
    }
    let bytes = std::fs::read(archivo).map_err(error)?;
    let texto = String::from_utf8(bytes).map_err(|_| "el archivo no es UTF-8".to_string())?;
    Ok(texto.trim_start_matches('\u{feff}').to_string())
}

fn filas(entidad: Entidad, formato: Formato, texto: &str) -> Result<Vec<Fila>, String> {
    Ok(match formato {
        Formato::Json => filas_json(texto)?,
        Formato::Html => filas_html(texto)
            .into_iter()
            .enumerate()
            .map(|(indice, celdas)| {
                de_celdas(entidad, &celdas, ",").map_err(|e| format!("fila {}: {e}", indice + 1))
            })
            .collect(),
        Formato::Csv | Formato::Txt => texto
            .lines()
            .enumerate()
            // La primera línea es la cabecera, como en C.
            .skip(1)
            .filter(|(_, linea)| !linea.trim().is_empty())
            // TXT de camisetas de C: "ID: 1 - Nombre: X" y debajo estadísticas con sangría.
            .filter(|(_, linea)| {
                !(formato == Formato::Txt && entidad == Entidad::Camisetas)
                    || !linea.starts_with(char::is_whitespace)
            })
            .map(|(indice, linea)| {
                let fila = if formato == Formato::Csv {
                    de_celdas(entidad, &campos_csv(linea), ",")
                } else {
                    de_txt(entidad, linea)
                };
                fila.map_err(|e| format!("línea {}: {e}", indice + 1))
            })
            .collect(),
    })
}

fn filas_json(texto: &str) -> Result<Vec<Fila>, String> {
    let valor: Value = serde_json::from_str(texto).map_err(|e| format!("JSON inválido: {e}"))?;
    let Value::Array(items) = valor else {
        return Err("el JSON debe ser un array".to_string());
    };
    Ok(items
        .into_iter()
        .enumerate()
        .map(|(indice, item)| {
            if item.is_object() {
                Ok(item)
            } else {
                Err(format!("elemento {}: no es un objeto", indice + 1))
            }
        })
        .collect())
}

/// Campos de una línea CSV; acepta comillas con `""` como en `export_service`.
fn campos_csv(linea: &str) -> Vec<String> {
    let mut campos = Vec::new();
    let mut actual = String::new();
    let mut comillas = false;
    let mut chars = linea.trim_end_matches('\r').chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if comillas && chars.peek() == Some(&'"') => {
                chars.next();
                actual.push('"');
            }
            '"' => comillas = !comillas,
            ',' if !comillas => campos.push(std::mem::take(&mut actual)),
            _ => actual.push(c),
        }
    }
    campos.push(actual);
    campos.into_iter().map(|c| c.trim().to_string()).collect()
}

/// Celdas `<td>` de cada `<tr>` con texto plano: sin etiquetas internas (el
/// resultado de C va en un `<span>`) y con las entidades decodificadas. Las
/// filas sin `<td>` (cabeceras) no cuentan.
fn filas_html(texto: &str) -> Vec<Vec<String>> {
    let minus = texto.to_ascii_lowercase();
    let mut filas = Vec::new();
    for (inicio, _) in minus.match_indices("<tr") {
        let fin = minus[inicio + 3..]
            .find("<tr")
            .map_or(minus.len(), |p| p + inicio + 3);
        let fila = &minus[inicio..fin];
        let mut celdas = Vec::new();
        let mut desde = 0;
        while let Some(td) = fila[desde..].find("<td") {
            let Some(abre) = fila[desde + td..].find('>') else {
                break;
            };
            let contenido = desde + td + abre + 1;
            let cierre = fila[contenido..]
                .find("</td")
                .map_or(fila.len(), |p| p + contenido);
            celdas.push(texto_html(&texto[inicio + contenido..inicio + cierre]));
            desde = cierre;
        }
        if !celdas.is_empty() {
            filas.push(celdas);
        }
    }
    filas
}

fn texto_html(html: &str) -> String {
    let mut plano = String::new();
    let mut en_etiqueta = false;
    for c in html.chars() {
        match c {
            '<' => en_etiqueta = true,
            '>' => en_etiqueta = false,
            _ if !en_etiqueta => plano.push(c),
            _ => {}
        }
    }
    // C escapaba al exportar y no decodificaba al importar ("&amp;" quedaba literal).
    plano
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .trim()
        .to_string()
}

fn entero(valor: &str, campo: &str) -> Result<i64, String> {
    valor
        .trim()
        .parse()
        .map_err(|_| format!("«{}» no es un número ({campo})", valor.trim()))
}

fn no_vacio<'a>(valor: &'a str, campo: &str) -> Result<&'a str, String> {
    let valor = valor.trim();
    if valor.is_empty() {
        Err(format!("falta {campo}"))
    } else {
        Ok(valor)
    }
}

/// CSV y HTML comparten columnas; `sep` une la descripción de una lesión si
/// traía el separador adentro.
fn de_celdas(entidad: Entidad, c: &[String], sep: &str) -> Fila {
    match entidad {
        Entidad::Camisetas => {
            if c.len() < 2 {
                return Err("se esperaban id y nombre".to_string());
            }
            Ok(json!({"id": entero(&c[0], "id")?, "nombre": no_vacio(&c[1], "el nombre")?}))
        }
        Entidad::Partidos => {
            if c.len() < 11 {
                return Err(format!("se esperaban 12 columnas y hay {}", c.len()));
            }
            // C agrega "Atajaste" al final al exportar y no la leía al importar:
            // terminaba pegada al comentario.
            let fin = if c.len() >= 13 && es_atajaste(&c[c.len() - 1]) {
                c.len() - 1
            } else {
                c.len()
            };
            Ok(json!({
                "cancha": no_vacio(&c[0], "la cancha")?,
                "fecha": no_vacio(&c[1], "la fecha")?,
                "goles": entero(&c[2], "goles")?,
                "asistencias": entero(&c[3], "asistencias")?,
                "camiseta": no_vacio(&c[4], "la camiseta")?,
                "resultado": c[5].trim(),
                "clima": c[6].trim(),
                "dia": c[7].trim(),
                "rendimiento_general": entero(&c[8], "rendimiento")?,
                "cansancio": entero(&c[9], "cansancio")?,
                "estado_animo": entero(&c[10], "estado de ánimo")?,
                "comentario_personal": c.get(11..fin).map_or(String::new(), |r| r.join(",")),
            }))
        }
        Entidad::Lesiones => {
            if c.len() < 5 {
                return Err("se esperaban id, jugador, tipo, descripción y fecha".to_string());
            }
            Ok(json!({
                "id": entero(&c[0], "id")?,
                "jugador": no_vacio(&c[1], "el jugador")?,
                "tipo": no_vacio(&c[2], "el tipo")?,
                "descripcion": c[3..c.len() - 1].join(sep).trim(),
                "fecha": no_vacio(&c[c.len() - 1], "la fecha")?,
            }))
        }
    }
}

fn es_atajaste(valor: &str) -> bool {
    matches!(valor.trim(), "-" | "SI" | "NO") || valor.trim().parse::<i64>().is_ok()
}

/// Texto `inicio…fin` dentro del bloque "Res:X Cli:Y Dia:Z RG:n Can:n EA:n".
fn etiqueta<'a>(meta: &'a str, inicio: &str, fin: Option<&str>) -> Result<&'a str, String> {
    let desde = meta
        .find(inicio)
        .ok_or_else(|| format!("falta {}", inicio.trim_end_matches(':')))?
        + inicio.len();
    let resto = &meta[desde..];
    let hasta = fin.and_then(|f| resto.find(f)).unwrap_or(resto.len());
    Ok(resto[..hasta].trim())
}

fn de_txt(entidad: Entidad, linea: &str) -> Fila {
    match entidad {
        // "1 - Azul" (formato que leía C) o "ID: 1 - Nombre: Azul" (el que escribía).
        Entidad::Camisetas => {
            let linea = linea.trim().trim_start_matches("ID:");
            let (id, nombre) = linea
                .split_once('-')
                .ok_or_else(|| "se esperaba «id - nombre»".to_string())?;
            let nombre = nombre.trim().trim_start_matches("Nombre:");
            let nombre = nombre.split(" | ").next().unwrap_or("");
            de_celdas(entidad, &[id.to_string(), nombre.to_string()], ",")
        }
        // "CANCHA | FECHA | G:g A:a | CAMISETA | Res:.. Cli:.. Dia:.. RG:n Can:n EA:n [| Atajaste:X] | Comentario"
        Entidad::Partidos => {
            let partes: Vec<&str> = linea.splitn(6, '|').map(str::trim).collect();
            if partes.len() < 5 {
                return Err("se esperaban al menos 5 bloques separados por |".to_string());
            }
            let (goles, asistencias) = partes[2]
                .strip_prefix("G:")
                .and_then(|resto| resto.split_once("A:"))
                .ok_or_else(|| "se esperaba «G:goles A:asistencias»".to_string())?;
            let mut comentario = partes.get(5).copied().unwrap_or("");
            if comentario.starts_with("Atajaste:") {
                comentario = comentario.split_once('|').map_or("", |(_, r)| r.trim());
            }
            let meta = partes[4];
            let celdas = [
                partes[0],
                partes[1],
                goles,
                asistencias,
                partes[3],
                etiqueta(meta, "Res:", Some(" Cli:"))?,
                etiqueta(meta, "Cli:", Some(" Dia:"))?,
                etiqueta(meta, "Dia:", Some(" RG:"))?,
                etiqueta(meta, "RG:", Some(" Can:"))?,
                etiqueta(meta, "Can:", Some(" EA:"))?,
                etiqueta(meta, "EA:", None)?,
                comentario,
            ];
            de_celdas(entidad, &celdas.map(str::to_string), ",")
        }
        // "ID - JUGADOR | TIPO | DESCRIPCION | FECHA"
        Entidad::Lesiones => {
            let (id, resto) = linea.split_once('-').ok_or_else(|| {
                "se esperaba «id - jugador | tipo | descripción | fecha»".to_string()
            })?;
            let mut celdas = vec![id.to_string()];
            celdas.extend(resto.split('|').map(str::to_string));
            de_celdas(entidad, &celdas, "|")
        }
    }
}

fn insertar(conn: &Connection, entidad: Entidad, item: &Value) -> Result<bool, DbError> {
    match entidad {
        Entidad::Camisetas => insertar_camiseta(conn, item),
        Entidad::Partidos => mobile_service::importar_item(conn, item),
        Entidad::Lesiones => insertar_lesion(conn, item),
    }
}

/// Como `insertar_camiseta` de C (conserva el id) y además no duplica nombres.
fn insertar_camiseta(conn: &Connection, item: &Value) -> Result<bool, DbError> {
    let (Some(id), Some(nombre)) = (
        item["id"].as_i64().filter(|id| *id > 0),
        item["nombre"].as_str().map(str::trim),
    ) else {
        return Ok(false);
    };
    if nombre.is_empty() {
        return Ok(false);
    }
    let filas = conn.execute(
        "INSERT INTO camiseta (id, nombre) SELECT ?1, ?2
          WHERE NOT EXISTS (SELECT 1 FROM camiseta WHERE id = ?1 OR nombre = ?2)",
        params![id, nombre],
    )?;
    Ok(filas > 0)
}

/// C no pasaba `camiseta_id` (NOT NULL) y su `INSERT OR IGNORE` descartaba toda
/// lesión. Acá se usa la camiseta indicada por nombre (`camiseta`) o, si no
/// viene, la primera activa. Duplicado = mismo id, como en C.
fn insertar_lesion(conn: &Connection, item: &Value) -> Result<bool, DbError> {
    let texto = |clave: &str| {
        item[clave]
            .as_str()
            .map(str::trim)
            .filter(|t| !t.is_empty())
    };
    let (Some(id), Some(jugador), Some(tipo), Some(fecha)) = (
        item["id"].as_i64().filter(|id| *id > 0),
        texto("jugador"),
        texto("tipo"),
        texto("fecha"),
    ) else {
        return Ok(false);
    };
    let camiseta_id: Option<i64> = match item["camiseta"].as_str() {
        Some(nombre) => conn
            .query_row(
                "SELECT id FROM camiseta WHERE nombre = ?1",
                [nombre],
                |row| row.get(0),
            )
            .optional()?,
        None => conn
            .query_row(
                "SELECT id FROM camiseta ORDER BY activa DESC, id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?,
    };
    let Some(camiseta_id) = camiseta_id else {
        return Ok(false);
    };
    let filas = conn.execute(
        "INSERT OR IGNORE INTO lesion (id, jugador, tipo, descripcion, fecha, camiseta_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            jugador,
            tipo,
            item["descripcion"].as_str().unwrap_or("").trim(),
            mobile_service::fecha_partido(fecha),
            camiseta_id
        ],
    )?;
    Ok(filas > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn base() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO camiseta (id, nombre) VALUES (1, 'Azul');
                 INSERT INTO cancha (id, nombre) VALUES (1, 'La Bombonera');",
            )
            .unwrap();
        db
    }

    fn importar_texto(
        db: &mut Database,
        archivos: &[(&str, &str)],
        formato: Formato,
    ) -> Vec<Resumen> {
        let dir = tempfile::tempdir().unwrap();
        for (nombre, contenido) in archivos {
            std::fs::write(dir.path().join(nombre), contenido).unwrap();
        }
        importar(db.conn_mut(), dir.path(), formato).unwrap()
    }

    fn contar(db: &Database, sql: &str) -> i64 {
        db.conn().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn csv_como_lo_exporta_c_con_atajaste_y_errores() {
        let mut db = base();
        let resumenes = importar_texto(
            &mut db,
            &[
                (
                    "camisetas.csv",
                    "id,nombre,total_goles\n2,Roja Titular,3\n1,Azul,0\nx,Mala\n",
                ),
                (
                    "partidos.csv",
                    "Cancha,Fecha,Goles,...\n\
                     Club Norte,10/03/2025 20:00,2,1,Roja Titular,VICTORIA,Mucho Calor,Noche,8,4,7,\"Buen partido, gol\",1\n\
                     Club Norte,10/03/2025 20:00,2,1,Roja Titular,VICTORIA,Mucho Calor,Noche,8,4,7,repetido,-\n\
                     La Bombonera,2025-03-11 18:00,dos,0,Azul,EMPATE,Nublado,Tarde,5,5,5,x\n\
                     La Bombonera,2025-03-12 18:00,1,0,Inexistente,DERROTA,Nublado,Tarde,5,5,5,x\n\
                     corta,1\n",
                ),
            ],
            Formato::Csv,
        );
        assert_eq!(resumenes.len(), 2);
        let camisetas = &resumenes[0];
        assert_eq!((camisetas.importados, camisetas.omitidos), (1, 1));
        assert_eq!(camisetas.errores, ["línea 4: «x» no es un número (id)"]);
        let partidos = &resumenes[1];
        assert_eq!(
            (partidos.importados, partidos.omitidos),
            (1, 2),
            "{partidos:?}"
        );
        assert_eq!(partidos.errores.len(), 2);
        assert!(partidos.errores[0].starts_with("línea 4: «dos»"));
        assert!(partidos.errores[1].starts_with("línea 6: se esperaban 12 columnas"));
        let (fecha, resultado, clima, dia, comentario): (String, i64, i64, i64, String) = db
            .conn()
            .query_row(
                "SELECT fecha_hora, resultado, clima, dia, comentario_personal FROM partido",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(fecha, "2025-03-10 20:00:00");
        assert_eq!((resultado, clima, dia), (1, 5, 6));
        assert_eq!(comentario, "Buen partido, gol");
        // La camiseta inexistente no deja una cancha huérfana.
        assert_eq!(contar(&db, "SELECT COUNT(*) FROM cancha"), 2);
        let (historial, suspendido): (i64, i64) = db
            .conn()
            .query_row(
                "SELECT (SELECT COUNT(*) FROM undo_historial), suspendido FROM undo_control",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        // Solo las 2 filas de `base()`: lo importado no entra al deshacer.
        assert_eq!((historial, suspendido), (2, 0));
    }

    #[test]
    fn txt_de_c_con_atajaste_y_lesiones() {
        let mut db = base();
        let resumenes = importar_texto(
            &mut db,
            &[
                (
                    "camisetas.txt",
                    "LISTADO DE CAMISETAS CON ESTADISTICAS\n\nID: 2 - Nombre: Roja\n  Goles Totales: 3\n\n7 - Verde\nsin separador\n",
                ),
                (
                    "partidos.txt",
                    "PARTIDOS\n\n\
                     La Bombonera | 2025-03-10 20:00 | G:2 A:1 | Roja | Res:VICTORIA Cli:Lluvia Moderada Dia:Mañana RG:8 Can:4 EA:7 | Atajaste:SI | Gol de cabeza\n\
                     La Bombonera | 2025-03-11 20:00 | G:1 A:0 | Azul | Res:EMPATE Cli:Nublado Dia:Tarde RG:6 Can:5 EA:6 | Sin atajaste\n\
                     La Bombonera | 2025-03-12 20:00 | 1 0 | Azul | Res:EMPATE | x\n",
                ),
                (
                    "lesiones.txt",
                    "LISTADO DE LESIONES\n\n5 - Ana | Esguince | Tobillo | izquierdo | 15/03/2026 18:30\n6 - Ana | Golpe\n",
                ),
            ],
            Formato::Txt,
        );
        assert_eq!(resumenes[0].importados, 2, "{:?}", resumenes[0]);
        assert_eq!(resumenes[0].errores.len(), 1);
        assert_eq!(resumenes[1].importados, 2, "{:?}", resumenes[1]);
        assert!(resumenes[1].errores[0].starts_with("línea 5: se esperaba «G:goles"));
        assert_eq!(resumenes[2].importados, 1, "{:?}", resumenes[2]);
        assert_eq!(resumenes[2].errores.len(), 1);
        let (clima, dia, comentario): (i64, i64, String) = db
            .conn()
            .query_row(
                "SELECT clima, dia, comentario_personal FROM partido WHERE goles = 2",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((clima, dia), (10, 2));
        assert_eq!(comentario, "Gol de cabeza");
        let (descripcion, fecha, camiseta): (String, String, i64) = db
            .conn()
            .query_row(
                "SELECT descripcion, fecha, camiseta_id FROM lesion WHERE id = 5",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(descripcion, "Tobillo | izquierdo");
        assert_eq!(fecha, "2026-03-15 18:30:00");
        assert_eq!(camiseta, 1);
    }

    #[test]
    fn json_acepta_resultado_como_texto_y_reporta_basura() {
        let mut db = base();
        let resumenes = importar_texto(
            &mut db,
            &[
                (
                    "partidos.json",
                    r#"[{"cancha":"Club Sur","fecha":"2025-04-01 19:00","goles":1,"asistencias":0,
                         "camiseta":"Azul","resultado":"DERROTA","clima":"Frio","dia":"Noche",
                         "rendimiento_general":6,"cansancio":3,"estado_animo":5,"comentario_personal":"ok"},
                        {"cancha":"Club Sur","goles":1}, 7]"#,
                ),
                (
                    "lesiones.json",
                    r#"[{"id":9,"jugador":"Ana","tipo":"Golpe","descripcion":"Rodilla","fecha":"2025-04-02","camiseta":"Azul"}]"#,
                ),
                ("camisetas.json", "{no es json"),
            ],
            Formato::Json,
        );
        assert!(resumenes[0].errores[0].starts_with("JSON inválido"));
        assert_eq!((resumenes[1].importados, resumenes[1].omitidos), (1, 1));
        assert_eq!(resumenes[1].errores, ["elemento 3: no es un objeto"]);
        assert_eq!(resumenes[2].importados, 1);
        let (resultado, clima, dia): (i64, i64, i64) = db
            .conn()
            .query_row("SELECT resultado, clima, dia FROM partido", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!((resultado, clima, dia), (3, 7, 6));
    }

    #[test]
    fn html_por_fila_con_span_y_entidades() {
        let mut db = base();
        let resumenes = importar_texto(
            &mut db,
            &[
                (
                    "camisetas.html",
                    "<table><tr><th>ID</th><th>Nombre</th></tr>\
                     <tr><td>3</td><td>Pibes &amp; Cía</td><td>4</td><td>9</td></tr>\
                     <tr><td>4</td><td>Blanca</td><td>0</td></tr></table>",
                ),
                (
                    "partidos.html",
                    "<table>\n<tr><th>Cancha</th></tr><tr><td>Club Este</td><td>2025-05-01 10:00</td>\
                     <td>0</td><td>2</td><td>Blanca</td><td><span class=\"badge\">VICTORIA</span></td>\
                     <td>Despejado</td><td>Mediodia</td><td>7</td><td>2</td><td>8</td><td>&lt;3</td><td>NO</td></tr>\
                     <tr><td>solo una</td></tr></table>",
                ),
                (
                    "lesiones.html",
                    "<tr><td>1</td><td>Ana</td><td>Golpe</td><td>Codo</td><td>2025-05-02</td></tr>",
                ),
            ],
            Formato::Html,
        );
        assert_eq!(resumenes[0].importados, 2, "{:?}", resumenes[0]);
        assert_eq!(resumenes[1].importados, 1, "{:?}", resumenes[1]);
        assert_eq!(
            resumenes[1].errores,
            ["fila 2: se esperaban 12 columnas y hay 1"]
        );
        assert_eq!(resumenes[2].importados, 1);
        assert_eq!(
            contar(
                &db,
                "SELECT COUNT(*) FROM camiseta WHERE nombre = 'Pibes & Cía'"
            ),
            1
        );
        let (resultado, dia, comentario): (i64, i64, String) = db
            .conn()
            .query_row(
                "SELECT resultado, dia, comentario_personal FROM partido",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((resultado, dia), (1, 3));
        assert_eq!(comentario, "<3");
    }

    #[test]
    fn sin_archivos_es_error_y_no_toca_la_base() {
        let mut db = base();
        let dir = tempfile::tempdir().unwrap();
        let error = importar(db.conn_mut(), dir.path(), Formato::Csv).unwrap_err();
        assert!(error.contains("camisetas.csv"), "{error}");
        assert_eq!(
            campos_csv("a,\"b, c\",\"d \"\"e\"\"\""),
            ["a", "b, c", "d \"e\""]
        );
    }
}
