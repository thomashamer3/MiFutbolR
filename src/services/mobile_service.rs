//! Intercambio con MiFutbolMobile por archivos (port de
//! `exportar_para_mifutbolmobile` de `export.c` y de la sincronización de
//! partidos de `import.c`).
//!
//! - Exportar: `Exportaciones/mifutbolmobile_import.json` con el formato
//!   `mifutbolmobile-import` v1 (mismas claves y códigos que C).
//! - Importar: `Importaciones/partidos.json` (arreglo de partidos del móvil).
//!   Con el marcador `Importaciones/.sync_partidos_auto` se importa al iniciar
//!   y el archivo se renombra a `partidos_importados_<sello>.json`.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value, json};

use crate::db::DbError;
use crate::db::sync_repo::{clima_codigo, codigo_enum, dia_codigo, resultado_codigo};
use crate::domain::cancha_codes::{decode_superficies, decode_tipos};
use crate::domain::models::{Superficie, TipoCancha};
use crate::services::export_service::ExportError;

/// Archivo que exporta para el móvil.
pub const ARCHIVO_EXPORTACION: &str = "mifutbolmobile_import.json";
/// Archivo de partidos que deja el móvil.
pub const ARCHIVO_PARTIDOS: &str = "partidos.json";
/// Marcador de importación automática al iniciar.
pub const MARCADOR_AUTO: &str = ".sync_partidos_auto";

// ----------------------------------------------------------- Códigos móviles

fn clima_mobile(clima: i64) -> i64 {
    match clima {
        1..=4 => clima,
        5 | 8 => 6,
        6 | 7 => 5,
        9..=12 => 3,
        _ => 0,
    }
}

fn categoria_mobile(categoria: i64) -> i64 {
    match categoria {
        0 => 4,
        1 => 2,
        2..=4 => 1,
        5 => 3,
        _ => 5,
    }
}

fn tipo_futbol_mobile(tipo: i64) -> i64 {
    match tipo {
        1 => 7,
        2 => 8,
        3 => 11,
        _ => 5,
    }
}

/// Texto del tipo de cancha como `texto_tipo_cancha` de C.
fn tipo_cancha_texto(codigo: i64) -> String {
    let nombres: Vec<&str> = decode_tipos(codigo)
        .into_iter()
        .map(|tipo| match tipo {
            TipoCancha::F5 => "Futbol 5",
            TipoCancha::F7 => "Futbol 7",
            TipoCancha::F8 => "Futbol 8",
            TipoCancha::F11 => "Futbol 11",
            TipoCancha::Otro => "Otro",
        })
        .collect();
    if nombres.is_empty() {
        "No especificado".to_string()
    } else {
        nombres.join(", ")
    }
}

/// Texto de la superficie como `texto_superficie` de C.
fn superficie_texto(codigo: i64) -> String {
    let nombres: Vec<&str> = decode_superficies(codigo)
        .into_iter()
        .filter_map(|superficie| match superficie {
            Superficie::Natural => Some("Natural"),
            Superficie::Sintetico => Some("Sintetico"),
            Superficie::Cemento => Some("Cemento"),
            Superficie::Otra => Some("Otra"),
            Superficie::NoSe => None,
        })
        .collect();
    if nombres.is_empty() {
        "No especificada".to_string()
    } else {
        nombres.join(", ")
    }
}

fn techada_texto(estado: i64) -> &'static str {
    match estado {
        1 => "SI",
        0 => "NO",
        _ => "NO SE",
    }
}

/// "AAAA-MM-DD HH:MM" → "AAAA-MM-DDTHH:MM:00" (`mm_fecha_hora_iso`).
fn fecha_hora_iso(origen: &str) -> String {
    let mut iso = origen.replace(' ', "T");
    if iso.len() == 16 {
        iso.push_str(":00");
    }
    iso
}

// ---------------------------------------------------------------- Exportar

