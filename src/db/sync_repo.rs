//! Parte local de la sincronización con PocketBase (port de `sync_queue.c`, la
//! mitad sin red de `sync.c` y de `sync_entidades.c`): configuración, cola,
//! conflictos, armado de payloads y aplicación de registros remotos.
//!
//! Las escrituras que vienen del servidor van en una transacción con el
//! deshacer suspendido: no son acciones del usuario.

use std::cmp::Ordering;

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde_json::{Map, Value, json};

use super::error::DbError;
use super::partido_repo;

/// UUID v4 generado en SQL (mismo formato que `sync_entidades.c`).
const UUID_SQL: &str = "lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || \
     substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || \
     substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))";

/// Marca de tiempo local ISO (`sync_ahora` de C).
#[must_use]
pub fn ahora_iso() -> String {
    crate::domain::dates::ahora_fmt("%Y-%m-%dT%H:%M:%S")
}

/// Compara marcas ISO aceptando 'T' o espacio como separador (`sync_comparar_fechas`).
#[must_use]
pub fn comparar_fechas(a: &str, b: &str) -> Ordering {
    a.replacen('T', " ", 1).cmp(&b.replacen('T', " ", 1))
}

/// "AAAA-MM-DD HH:MM:SS" → "AAAA-MM-DDTHH:MM:SS".
fn a_iso(texto: &str) -> String {
    texto.replace(' ', "T")
}

// ------------------------------------------------------------ Configuración

/// Valor de `sync_config` (vacío si no existe).
pub fn config_get(conn: &Connection, clave: &str) -> Result<String, DbError> {
    Ok(conn
        .query_row(
            "SELECT COALESCE(valor, '') FROM sync_config WHERE clave = ?1",
            [clave],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_default())
}

/// Guarda un valor de `sync_config`.
pub fn config_set(conn: &Connection, clave: &str, valor: &str) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO sync_config (clave, valor, actualizado_en)
         VALUES (?1, ?2, datetime('now','localtime'))
         ON CONFLICT(clave) DO UPDATE SET valor = excluded.valor,
                                          actualizado_en = excluded.actualizado_en",
        params![clave, valor],
    )?;
    Ok(())
}

/// Completa uuid y `updated_at` faltantes y compacta la cola (`sync_init_db`).
/// El uuid nuevo de un partido lo encola por el trigger de UPDATE.
pub fn preparar(conn: &mut Connection) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])?;
    for tabla in ["partido", "camiseta", "botin"] {
        tx.execute(
            &format!("UPDATE {tabla} SET uuid = {UUID_SQL} WHERE uuid IS NULL OR TRIM(uuid) = ''"),
            [],
        )?;
    }
    tx.execute(
        "UPDATE camiseta SET updated_at = COALESCE(NULLIF(created_at, ''), datetime('now','localtime'))
          WHERE updated_at = ''",
        [],
    )?;
    tx.execute(
        "UPDATE botin SET updated_at = datetime('now','localtime') WHERE updated_at = ''",
        [],
    )?;
    tx.execute(
        "DELETE FROM sync_dirty WHERE uuid IS NULL OR TRIM(uuid) = ''",
        [],
    )?;
    tx.execute(
        "DELETE FROM sync_dirty WHERE id NOT IN (
            SELECT MIN(id) FROM sync_dirty GROUP BY uuid, operacion, estado)",
        [],
    )?;
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(())
}

// --------------------------------------------------------------------- Cola

/// Operación pendiente de la cola local.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pendiente {
    /// Id de la fila de la cola.
    pub id: i64,
    /// Uuid del partido.
    pub uuid: String,
    /// `upsert` o `eliminar`.
    pub operacion: String,
    /// Cuándo se encoló (hora local).
    pub creado_en: String,
}

fn pendiente_de(row: &rusqlite::Row<'_>) -> rusqlite::Result<Pendiente> {
    Ok(Pendiente {
        id: row.get(0)?,
        uuid: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        operacion: row.get(2)?,
        creado_en: row.get(3)?,
    })
}

/// Próxima operación pendiente o con error cuyo id supere `despues_de`.
pub fn siguiente_pendiente(
    conn: &Connection,
    despues_de: i64,
) -> Result<Option<Pendiente>, DbError> {
    Ok(conn
        .query_row(
            "SELECT id, uuid, operacion, creado_en FROM sync_dirty
              WHERE estado IN ('pendiente', 'error') AND id > ?1 ORDER BY id LIMIT 1",
            [despues_de],
            pendiente_de,
        )
        .optional()?)
}

/// Última operación de la cola para ese uuid, en cualquier estado (`sync_buscar_pendiente`).
pub fn ultimo_pendiente(conn: &Connection, uuid: &str) -> Result<Option<Pendiente>, DbError> {
    Ok(conn
        .query_row(
            "SELECT id, uuid, operacion, creado_en FROM sync_dirty
              WHERE uuid = ?1 ORDER BY id DESC LIMIT 1",
            [uuid],
            pendiente_de,
        )
        .optional()?)
}

/// Quita de la cola todas las operaciones del uuid.
pub fn marcar_hecho(conn: &Connection, uuid: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM sync_dirty WHERE uuid = ?1", [uuid])?;
    Ok(())
}

/// Marca con error las operaciones del uuid de la fila `id`.
pub fn marcar_error(conn: &Connection, id: i64, error: &str) -> Result<(), DbError> {
    conn.execute(
        "UPDATE sync_dirty SET estado = 'error', ultimo_error = ?1
          WHERE uuid = (SELECT uuid FROM sync_dirty WHERE id = ?2)",
        params![error, id],
    )?;
    Ok(())
}