/// Ejecuta `sql` y arma un objeto por fila con `fila`.
fn filas(
    conn: &Connection,
    sql: &str,
    fila: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
) -> Result<Vec<Value>, DbError> {
    let mut stmt = conn.prepare(sql)?;
    let items = stmt
        .query_map([], |row| fila(row))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn bool_de(row: &rusqlite::Row<'_>, indice: usize) -> rusqlite::Result<bool> {
    Ok(row.get::<_, i64>(indice)? != 0)
}

fn canchas(conn: &Connection) -> Result<Vec<Value>, DbError> {
    filas(
        conn,
        "SELECT nombre, telefono, direccion, localidad, tipo_cancha_codigo, superficie_codigo,
                techada_estado_codigo, tiene_iluminacion, horario_apertura_min, horario_cierre_min,
                precio_hora_dia_centavos, precio_hora_noche_centavos, tiene_vestuarios,
                tiene_duchas, tiene_buffet, tiene_estacionamiento, cantidad_canchas, estado,
                descripcion, imagen_ruta, activa, tiene_grabacion, latitud, longitud
           FROM cancha ORDER BY id",
        |row| {
            Ok(json!({
                "nombre": row.get::<_, String>(0)?,
                "telefono": row.get::<_, String>(1)?,
                "direccion": row.get::<_, String>(2)?,
                "localidad": row.get::<_, String>(3)?,
                "tipoCancha": tipo_cancha_texto(row.get(4)?),
                "superficie": superficie_texto(row.get(5)?),
                "techadaEstado": techada_texto(row.get(6)?),
                "tieneIluminacion": bool_de(row, 7)?,
                "horarioAperturaMin": row.get::<_, i64>(8)?,
                "horarioCierreMin": row.get::<_, i64>(9)?,
                "precioHoraDiaCentavos": row.get::<_, i64>(10)?,
                "precioHoraNocheCentavos": row.get::<_, i64>(11)?,
                "tieneVestuarios": bool_de(row, 12)?,
                "tieneDuchas": bool_de(row, 13)?,
                "tieneBuffet": bool_de(row, 14)?,
                "tieneEstacionamiento": bool_de(row, 15)?,
                "cantidadCanchas": row.get::<_, i64>(16)?,
                "estado": row.get::<_, String>(17)?,
                "descripcion": row.get::<_, String>(18)?,
                "imagenRuta": row.get::<_, String>(19)?,
                "activa": bool_de(row, 20)?,
                "tieneGrabacion": bool_de(row, 21)?,
                "latitud": row.get::<_, Option<f64>>(22)?,
                "longitud": row.get::<_, Option<f64>>(23)?,
            }))
        },
    )
}

fn camisetas(conn: &Connection) -> Result<Vec<Value>, DbError> {
    filas(
        conn,
        "SELECT nombre, color_principal, color_secundario, marca, modelo, temporada,
                estado_fisico, fecha_compra, costo_centavos, observaciones, proveedor,
                fue_regalo, regalo_de, imagen_ruta, sorteada, activa
           FROM camiseta ORDER BY id",
        |row| {
            Ok(json!({
                "nombre": row.get::<_, String>(0)?,
                "colorPrincipal": row.get::<_, String>(1)?,
                "colorSecundario": row.get::<_, String>(2)?,
                "marca": row.get::<_, String>(3)?,
                "modelo": row.get::<_, String>(4)?,
                "temporada": row.get::<_, String>(5)?,
                "estadoFisico": row.get::<_, String>(6)?,
                "fechaCompra": row.get::<_, String>(7)?,
                "costoCentavos": row.get::<_, i64>(8)?,
                "observaciones": row.get::<_, String>(9)?,
                "proveedor": row.get::<_, String>(10)?,
                "fueRegalo": bool_de(row, 11)?,
                "regaloDe": row.get::<_, String>(12)?,
                "imagenRuta": row.get::<_, String>(13)?,
                "sorteada": bool_de(row, 14)?,
                "activa": bool_de(row, 15)?,
            }))
        },
    )
}

fn partidos(conn: &Connection) -> Result<Vec<Value>, DbError> {
    filas(
        conn,
        "SELECT p.fecha_hora, p.goles, p.asistencias, p.resultado, p.rendimiento_general,
                p.cansancio, p.estado_animo, p.comentario_personal, p.clima, p.dia,
                p.precio_centavos, p.tipo_partido, p.rival_nombre, p.tipo_rival,
                p.posicion_jugada, p.minutos_jugados, p.intensidad, p.condicion_cancha,
                p.arbitraje, p.eventos_clave, p.rating_tecnico, p.rating_fisico,
                p.rating_mental, p.estado_cancha, p.goles_equipo, p.goles_rival,
                p.formato_partido, p.tarjeta, p.goles_en_contra, p.dolor_fisico,
                p.temperatura_c, p.arbitraje_score, p.lo_mejor, p.que_mejorar, p.tags,
                p.goles_detalle, p.asistencias_detalle, p.atajaste_todo_el_partido,
                p.precip_mm, p.wind_kmh, p.weather_code, p.apparent_temp_c, p.clima_json,
                p.diferencia_gol, can.nombre, c.nombre,
                (SELECT nombre FROM botin WHERE id = p.botin_id),
                (SELECT nombre FROM equipo WHERE partido_id = p.id ORDER BY id LIMIT 1),
                COALESCE((SELECT favorito FROM partido_meta WHERE partido_id = p.id), 0)
           FROM partido p JOIN cancha can ON can.id = p.cancha_id
           JOIN camiseta c ON c.id = p.camiseta_id
          ORDER BY p.fecha_hora DESC, p.id DESC",
        |row| {
            let entero = |indice: usize| row.get::<_, i64>(indice);
            let texto = |indice: usize| row.get::<_, String>(indice);
            let real = |indice: usize| row.get::<_, Option<f64>>(indice);
            // Muchas claves: un solo json! pasa el límite de recursión de la macro.
            let campos: Vec<(&str, Value)> = vec![
                ("fechaHora", json!(fecha_hora_iso(&texto(0)?))),
                ("goles", json!(entero(1)?)),
                ("asistencias", json!(entero(2)?)),
                ("resultado", json!(entero(3)?)),
                ("rendimientoGeneral", json!(entero(4)?)),
                ("cansancio", json!(entero(5)?)),
                ("estadoAnimo", json!(entero(6)?)),
                ("comentarioPersonal", json!(texto(7)?)),
                ("clima", json!(clima_mobile(entero(8)?))),
                ("dia", json!(entero(9)?)),
                ("precioCentavos", json!(entero(10)?)),
                ("tipoPartido", json!(entero(11)?)),
                ("rivalNombre", json!(texto(12)?)),
                ("tipoRival", json!(texto(13)?)),
                ("posicionJugada", json!(texto(14)?)),
                ("minutosJugados", json!(entero(15)?)),
                ("intensidad", json!(entero(16)?)),
                // MiFutbolR no registra el esfuerzo percibido.
                ("esfuerzoPercibido", json!(0)),
                ("condicionCancha", json!(texto(17)?)),
                ("arbitraje", json!(texto(18)?)),
                ("eventosClave", json!(texto(19)?)),
                ("ratingTecnico", json!(entero(20)?)),
                ("ratingFisico", json!(entero(21)?)),
                ("ratingMental", json!(entero(22)?)),
                ("estadoCancha", json!(entero(23)?)),
                ("golesEquipo", json!(entero(24)?)),
                ("golesRival", json!(entero(25)?)),
                ("formatoPartido", json!(texto(26)?)),
                ("tarjeta", json!(entero(27)?)),
                ("golesEnContra", json!(entero(28)?)),
                ("dolorFisico", json!(entero(29)?)),
                ("temperaturaC", json!(real(30)?)),
                ("arbitrajeScore", json!(entero(31)?)),
                ("loMejor", json!(texto(32)?)),
                ("queMejorar", json!(texto(33)?)),
                ("tags", json!(texto(34)?)),
                ("golesDetalle", json!(texto(35)?)),
                ("asistenciasDetalle", json!(texto(36)?)),
                ("atajasteTodoElPartido", json!(entero(37)? != 2)),
                ("precipMm", json!(real(38)?)),
                ("windKmh", json!(real(39)?)),
                ("weatherCode", json!(row.get::<_, Option<i64>>(40)?)),
                ("apparentTempC", json!(real(41)?)),
                ("climaJson", json!(texto(42)?)),
                ("diferenciaGol", json!(entero(43)?)),
                ("cancha", json!(texto(44)?)),
                ("camiseta", json!(texto(45)?)),
                ("estado", json!("finalizado")),
                ("favorito", json!(entero(48)? != 0)),
            ];
            let mut item = Value::Object(
                campos
                    .into_iter()
                    .map(|(clave, valor)| (clave.to_string(), valor))
                    .collect(),
            );
            if let Some(botin) = row.get::<_, Option<String>>(46)? {
                item["botin"] = json!(botin);
            }
            if let Some(equipo) = row.get::<_, Option<String>>(47)? {
                item["equipo"] = json!(equipo);
            }
            Ok(item)
        },
    )
}

/// JSON `mifutbolmobile-import` completo. `usuario` va en `perfil.nombre`
/// (si falta, se usa el nombre de la tabla `usuario`).
pub fn json_mobile(conn: &Connection, usuario: Option<&str>) -> Result<Value, DbError> {
    let mut raiz = Map::new();
    raiz.insert("formato".into(), json!("mifutbolmobile-import"));
    raiz.insert("version".into(), json!(1));
    raiz.insert("origen".into(), json!("MiFutbolC"));
    raiz.insert(
        "generadoEn".into(),
        json!(crate::domain::dates::ahora_fmt("%Y-%m-%dT%H:%M:%S")),
    );
    let nombre = match usuario {
        Some(nombre) if !nombre.is_empty() => nombre.to_string(),
        _ => conn
            .query_row("SELECT nombre FROM usuario WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?
            .unwrap_or_default(),
    };
    if !nombre.is_empty() {
        raiz.insert("perfil".into(), json!({"nombre": nombre}));
    }
    raiz.insert("canchas".into(), json!(canchas(conn)?));
    raiz.insert("camisetas".into(), json!(camisetas(conn)?));
    raiz.insert(
        "equipos".into(),
        json!(filas(
            conn,
            "SELECT nombre, tipo, tipo_futbol, imagen_ruta, activa FROM equipo ORDER BY id",
            |row| Ok(json!({
                "nombre": row.get::<_, String>(0)?,
                "tipo": if row.get::<_, i64>(1)? == 0 { 1 } else { 2 },
                "tipoFutbol": tipo_futbol_mobile(row.get(2)?),
                "imagenRuta": row.get::<_, String>(3)?,
                "activa": bool_de(row, 4)?,
            })),
        )?),
    );
    raiz.insert(
        "botines".into(),
        json!(filas(
            conn,
            "SELECT nombre, sorteada, activa FROM botin ORDER BY id",
            |row| Ok(json!({
                "nombre": row.get::<_, String>(0)?,
                "sorteada": bool_de(row, 1)?,
                "activa": bool_de(row, 2)?,
            })),
        )?),
    );
    raiz.insert("partidos".into(), json!(partidos(conn)?));
    raiz.insert(
        "lesiones".into(),
        json!(filas(
            conn,
            "SELECT l.tipo, l.descripcion, l.fecha, c.nombre, l.estado
               FROM lesion l LEFT JOIN camiseta c ON c.id = l.camiseta_id ORDER BY l.id",
            |row| {
                let mut item = json!({
                    "tipo": row.get::<_, String>(0)?,
                    "descripcion": row.get::<_, String>(1)?,
                    "fecha": row.get::<_, String>(2)?,
                    "estado": row.get::<_, String>(4)?,
                });
                if let Some(camiseta) = row.get::<_, Option<String>>(3)? {
                    item["camiseta"] = json!(camiseta);
                }
                Ok(item)
            },
        )?),
    );
    raiz.insert(
        "financiamiento".into(),
        json!(filas(
            conn,
            "SELECT fecha, tipo, categoria, descripcion, monto_centavos, item_especifico
               FROM financiamiento ORDER BY id",
            |row| {
                let mut item = json!({
                    "fecha": row.get::<_, String>(0)?,
                    "tipo": if row.get::<_, i64>(1)? == 0 { 2 } else { 1 },
                    "categoria": categoria_mobile(row.get(2)?),
                    "descripcion": row.get::<_, String>(3)?,
                    "montoCentavos": row.get::<_, i64>(4)?,
                });
                let especifico: String = row.get(5)?;
                if !especifico.is_empty() {
                    item["itemEspecifico"] = json!(especifico);
                }
                Ok(item)
            },
        )?),
    );
    // C los leía de Importaciones/recordatorios.json; en MiFutbolR están en la base.
    raiz.insert(
        "recordatorios".into(),
        json!(filas(
            conn,
            "SELECT titulo, nota, fecha, hora, tematica, completado FROM recordatorio ORDER BY id",
            |row| {
                let titulo: String = row.get(0)?;
                let tematica: String = row.get(4)?;
                let hora: String = row.get(3)?;
                let fecha: String = row.get(2)?;
                let fecha = if hora.is_empty() {
                    fecha
                } else {
                    format!("{fecha} {hora}")
                };
                let titulo = [titulo.as_str(), tematica.as_str()]
                    .into_iter()
                    .find(|texto| !texto.is_empty())
                    .unwrap_or("Recordatorio")
                    .to_string();
                Ok(json!({
                    "titulo": titulo,
                    "descripcion": row.get::<_, String>(1)?,
                    "fechaHora": fecha_hora_iso(&fecha),
                    "tematica": tematica,
                    "completado": bool_de(row, 5)?,
                }))
            },
        )?),
    );
    Ok(Value::Object(raiz))
}

/// Escribe `mifutbolmobile_import.json` en `dir` (Exportaciones).
pub fn exportar(
    conn: &Connection,
    usuario: Option<&str>,
    dir: &Path,
) -> Result<PathBuf, ExportError> {
    let json = json_mobile(conn, usuario).map_err(|error| ExportError(error.to_string()))?;
    std::fs::create_dir_all(dir)?;
    let ruta = dir.join(ARCHIVO_EXPORTACION);
    std::fs::write(&ruta, serde_json::to_string_pretty(&json)?)?;
    Ok(ruta)
}

// ---------------------------------------------------------------- Importar

/// "DD/MM/AAAA[ HH:MM]" o ISO → "AAAA-MM-DD HH:MM:SS".
pub(crate) fn fecha_partido(texto: &str) -> String {
    let texto = texto.trim().replacen('T', " ", 1);
    let texto = match texto.as_bytes() {
        [d1, d2, b'/', m1, m2, b'/', a1, a2, a3, a4, ..] => {
            let resto = texto.get(10..).unwrap_or("");
            format!(
                "{}{}{}{}-{}{}-{}{}{resto}",
                *a1 as char,
                *a2 as char,
                *a3 as char,
                *a4 as char,
                *m1 as char,
                *m2 as char,
                *d1 as char,
                *d2 as char
            )
        }
        _ => texto,
    };
    match texto.len() {
        10 => format!("{texto} 00:00:00"),
        16 => format!("{texto}:00"),
        _ => texto,
    }
}

/// Importa el arreglo de partidos del móvil (`importar_partidos_json_array`):
/// la cancha se crea si falta, la camiseta debe existir y no se duplican
/// partidos con la misma cancha, fecha y camiseta. Devuelve (importados, total).
pub fn importar_partidos(conn: &mut Connection, ruta: &Path) -> Result<(usize, usize), String> {
    let texto =
        std::fs::read_to_string(ruta).map_err(|error| format!("{}: {error}", ruta.display()))?;
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&texto) else {
        return Err("El archivo no contiene un array JSON de partidos".to_string());
    };
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])
        .map_err(|e| e.to_string())?;
    let mut importados = 0;
    for item in &items {
        if importar_item(&tx, item).unwrap_or(false) {
            importados += 1;
        }
    }
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok((importados, items.len()))
}