/// Marca en conflicto las operaciones del uuid.
pub fn marcar_conflicto(conn: &Connection, uuid: &str) -> Result<(), DbError> {
    conn.execute(
        "UPDATE sync_dirty SET estado = 'conflicto' WHERE uuid = ?1",
        [uuid],
    )?;
    Ok(())
}

/// Partidos distintos pendientes de subir.
pub fn contar_pendientes(conn: &Connection) -> Result<i64, DbError> {
    Ok(conn.query_row(
        "SELECT COUNT(DISTINCT uuid) FROM sync_dirty WHERE estado IN ('pendiente', 'error')",
        [],
        |row| row.get(0),
    )?)
}

/// Conflictos sin resolver.
pub fn contar_conflictos(conn: &Connection) -> Result<i64, DbError> {
    Ok(conn.query_row("SELECT COUNT(*) FROM sync_conflicto", [], |row| row.get(0))?)
}

/// Encola todos los partidos con uuid (`sync_queue_enqueue_all`).
pub fn encolar_todos(conn: &Connection) -> Result<usize, DbError> {
    Ok(conn.execute(
        "INSERT INTO sync_dirty (partido_id, uuid, operacion, estado, creado_en)
         SELECT id, uuid, 'upsert', 'pendiente', datetime('now','localtime')
           FROM partido WHERE uuid IS NOT NULL AND TRIM(uuid) <> ''",
        [],
    )?)
}

// --------------------------------------------------------------- Conflictos

/// Conflicto entre la versión local y la del servidor.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflicto {
    /// Id de la fila.
    pub id: i64,
    /// Uuid del partido.
    pub uuid: String,
    /// Payload local.
    pub local: Value,
    /// Registro remoto completo.
    pub remoto: Value,
}

/// Guarda (o reemplaza) el conflicto del uuid.
pub fn guardar_conflicto(
    conn: &Connection,
    uuid: &str,
    local: &Value,
    remoto: &Value,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO sync_conflicto (uuid, local_json, remoto_json, descripcion, creado_en)
         VALUES (?1, ?2, ?3, 'El partido cambió en otro dispositivo.', datetime('now','localtime'))
         ON CONFLICT(uuid) DO UPDATE SET local_json = excluded.local_json,
             remoto_json = excluded.remoto_json, creado_en = excluded.creado_en",
        params![uuid, local.to_string(), remoto.to_string()],
    )?;
    Ok(())
}

/// Primer conflicto pendiente.
pub fn primer_conflicto(conn: &Connection) -> Result<Option<Conflicto>, DbError> {
    let fila = conn
        .query_row(
            "SELECT id, uuid, COALESCE(local_json, '{}'), COALESCE(remoto_json, '{}')
               FROM sync_conflicto ORDER BY id LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    Ok(fila.map(|(id, uuid, local, remoto)| Conflicto {
        id,
        uuid,
        local: serde_json::from_str(&local).unwrap_or(Value::Null),
        remoto: serde_json::from_str(&remoto).unwrap_or(Value::Null),
    }))
}

/// Borra un conflicto ya resuelto.
pub fn borrar_conflicto(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM sync_conflicto WHERE id = ?1", [id])?;
    Ok(())
}

// ------------------------------------------------------ Enumeraciones de C

fn resultado_texto(codigo: i64) -> Option<&'static str> {
    ["victoria", "empate", "derrota"]
        .get(usize::try_from(codigo - 1).ok()?)
        .copied()
}

const CLIMAS: [&str; 12] = [
    "Despejado",
    "Nublado",
    "Lluvia",
    "Ventoso",
    "Mucho Calor",
    "Mucho Frio",
    "Frio",
    "Calor",
    "Llovizna leve",
    "Lluvia Moderada",
    "Lluvia fuerte",
    "Cancha inundada",
];

fn clima_texto(codigo: i64) -> Option<&'static str> {
    CLIMAS.get(usize::try_from(codigo - 1).ok()?).copied()
}

fn tipo_texto(codigo: i64) -> &'static str {
    match codigo {
        2 => "torneo",
        3 => "entrenamiento",
        _ => "amistoso",
    }
}

fn tarjeta_texto(codigo: i64) -> &'static str {
    match codigo {
        2 => "amarilla",
        3 => "roja",
        _ => "ninguna",
    }
}

fn estado_cancha_texto(codigo: i64) -> &'static str {
    match codigo {
        1 => "Excelente",
        2 => "Buena",
        3 => "Regular",
        4 => "Mala",
        5 => "Pesima",
        _ => "No definido",
    }
}

/// Código de una enumeración que llega como texto o como número. C solo
/// aceptaba texto: un número (de la app móvil) quedaba en 0.
pub(crate) fn codigo_enum(valor: &Value, desde_texto: fn(&str) -> i64) -> i64 {
    valor
        .as_i64()
        .unwrap_or_else(|| valor.as_str().map_or(0, desde_texto))
}

pub(crate) fn resultado_codigo(texto: &str) -> i64 {
    match texto.to_lowercase().as_str() {
        "victoria" | "1" => 1,
        "empate" | "2" => 2,
        "derrota" | "3" => 3,
        _ => 0,
    }
}

/// Franja horaria de C: los textos de `dia_to_text` (1 Madrugada … 6 Noche),
/// con o sin tildes, o el número. C los convertía mal al importar ("Dia"=1,
/// "Tarde"=2, "Noche"=3).
pub(crate) fn dia_codigo(texto: &str) -> i64 {
    match texto.trim().to_lowercase().as_str() {
        "madrugada" => 1,
        "manana" | "mañana" => 2,
        "mediodia" | "mediodía" => 3,
        "tarde" => 4,
        "atardecer" => 5,
        "noche" => 6,
        otro => otro
            .parse()
            .ok()
            .filter(|n| (0..=6).contains(n))
            .unwrap_or(0),
    }
}

pub(crate) fn clima_codigo(texto: &str) -> i64 {
    if let Some(posicion) = CLIMAS
        .iter()
        .position(|clima| clima.eq_ignore_ascii_case(texto))
    {
        return posicion as i64 + 1;
    }
    // Nombres usados por la app móvil.
    for (parte, codigo) in [
        ("Despejado", 1),
        ("Soleado", 1),
        ("Nublado", 2),
        ("nubes", 2),
        ("Llovizna", 9),
        ("Lluvia", 3),
        ("Tormenta", 11),
        ("Nieve", 7),
    ] {
        if texto.contains(parte) {
            return codigo;
        }
    }
    0
}

fn tipo_codigo(texto: &str) -> i64 {
    match texto.to_lowercase().as_str() {
        "entrenamiento" | "3" => 3,
        "torneo" | "oficial" | "temporada" | "2" => 2,
        _ => 1,
    }
}

fn tarjeta_codigo(texto: &str) -> i64 {
    match texto.to_lowercase().as_str() {
        "amarilla" | "2" => 2,
        "roja" | "doble_amarilla" | "3" => 3,
        _ => 1,
    }
}

fn estado_cancha_codigo(texto: &str) -> i64 {
    match texto.to_lowercase().as_str() {
        "excelente" => 1,
        "buena" => 2,
        "regular" => 3,
        "mala" => 4,
        "pesima" | "pésima" => 5,
        _ => 0,
    }
}

/// 5/7/8/11 a partir de "F5", "Futbol 7", ... (`formato_numero`).
fn formato_numero(texto: &str) -> i64 {
    // "11" antes que "1": C probaba "5", "7", "8" y luego "11".
    [("11", 11), ("5", 5), ("7", 7), ("8", 8)]
        .into_iter()
        .find(|(parte, _)| texto.contains(parte))
        .map_or(0, |(_, numero)| numero)
}

// ------------------------------------------------------------- Partidos

fn columnas(conn: &Connection, tabla: &str) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{tabla}\")"))?;
    let nombres = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(nombres)
}