pub(crate) fn importar_item(conn: &Connection, item: &Value) -> Result<bool, DbError> {
    let (Some(cancha), Some(fecha), Some(goles), Some(asistencias), Some(camiseta)) = (
        item["cancha"].as_str(),
        item["fecha"].as_str(),
        item["goles"].as_i64(),
        item["asistencias"].as_i64(),
        item["camiseta"].as_str(),
    ) else {
        return Ok(false);
    };
    let entero = |clave: &str| item[clave].as_i64().unwrap_or(0);
    // Resultado, clima y franja como número (móvil) o como texto (exportadores de C).
    let codigo = |clave: &str, desde_texto: fn(&str) -> i64| codigo_enum(&item[clave], desde_texto);
    // La camiseta primero: C creaba la cancha aunque después omitiera el partido.
    let Some(camiseta_id) = conn
        .query_row(
            "SELECT id FROM camiseta WHERE nombre = ?1",
            [camiseta],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    else {
        return Ok(false);
    };
    let cancha_id = match conn
        .query_row("SELECT id FROM cancha WHERE nombre = ?1", [cancha], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
    {
        Some(id) => id,
        None => {
            conn.execute("INSERT INTO cancha (nombre) VALUES (?1)", [cancha])?;
            conn.last_insert_rowid()
        }
    };
    let fecha = fecha_partido(fecha);
    let existe: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido WHERE cancha_id = ?1 AND fecha_hora = ?2 AND camiseta_id = ?3",
        params![cancha_id, fecha, camiseta_id],
        |row| row.get(0),
    )?;
    if existe > 0 {
        return Ok(false);
    }
    // Una fila fuera de rango (CHECK) se omite sin cortar el resto.
    let insertado = conn.execute(
        "INSERT INTO partido (cancha_id, fecha_hora, mes_anio, goles, asistencias, camiseta_id,
             resultado, clima, dia, rendimiento_general, cansancio, estado_animo, comentario_personal)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            cancha_id,
            fecha,
            fecha.get(..7).unwrap_or(""),
            goles,
            asistencias,
            camiseta_id,
            codigo("resultado", resultado_codigo),
            codigo("clima", clima_codigo),
            codigo("dia", dia_codigo),
            entero("rendimiento_general"),
            entero("cansancio"),
            entero("estado_animo"),
            item["comentario_personal"].as_str().unwrap_or(""),
        ],
    );
    Ok(insertado.is_ok())
}