fn nombre_por_id(conn: &Connection, tabla: &str, id: i64) -> Result<String, DbError> {
    Ok(conn
        .query_row(
            &format!("SELECT nombre FROM {tabla} WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_default())
}

/// Id por nombre (sin distinguir mayúsculas); si no existe se crea activa
/// (`sync_id_por_nombre`). Sin nombre ni defecto devuelve 0.
fn id_por_nombre(
    conn: &Connection,
    tabla: &str,
    nombre: &str,
    defecto: Option<&str>,
) -> Result<i64, DbError> {
    let usado = if nombre.is_empty() {
        defecto.unwrap_or("")
    } else {
        nombre
    };
    if usado.is_empty() {
        return Ok(0);
    }
    if let Some(id) = conn
        .query_row(
            &format!("SELECT id FROM {tabla} WHERE nombre = ?1 COLLATE NOCASE LIMIT 1"),
            [usado],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        &format!("INSERT INTO {tabla} (nombre, activa) VALUES (?1, 1)"),
        [usado],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Payload local de un partido con los nombres de columna de MiFutbolC
/// (`sync_payload_local`): enumeraciones como texto, fecha ISO, `precio` en
/// pesos y los nombres de cancha, camiseta y botín.
pub fn payload_partido(conn: &Connection, uuid: &str) -> Result<Option<Value>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM partido WHERE uuid = ?1 LIMIT 1")?;
    let nombres: Vec<String> = stmt.column_names().map(String::from).collect();
    let fila = stmt
        .query_row([uuid], |row| {
            (0..nombres.len())
                .map(|indice| row.get::<_, SqlValue>(indice))
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .optional()?;
    let Some(fila) = fila else {
        return Ok(None);
    };
    let mut payload = Map::new();
    let (mut cancha, mut camiseta, mut botin) = (0, 0, 0);
    let mut actualizado = String::new();
    for (nombre, valor) in nombres.iter().zip(fila) {
        let clave = nombre.clone();
        match (nombre.as_str(), valor) {
            ("cancha_id", SqlValue::Integer(id)) => cancha = id,
            ("camiseta_id", SqlValue::Integer(id)) => camiseta = id,
            ("botin_id", SqlValue::Integer(id)) => botin = id,
            ("updated_at", SqlValue::Text(texto)) => actualizado = texto,
            (
                "id" | "cancha_id" | "camiseta_id" | "botin_id" | "created_at" | "updated_at"
                | "uuid",
                _,
            )
            | (_, SqlValue::Null | SqlValue::Blob(_)) => {}
            ("resultado", SqlValue::Integer(codigo)) => {
                if let Some(texto) = resultado_texto(codigo) {
                    payload.insert(clave, json!(texto));
                }
            }
            ("clima", SqlValue::Integer(codigo)) => {
                if let Some(texto) = clima_texto(codigo) {
                    payload.insert(clave, json!(texto));
                }
            }
            ("tipo_partido", SqlValue::Integer(codigo)) => {
                payload.insert(clave, json!(tipo_texto(codigo)));
            }
            ("tarjeta", SqlValue::Integer(codigo)) => {
                payload.insert(clave, json!(tarjeta_texto(codigo)));
            }
            ("estado_cancha", SqlValue::Integer(codigo)) => {
                payload.insert(clave, json!(estado_cancha_texto(codigo)));
            }
            ("formato_partido", SqlValue::Text(texto)) => {
                payload.insert(clave, json!(formato_numero(&texto)));
                payload.insert("formato_partido_texto".into(), json!(texto));
            }
            ("fecha_hora", SqlValue::Text(texto)) => {
                payload.insert(clave, json!(a_iso(&texto)));
            }
            // C guarda el precio en pesos enteros.
            ("precio_centavos", SqlValue::Integer(centavos)) => {
                payload.insert("precio".into(), json!(centavos / 100));
            }
            // -1 es "sin marcador".
            ("goles_equipo" | "goles_rival", SqlValue::Integer(valor)) if valor < 0 => {}
            (_, SqlValue::Integer(valor)) => {
                payload.insert(clave, json!(valor));
            }
            (_, SqlValue::Real(valor)) => {
                payload.insert(clave, json!(valor));
            }
            (_, SqlValue::Text(texto)) => {
                payload.insert(clave, json!(texto));
            }
        }
    }
    payload.insert("uuid".into(), json!(uuid));
    payload.insert(
        "cancha_nombre".into(),
        json!(nombre_por_id(conn, "cancha", cancha)?),
    );
    payload.insert(
        "camiseta_nombre".into(),
        json!(nombre_por_id(conn, "camiseta", camiseta)?),
    );
    payload.insert(
        "botin_nombre".into(),
        json!(nombre_por_id(conn, "botin", botin)?),
    );
    let actualizado = if actualizado.is_empty() {
        payload
            .get("fecha_hora")
            .and_then(Value::as_str)
            .unwrap_or("1970-01-01T00:00:00")
            .to_string()
    } else {
        a_iso(&actualizado)
    };
    payload.insert("actualizado_en".into(), json!(actualizado));
    Ok(Some(Value::Object(payload)))
}

fn texto<'a>(valor: &'a Value, clave: &str) -> &'a str {
    valor.get(clave).and_then(Value::as_str).unwrap_or("")
}

/// Valor SQL de un valor JSON; los arreglos se unen con comas (tags).
fn json_a_sql(valor: &Value) -> SqlValue {
    match valor {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Number(n) => n.as_i64().map_or_else(
            || SqlValue::Real(n.as_f64().unwrap_or(0.0)),
            SqlValue::Integer,
        ),
        Value::String(s) => SqlValue::Text(s.clone()),
        Value::Array(items) => SqlValue::Text(
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => SqlValue::Text(valor.to_string()),
    }
}

/// "AAAA-MM-DDTHH:MM" → "AAAA-MM-DD HH:MM:SS" (formato de la tabla).
fn fecha_local(texto: &str) -> String {
    let mut fecha = texto.replacen('T', " ", 1);
    if fecha.len() == 16 {
        fecha.push_str(":00");
    }
    fecha
}

/// Aplica un registro remoto de `partidos_sync` (`sync_aplicar_remoto`). Sin
/// `forzar`, descarta el remoto si la copia local no es más vieja. Devuelve si
/// cambió algo.
pub fn aplicar_partido(
    conn: &mut Connection,
    registro: &Value,
    forzar: bool,
) -> Result<bool, DbError> {
    let uuid = texto(registro, "uuid");
    if uuid.is_empty() {
        return Ok(false);
    }
    if registro.get("deleted") == Some(&Value::Bool(true)) {
        return eliminar_partido(conn, uuid);
    }
    let mapa = match registro.get("payload") {
        Some(Value::Object(payload)) => payload.clone(),
        _ => {
            let mut minimo = Map::new();
            minimo.insert("fecha_hora".into(), json!(texto(registro, "fecha")));
            minimo.insert(
                "goles".into(),
                json!(registro["goles"].as_i64().unwrap_or(0)),
            );
            minimo.insert(
                "asistencias".into(),
                json!(registro["asistencias"].as_i64().unwrap_or(0)),
            );
            minimo
        }
    };
    let payload = Value::Object(mapa);
    let remoto_actualizado = texto(registro, "updated_at_client");
    let tx = conn.transaction()?;
    let local: Option<(i64, String)> = tx
        .query_row(
            "SELECT id, updated_at FROM partido WHERE uuid = ?1 LIMIT 1",
            [uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((_, local_actualizado)) = &local
        && !forzar
        && !local_actualizado.is_empty()
        && !remoto_actualizado.is_empty()
        && comparar_fechas(remoto_actualizado, local_actualizado) != Ordering::Greater
    {
        return Ok(false);
    }
    let mut fecha = texto(&payload, "fecha_hora").to_string();
    if fecha.is_empty() {
        fecha = texto(registro, "fecha").to_string();
    }
    if fecha.is_empty() {
        return Ok(false);
    }
    let fecha = fecha_local(&fecha);
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])?;

    let tabla = columnas(&tx, "partido")?;
    let mut nombres: Vec<String> = Vec::new();
    let mut valores: Vec<SqlValue> = Vec::new();
    for (clave, valor) in payload.as_object().into_iter().flatten() {
        if valor.is_null() {
            // Las columnas de texto son NOT NULL: un nulo remoto no pisa lo local.
            continue;
        }
        let (columna, sql) = match clave.as_str() {
            "id"
            | "cancha_id"
            | "camiseta_id"
            | "botin_id"
            | "equipo_id"
            | "cancha_nombre"
            | "camiseta_nombre"
            | "botin_nombre"
            | "formato_partido_texto"
            | "actualizado_en"
            | "fecha_hora"
            | "uuid"
            | "mes_anio"
            | "created_at"
            | "updated_at"
            | "precio_centavos" => continue,
            "precio" => (
                "precio_centavos",
                SqlValue::Integer(valor.as_f64().unwrap_or(0.0).round() as i64 * 100),
            ),
            "resultado" => (
                "resultado",
                SqlValue::Integer(codigo_enum(valor, resultado_codigo)),
            ),
            "clima" => ("clima", SqlValue::Integer(codigo_enum(valor, clima_codigo))),
            "tipo_partido" => (
                "tipo_partido",
                SqlValue::Integer(codigo_enum(valor, tipo_codigo)),
            ),
            "tarjeta" => (
                "tarjeta",
                SqlValue::Integer(codigo_enum(valor, tarjeta_codigo)),
            ),
            "estado_cancha" => (
                "estado_cancha",
                SqlValue::Integer(codigo_enum(valor, estado_cancha_codigo)),
            ),
            "formato_partido" => {
                let numero = valor
                    .as_i64()
                    .unwrap_or_else(|| valor.as_str().map_or(0, formato_numero));
                let texto_remoto = payload
                    .get("formato_partido_texto")
                    .and_then(Value::as_str)
                    .or_else(|| valor.as_str())
                    .unwrap_or("");
                let formato = if numero > 0 {
                    format!("F{numero}")
                } else {
                    texto_remoto.to_string()
                };
                ("formato_partido", SqlValue::Text(formato))
            }
            otra => (otra, json_a_sql(valor)),
        };
        if tabla.iter().any(|nombre| nombre == columna) {
            nombres.push(columna.to_string());
            valores.push(sql);
        }
    }
    let cancha_id = id_por_nombre(
        &tx,
        "cancha",
        texto(&payload, "cancha_nombre"),
        Some("Sin cancha"),
    )?;
    let camiseta_id = id_por_nombre(
        &tx,
        "camiseta",
        texto(&payload, "camiseta_nombre"),
        Some("Sin camiseta"),
    )?;
    let botin_id = id_por_nombre(&tx, "botin", texto(&payload, "botin_nombre"), None)?;
    let mes_anio = fecha.get(..7).unwrap_or("").to_string();
    for (columna, valor) in [
        ("fecha_hora", SqlValue::Text(fecha)),
        ("mes_anio", SqlValue::Text(mes_anio)),
        ("uuid", SqlValue::Text(uuid.to_string())),
        ("cancha_id", SqlValue::Integer(cancha_id)),
        ("camiseta_id", SqlValue::Integer(camiseta_id)),
    ] {
        nombres.push(columna.into());
        valores.push(valor);
    }
    if botin_id > 0 {
        nombres.push("botin_id".into());
        valores.push(SqlValue::Integer(botin_id));
    }
    if !remoto_actualizado.is_empty() {
        nombres.push("updated_at".into());
        valores.push(SqlValue::Text(remoto_actualizado.replacen('T', " ", 1)));
    }
    let sql = if let Some((id, _)) = local {
        valores.push(SqlValue::Integer(id));
        format!(
            "UPDATE partido SET {} WHERE id = ?",
            nombres
                .iter()
                .map(|nombre| format!("\"{nombre}\" = ?"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!(
            "INSERT INTO partido ({}) VALUES ({})",
            nombres
                .iter()
                .map(|nombre| format!("\"{nombre}\""))
                .collect::<Vec<_>>()
                .join(", "),
            vec!["?"; nombres.len()].join(", ")
        )
    };
    tx.execute(&sql, params_from_iter(valores))?;
    // El cambio vino del servidor: no se reenvía.
    marcar_hecho(&tx, uuid)?;
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(true)
}

/// Borra el partido local con ese uuid (borrado remoto). Devuelve si había alguno.
pub fn eliminar_partido(conn: &mut Connection, uuid: &str) -> Result<bool, DbError> {
    let tx = conn.transaction()?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])?;
    let ids = {
        let mut stmt = tx.prepare("SELECT id FROM partido WHERE uuid = ?1")?;
        stmt.query_map([uuid], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    for id in &ids {
        partido_repo::borrar_con_dependencias(&tx, *id)?;
    }
    marcar_hecho(&tx, uuid)?;
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(!ids.is_empty())
}

/// "Conservar local": la copia local pasa a ser la más nueva y el trigger la
/// vuelve a encolar.
pub fn tocar_partido(conn: &mut Connection, uuid: &str) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])?;
    tx.execute(
        "UPDATE partido SET updated_at = datetime('now','localtime') WHERE uuid = ?1",
        [uuid],
    )?;
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(())
}

// ------------------------------------------------------- Camisetas y botines

/// Entidad sincronizada aparte de los partidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entidad {
    /// `camisetas_sync`.
    Camiseta,
    /// `botines_sync`.
    Botin,
}

#[derive(Clone, Copy)]
enum Tipo {
    Texto,
    Entero,
    Bool,
}

const CAMPOS_CAMISETA: &[(&str, Tipo, i64)] = &[
    ("color_principal", Tipo::Texto, 0),
    ("color_secundario", Tipo::Texto, 0),
    ("marca", Tipo::Texto, 0),
    ("modelo", Tipo::Texto, 0),
    ("temporada", Tipo::Texto, 0),
    ("estado_fisico", Tipo::Texto, 0),
    ("fecha_compra", Tipo::Texto, 0),
    ("costo_centavos", Tipo::Entero, 0),
    ("observaciones", Tipo::Texto, 0),
    ("proveedor", Tipo::Texto, 0),
    ("fue_regalo", Tipo::Bool, 0),
    ("regalo_de", Tipo::Texto, 0),
    ("activa", Tipo::Bool, 1),
];

const CAMPOS_BOTIN: &[(&str, Tipo, i64)] = &[
    ("marca", Tipo::Texto, 0),
    ("modelo", Tipo::Texto, 0),
    ("talla", Tipo::Texto, 0),
    ("superficie", Tipo::Texto, 0),
    ("estado_fisico", Tipo::Texto, 0),
    ("fecha_compra", Tipo::Texto, 0),
    ("costo_centavos", Tipo::Entero, 0),
    ("activa", Tipo::Bool, 1),
];

/// `updated_at` local como marca UTC ISO (formato de `sync_entidades.c`).
const ACTUALIZADO_UTC: &str =
    "COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ', NULLIF(updated_at, ''), 'utc'), '')";

impl Entidad {
    /// Tabla local.
    #[must_use]
    pub const fn tabla(self) -> &'static str {
        match self {
            Self::Camiseta => "camiseta",
            Self::Botin => "botin",
        }
    }

    /// Colección de PocketBase.
    #[must_use]
    pub const fn coleccion(self) -> &'static str {
        match self {
            Self::Camiseta => "camisetas_sync",
            Self::Botin => "botines_sync",
        }
    }

    /// Entidad de una lápida.
    #[must_use]
    pub fn desde_tabla(tabla: &str) -> Option<Self> {
        match tabla {
            "camiseta" => Some(Self::Camiseta),
            "botin" => Some(Self::Botin),
            _ => None,
        }
    }

    const fn campos(self) -> &'static [(&'static str, Tipo, i64)] {
        match self {
            Self::Camiseta => CAMPOS_CAMISETA,
            Self::Botin => CAMPOS_BOTIN,
        }
    }
}

/// Fila local lista para subir.
#[derive(Debug, Clone, PartialEq)]
pub struct FilaEntidad {
    /// Uuid.
    pub uuid: String,
    /// Nombre.
    pub nombre: String,
    /// Payload con las columnas de C.
    pub payload: Value,
    /// Última modificación (UTC ISO).
    pub actualizado: String,
}

/// Filas con uuid de la entidad.
pub fn filas_entidad(conn: &Connection, entidad: Entidad) -> Result<Vec<FilaEntidad>, DbError> {
    let campos = entidad.campos();
    let lista = campos
        .iter()
        .map(|(nombre, _, _)| *nombre)
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT uuid, nombre, {lista}, {ACTUALIZADO_UTC} FROM {}
          WHERE uuid IS NOT NULL AND TRIM(uuid) <> '' ORDER BY id",
        entidad.tabla()
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt
        .query_map([], |row| {
            let nombre: String = row.get(1)?;
            let mut payload = Map::new();
            payload.insert("nombre".into(), json!(nombre));
            for (indice, (campo, tipo, _)) in campos.iter().enumerate() {
                let valor = match tipo {
                    Tipo::Texto => json!(row.get::<_, String>(indice + 2)?),
                    Tipo::Entero => json!(row.get::<_, i64>(indice + 2)?),
                    Tipo::Bool => json!(row.get::<_, i64>(indice + 2)? != 0),
                };
                payload.insert((*campo).into(), valor);
            }
            Ok(FilaEntidad {
                uuid: row.get(0)?,
                nombre,
                payload: Value::Object(payload),
                actualizado: row.get(campos.len() + 2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}

fn actualizado_local(
    conn: &Connection,
    entidad: Entidad,
    uuid: &str,
) -> Result<Option<String>, DbError> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {ACTUALIZADO_UTC} FROM {} WHERE uuid = ?1",
                entidad.tabla()
            ),
            [uuid],
            |row| row.get(0),
        )
        .optional()?)
}

/// Qué hizo la descarga de un registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Descarga {
    /// Insertado o actualizado.
    Importado,
    /// Borrado (o desactivado si lo usa algún partido).
    Eliminado,
    /// Sin cambios: la copia local es igual o más nueva.
    Omitido,
}