/// ¿Está activa la importación automática al iniciar?
#[must_use]
pub fn auto_activo(import_dir: &Path) -> bool {
    import_dir.join(MARCADOR_AUTO).exists()
}

/// Crea o borra el marcador. Devuelve el nuevo estado.
pub fn alternar_auto(import_dir: &Path) -> std::io::Result<bool> {
    let marcador = import_dir.join(MARCADOR_AUTO);
    if marcador.exists() {
        std::fs::remove_file(marcador)?;
        Ok(false)
    } else {
        std::fs::create_dir_all(import_dir)?;
        std::fs::write(marcador, "activo\n")?;
        Ok(true)
    }
}

/// Importación al iniciar (`sincronizar_partidos_pendientes`): con el marcador
/// y `partidos.json` presentes, importa y renombra el archivo para no
/// reimportarlo. Devuelve (importados, destino) si importó algo.
pub fn sincronizar_pendientes(
    conn: &mut Connection,
    import_dir: &Path,
) -> Result<Option<(usize, PathBuf)>, String> {
    let ruta = import_dir.join(ARCHIVO_PARTIDOS);
    if !auto_activo(import_dir) || !ruta.exists() {
        return Ok(None);
    }
    let (importados, _) = importar_partidos(conn, &ruta)?;
    if importados == 0 {
        return Ok(None);
    }
    let destino = import_dir.join(format!(
        "partidos_importados_{}.json",
        crate::domain::dates::ahora_fmt("%Y%m%d_%H%M%S")
    ));
    if std::fs::rename(&ruta, &destino).is_err() {
        let _ = std::fs::remove_file(&ruta);
    }
    Ok(Some((importados, destino)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn base() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (id, nombre, tipo_cancha_codigo, superficie_codigo, techada_estado_codigo)
                     VALUES (1, 'La Bombonera', 1003, 2, 1);
                 INSERT INTO camiseta (id, nombre) VALUES (1, 'Titular');
                 INSERT INTO botin (id, nombre) VALUES (1, 'Nike');
                 INSERT INTO partido (cancha_id, camiseta_id, botin_id, fecha_hora, goles, clima,
                     precio_centavos, atajaste_todo_el_partido)
                 VALUES (1, 1, 1, '2026-03-10 20:00:00', 2, 5, 150000, 2);
                 INSERT INTO financiamiento (fecha, tipo, categoria, descripcion, monto_centavos)
                     VALUES ('2026-03-01', 0, 5, 'Cancha', 120050);
                 INSERT INTO recordatorio (titulo, fecha, hora, tematica) VALUES ('', '2026-04-01', '18:30', 'Torneo');",
            )
            .unwrap();
        db
    }

    #[test]
    fn exporta_el_formato_de_mifutbolmobile() {
        let db = base();
        let dir = tempfile::tempdir().unwrap();
        let ruta = exportar(db.conn(), Some("Tomi"), dir.path()).unwrap();
        assert_eq!(ruta.file_name().unwrap(), ARCHIVO_EXPORTACION);
        let json: Value = serde_json::from_str(&std::fs::read_to_string(ruta).unwrap()).unwrap();
        assert_eq!(json["formato"], "mifutbolmobile-import");
        assert_eq!(json["version"], 1);
        assert_eq!(json["origen"], "MiFutbolC");
        assert_eq!(json["perfil"]["nombre"], "Tomi");
        let cancha = &json["canchas"][0];
        assert_eq!(cancha["tipoCancha"], "Futbol 5, Futbol 7");
        assert_eq!(cancha["superficie"], "Sintetico");
        assert_eq!(cancha["techadaEstado"], "SI");
        let partido = &json["partidos"][0];
        assert_eq!(partido["fechaHora"], "2026-03-10T20:00:00");
        assert_eq!(partido["clima"], 6, "5 (Mucho Calor) → 6 en el móvil");
        assert_eq!(partido["precioCentavos"], 150_000);
        assert_eq!(partido["atajasteTodoElPartido"], false);
        assert_eq!(partido["cancha"], "La Bombonera");
        assert_eq!(partido["botin"], "Nike");
        assert!(partido.get("equipo").is_none());
        assert_eq!(partido["estado"], "finalizado");
        let movimiento = &json["financiamiento"][0];
        assert_eq!(
            (
                movimiento["tipo"].as_i64(),
                movimiento["categoria"].as_i64()
            ),
            (Some(2), Some(3))
        );
        assert_eq!(movimiento["montoCentavos"], 120_050);
        let recordatorio = &json["recordatorios"][0];
        assert_eq!(recordatorio["titulo"], "Torneo");
        assert_eq!(recordatorio["fechaHora"], "2026-04-01T18:30:00");
        for clave in ["equipos", "botines", "lesiones", "camisetas"] {
            assert!(json[clave].is_array(), "{clave}");
        }
    }

    #[test]
    fn importa_partidos_del_movil_sin_duplicar() {
        let mut db = base();
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join(ARCHIVO_PARTIDOS);
        std::fs::write(
            &ruta,
            r#"[
              {"cancha": "Nueva", "fecha": "2026-05-01T18:30", "goles": 3, "asistencias": 1,
               "camiseta": "Titular", "resultado": 1, "comentario_personal": "bien"},
              {"cancha": "Nueva", "fecha": "2026-05-01 18:30:00", "goles": 3, "asistencias": 1, "camiseta": "Titular"},
              {"cancha": "X", "fecha": "02/05/2026", "goles": 0, "asistencias": 0, "camiseta": "No existe"},
              {"cancha": "X", "goles": 1}
            ]"#,
        )
        .unwrap();
        assert_eq!(importar_partidos(db.conn_mut(), &ruta).unwrap(), (1, 4));
        let (fecha, mes, comentario): (String, String, String) = db
            .conn()
            .query_row(
                "SELECT p.fecha_hora, p.mes_anio, p.comentario_personal FROM partido p
                   JOIN cancha c ON c.id = p.cancha_id WHERE c.nombre = 'Nueva'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (fecha.as_str(), mes.as_str(), comentario.as_str()),
            ("2026-05-01 18:30:00", "2026-05", "bien")
        );
        assert_eq!(fecha_partido("02/05/2026 10:15"), "2026-05-02 10:15:00");
    }

    #[test]
    fn al_iniciar_solo_importa_con_marcador_y_renombra() {
        let mut db = base();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(ARCHIVO_PARTIDOS),
            r#"[{"cancha": "Nueva", "fecha": "2026-06-01 10:00:00", "goles": 1, "asistencias": 0, "camiseta": "Titular"}]"#,
        )
        .unwrap();
        assert_eq!(
            sincronizar_pendientes(db.conn_mut(), dir.path()).unwrap(),
            None
        );
        assert!(alternar_auto(dir.path()).unwrap());
        let (importados, destino) = sincronizar_pendientes(db.conn_mut(), dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(importados, 1);
        assert!(destino.exists());
        assert!(!dir.path().join(ARCHIVO_PARTIDOS).exists());
        assert!(!alternar_auto(dir.path()).unwrap());
        assert!(!auto_activo(dir.path()));
    }
}