/// Aplica un registro de `camisetas_sync`/`botines_sync` (`procesar_item_descarga`).
/// Gana la marca más nueva; sin uuid local se adopta la fila con el mismo nombre.
pub fn aplicar_entidad(
    conn: &mut Connection,
    entidad: Entidad,
    item: &Value,
) -> Result<Descarga, DbError> {
    let uuid = texto(item, "uuid");
    if uuid.is_empty() {
        return Ok(Descarga::Omitido);
    }
    let actualizado = match texto(item, "updated_at_client") {
        "" => jiff::Timestamp::now()
            .strftime("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        marca => marca.to_string(),
    };
    let tabla = entidad.tabla();
    let tx = conn.transaction()?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])?;
    let local = actualizado_local(&tx, entidad, uuid)?;
    let mas_nuevo_local =
        |local: &str, orden: Ordering| comparar_fechas(local, &actualizado) >= orden;

    let resultado = if item.get("deleted") == Some(&Value::Bool(true)) {
        if local
            .as_deref()
            .is_some_and(|l| mas_nuevo_local(l, Ordering::Greater))
        {
            Descarga::Omitido
        } else {
            let borrado = match tx.execute(&format!("DELETE FROM {tabla} WHERE uuid = ?1"), [uuid])
            {
                Ok(filas) => filas,
                // Una camiseta usada por partidos no se borra (FK RESTRICT): se desactiva.
                Err(error) => {
                    let error = DbError::from(error);
                    if !error.is_foreign_key() {
                        return Err(error);
                    }
                    tx.execute(
                        &format!("UPDATE {tabla} SET activa = 0 WHERE uuid = ?1"),
                        [uuid],
                    )?
                }
            };
            tx.execute(
                "DELETE FROM sync_tombstones WHERE entidad = ?1 AND uuid = ?2",
                params![tabla, uuid],
            )?;
            if borrado > 0 {
                Descarga::Eliminado
            } else {
                Descarga::Omitido
            }
        }
    } else if local
        .as_deref()
        .is_some_and(|l| mas_nuevo_local(l, Ordering::Equal))
    {
        Descarga::Omitido
    } else {
        let payload = item.get("payload").cloned().unwrap_or_else(|| json!({}));
        let nombre = match texto(&payload, "nombre") {
            "" => texto(item, "nombre").to_string(),
            nombre => nombre.to_string(),
        };
        let mut existe = local.is_some();
        let mut omitir = false;
        if !existe
            && let Some((id, local_actualizado)) = tx
                .query_row(
                    &format!(
                        "SELECT id, {ACTUALIZADO_UTC} FROM {tabla}
                          WHERE nombre = ?1 COLLATE NOCASE LIMIT 1"
                    ),
                    [&nombre],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
        {
            tx.execute(
                &format!("UPDATE {tabla} SET uuid = ?1 WHERE id = ?2"),
                params![uuid, id],
            )?;
            existe = true;
            omitir = mas_nuevo_local(&local_actualizado, Ordering::Greater);
        }
        if omitir {
            Descarga::Omitido
        } else {
            let campos = entidad.campos();
            let mut valores: Vec<SqlValue> = vec![SqlValue::Text(nombre)];
            for (campo, tipo, defecto) in campos {
                let valor = payload.get(*campo).unwrap_or(&Value::Null);
                valores.push(match tipo {
                    Tipo::Texto => SqlValue::Text(valor.as_str().unwrap_or("").to_string()),
                    Tipo::Entero | Tipo::Bool => SqlValue::Integer(
                        valor
                            .as_i64()
                            .or_else(|| valor.as_bool().map(i64::from))
                            .unwrap_or(*defecto),
                    ),
                });
            }
            valores.push(SqlValue::Text(actualizado.clone()));
            valores.push(SqlValue::Text(uuid.to_string()));
            let lista: Vec<&str> = campos.iter().map(|(campo, _, _)| *campo).collect();
            let marca_local = "COALESCE(strftime('%Y-%m-%d %H:%M:%f', ?, 'localtime'), datetime('now','localtime'))";
            let sql = if existe {
                format!(
                    "UPDATE {tabla} SET nombre = ?, {}, updated_at = {marca_local} WHERE uuid = ?",
                    lista
                        .iter()
                        .map(|campo| format!("{campo} = ?"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                format!(
                    "INSERT INTO {tabla} (nombre, {}, updated_at, uuid) VALUES (?, {}, {marca_local}, ?)",
                    lista.join(", "),
                    vec!["?"; lista.len()].join(", ")
                )
            };
            tx.execute(&sql, params_from_iter(valores))?;
            Descarga::Importado
        }
    };
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(resultado)
}

/// Lápida de una camiseta o botín borrado localmente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lapida {
    /// Id de la fila.
    pub id: i64,
    /// `camiseta` o `botin`.
    pub entidad: String,
    /// Uuid borrado.
    pub uuid: String,
    /// Cuándo se borró (UTC ISO).
    pub creado_en: String,
}

/// Lápidas pendientes de enviar.
pub fn lapidas(conn: &Connection) -> Result<Vec<Lapida>, DbError> {
    let mut stmt =
        conn.prepare("SELECT id, entidad, uuid, creado_en FROM sync_tombstones ORDER BY id")?;
    let filas = stmt
        .query_map([], |row| {
            Ok(Lapida {
                id: row.get(0)?,
                entidad: row.get(1)?,
                uuid: row.get(2)?,
                creado_en: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}

/// Borra una lápida ya enviada.
pub fn borrar_lapida(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM sync_tombstones WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn base_con_partido() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (id, nombre) VALUES (1, 'La Bombonera');
                 INSERT INTO camiseta (id, nombre) VALUES (1, 'Titular');
                 INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, mes_anio, goles,
                     resultado, clima, precio_centavos, formato_partido, uuid, updated_at)
                 VALUES (1, 1, '2026-03-10 20:00:00', '2026-03', 2, 1, 3, 150000, 'F5',
                     'u-1', '2026-03-10 22:00:00');",
            )
            .unwrap();
        db
    }

    #[test]
    fn comparar_fechas_ignora_el_separador() {
        assert_eq!(
            comparar_fechas("2026-01-01T10:00:00", "2026-01-01 10:00:00"),
            Ordering::Equal
        );
        assert_eq!(
            comparar_fechas("2026-01-01T10:00:01", "2026-01-01 10:00:00"),
            Ordering::Greater
        );
    }

    #[test]
    fn payload_usa_los_nombres_y_textos_de_c() {
        let db = base_con_partido();
        let payload = payload_partido(db.conn(), "u-1").unwrap().unwrap();
        assert_eq!(payload["resultado"], "victoria");
        assert_eq!(payload["clima"], "Lluvia");
        assert_eq!(payload["precio"], 1500);
        assert_eq!(payload["formato_partido"], 5);
        assert_eq!(payload["fecha_hora"], "2026-03-10T20:00:00");
        assert_eq!(payload["cancha_nombre"], "La Bombonera");
        assert_eq!(payload["actualizado_en"], "2026-03-10T22:00:00");
        assert!(payload.get("goles_equipo").is_none(), "-1 = sin marcador");
        assert!(payload.get("id").is_none());
    }

    #[test]
    fn aplicar_remoto_gana_solo_si_es_mas_nuevo() {
        let mut db = base_con_partido();
        let mut registro = json!({
            "uuid": "u-1",
            "updated_at_client": "2026-03-10T21:00:00",
            "payload": payload_partido(db.conn(), "u-1").unwrap().unwrap(),
        });
        registro["payload"]["goles"] = json!(5);
        // Remoto más viejo que el local (22:00): se descarta.
        assert!(!aplicar_partido(db.conn_mut(), &registro, false).unwrap());
        // Forzado (conservar servidor) o más nuevo: se aplica.
        registro["updated_at_client"] = json!("2026-03-11T08:00:00");
        assert!(aplicar_partido(db.conn_mut(), &registro, false).unwrap());
        let (goles, precio, actualizado): (i64, i64, String) = db
            .conn()
            .query_row(
                "SELECT goles, precio_centavos, updated_at FROM partido WHERE uuid = 'u-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!((goles, precio), (5, 150_000));
        assert_eq!(actualizado, "2026-03-11 08:00:00");
        // El cambio vino del servidor: no queda en la cola.
        assert_eq!(contar_pendientes(db.conn()).unwrap(), 0);
        let undo: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM undo_historial WHERE tabla = 'partido' AND operacion = 'editar'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            undo, 0,
            "lo aplicado desde el servidor no entra al deshacer"
        );
    }

    #[test]
    fn aplicar_remoto_nuevo_crea_partido_y_cancha() {
        let mut db = Database::open_in_memory().unwrap();
        let registro = json!({
            "uuid": "u-9",
            "fecha": "2026-05-01T18:30:00",
            "updated_at_client": "2026-05-01T20:00:00",
            "payload": {"goles": 1, "asistencias": 2, "resultado": "empate",
                        "cancha_nombre": "Nueva", "tags": ["a", "b"]}
        });
        assert!(aplicar_partido(db.conn_mut(), &registro, false).unwrap());
        let (fecha, mes, resultado, tags, cancha): (String, String, i64, String, String) = db
            .conn()
            .query_row(
                "SELECT p.fecha_hora, p.mes_anio, p.resultado, p.tags, c.nombre
                   FROM partido p JOIN cancha c ON c.id = p.cancha_id WHERE p.uuid = 'u-9'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(fecha, "2026-05-01 18:30:00");
        assert_eq!(mes, "2026-05");
        assert_eq!(resultado, 2);
        assert_eq!(tags, "a,b");
        assert_eq!(cancha, "Nueva");
        // Borrado remoto.
        let borrado = json!({"uuid": "u-9", "deleted": true});
        assert!(aplicar_partido(db.conn_mut(), &borrado, false).unwrap());
        assert!(payload_partido(db.conn(), "u-9").unwrap().is_none());
        assert_eq!(contar_pendientes(db.conn()).unwrap(), 0);
    }

    #[test]
    fn triggers_y_preparar_llenan_la_cola() {
        let mut db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (id, nombre) VALUES (1, 'C');
                 INSERT INTO camiseta (id, nombre) VALUES (1, 'K');
                 INSERT INTO partido (cancha_id, camiseta_id, fecha_hora) VALUES (1, 1, '2026-01-01 10:00:00');",
            )
            .unwrap();
        // Sin uuid no se encola; `preparar` lo asigna y el UPDATE lo encola.
        assert_eq!(contar_pendientes(db.conn()).unwrap(), 0);
        preparar(db.conn_mut()).unwrap();
        assert_eq!(contar_pendientes(db.conn()).unwrap(), 1);
        let uuid: String = db
            .conn()
            .query_row("SELECT uuid FROM camiseta WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(uuid.len(), 36);
        // Borrar una camiseta con uuid deja lápida.
        db.conn().execute("DELETE FROM partido", []).unwrap();
        db.conn().execute("DELETE FROM camiseta", []).unwrap();
        assert_eq!(lapidas(db.conn()).unwrap().len(), 1);
        let siguiente = siguiente_pendiente(db.conn(), 0).unwrap().unwrap();
        assert_eq!(siguiente.operacion, "upsert");
        assert!(
            ultimo_pendiente(db.conn(), &siguiente.uuid)
                .unwrap()
                .unwrap()
                .operacion
                == "eliminar"
        );
    }

    #[test]
    fn entidad_gana_la_marca_mas_nueva_y_adopta_por_nombre() {
        let mut db = Database::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO camiseta (nombre, marca, updated_at) VALUES ('Titular', 'Vieja', '2026-01-01 10:00:00')",
                [],
            )
            .unwrap();
        let item = |marca: &str, cuando: &str| {
            json!({"uuid": "r-1", "updated_at_client": cuando,
                   "payload": {"nombre": "titular", "marca": marca, "activa": true}})
        };
        // Más viejo que la local: adopta el uuid pero no pisa.
        assert_eq!(
            aplicar_entidad(
                db.conn_mut(),
                Entidad::Camiseta,
                &item("Remota", "2000-01-01T00:00:00.000Z")
            )
            .unwrap(),
            Descarga::Omitido
        );
        // Más nuevo: actualiza la misma fila (ya con el uuid remoto).
        assert_eq!(
            aplicar_entidad(
                db.conn_mut(),
                Entidad::Camiseta,
                &item("Remota", "2099-01-01T00:00:00.000Z")
            )
            .unwrap(),
            Descarga::Importado
        );
        let filas = filas_entidad(db.conn(), Entidad::Camiseta).unwrap();
        assert_eq!(filas.len(), 1);
        assert_eq!(filas[0].uuid, "r-1");
        assert_eq!(filas[0].payload["marca"], "Remota");
        // Borrado remoto sin lápida propia.
        let borrado = json!({"uuid": "r-1", "deleted": true, "updated_at_client": "2099-02-01T00:00:00.000Z"});
        assert_eq!(
            aplicar_entidad(db.conn_mut(), Entidad::Camiseta, &borrado).unwrap(),
            Descarga::Eliminado
        );
        assert!(lapidas(db.conn()).unwrap().is_empty());
    }
}
