//! Repositorio de partidos (§12.7/§12.8). Incluye transacción financiera.

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use super::error::DbError;
use super::helpers::Page;
use crate::domain::Cents;
use crate::domain::dates;
use crate::domain::models::{
    Atajaste, Clima, DolorFisico, EstadoCancha, FormatoPartido, FranjaHoraria, Partido,
    PartidoFilter, PartidoResumen, PartidoRow, Resultado, Tarjeta, TipoPartido,
};

/// Columnas del partido en el orden de [`map_partido`].
const COLUMNS: &str = "id, cancha_id, camiseta_id, botin_id, fecha_hora, mes_anio, goles,
    asistencias, resultado, rendimiento_general, cansancio, estado_animo,
    comentario_personal, clima, dia, precio_centavos, tipo_partido, rival_nombre,
    tipo_rival, posicion_jugada, minutos_jugados, intensidad, condicion_cancha,
    arbitraje, eventos_clave, rating_tecnico, rating_fisico, rating_mental,
    estado_cancha, goles_equipo, goles_rival, formato_partido, tarjeta,
    goles_en_contra, dolor_fisico, temperatura_c, arbitraje_score, lo_mejor,
    que_mejorar, tags, goles_detalle, asistencias_detalle,
    atajaste_todo_el_partido, diferencia_gol";

const FILTER_SQL: &str = "WHERE (?1 = ''
        OR p.rival_nombre LIKE '%' || ?1 || '%' COLLATE NOCASE
        OR p.comentario_personal LIKE '%' || ?1 || '%' COLLATE NOCASE
        OR p.tags LIKE '%' || ?1 || '%' COLLATE NOCASE)
   AND (?2 = 0 OR p.cancha_id = ?2)
   AND (?3 = 0 OR p.camiseta_id = ?3)
   AND (?4 = -1 OR p.resultado = ?4)
   AND (?5 = -1 OR p.tipo_partido = ?5)
   AND (?6 = '' OR p.fecha_hora >= ?6)
   AND (?7 = '' OR p.fecha_hora <= ?7 || ' 23:59:59')
   AND (?8 = 0 OR EXISTS (SELECT 1 FROM partido_meta m WHERE m.partido_id = p.id AND m.favorito = 1))";

/// Lista paginada de partidos.
pub fn list(
    conn: &Connection,
    filter: &PartidoFilter,
    page: Page,
) -> Result<Vec<PartidoRow>, DbError> {
    let sql = format!(
        "SELECT p.id, p.fecha_hora, ca.nombre, c.nombre, p.rival_nombre, p.goles,
                p.asistencias, p.resultado, p.tipo_partido, p.precio_centavos,
                CASE WHEN m.favorito = 1 THEN 1 ELSE 0 END AS favorito
           FROM partido p
           JOIN cancha ca ON ca.id = p.cancha_id
           JOIN camiseta c ON c.id = p.camiseta_id
           LEFT JOIN partido_meta m ON m.partido_id = p.id
           {FILTER_SQL}
          ORDER BY {}
          LIMIT ?9 OFFSET ?10",
        filter.orden.sql_order()
    );
    let (desde, hasta) = fechas_filtro(filter);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(
            params![
                filter.texto.trim(),
                filter.cancha_id.unwrap_or(0),
                filter.camiseta_id.unwrap_or(0),
                filter.resultado.map_or(-1, Resultado::as_i64),
                filter.tipo_partido.map_or(-1, TipoPartido::as_i64),
                desde,
                hasta,
                i64::from(filter.favorito),
                page.sql_limit(),
                page.sql_offset(),
            ],
            |row| {
                Ok(PartidoRow {
                    id: row.get(0)?,
                    fecha_hora: parse_fecha(row, 1)?,
                    cancha: row.get(2)?,
                    camiseta: row.get(3)?,
                    rival: row.get(4)?,
                    goles: row.get(5)?,
                    asistencias: row.get(6)?,
                    resultado: Resultado::try_from(row.get::<_, i64>(7)?)
                        .unwrap_or(Resultado::NoAplica),
                    tipo_partido: TipoPartido::try_from(row.get::<_, i64>(8)?)
                        .unwrap_or(TipoPartido::Amistoso),
                    precio: Cents(row.get(9)?),
                    favorito: row.get::<_, i64>(10)? != 0,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Cuenta los partidos que cumplen el filtro.
pub fn count(conn: &Connection, filter: &PartidoFilter) -> Result<i64, DbError> {
    let sql = format!("SELECT COUNT(*) FROM partido p {FILTER_SQL}");
    let (desde, hasta) = fechas_filtro(filter);
    let total = conn.query_row(
        &sql,
        params![
            filter.texto.trim(),
            filter.cancha_id.unwrap_or(0),
            filter.camiseta_id.unwrap_or(0),
            filter.resultado.map_or(-1, Resultado::as_i64),
            filter.tipo_partido.map_or(-1, TipoPartido::as_i64),
            desde,
            hasta,
            i64::from(filter.favorito),
        ],
        |row| row.get(0),
    )?;
    Ok(total)
}

/// Resumen agregado del filtro actual.
pub fn resumen(conn: &Connection, filter: &PartidoFilter) -> Result<PartidoResumen, DbError> {
    let sql = format!(
        "SELECT COUNT(*),
                COALESCE(SUM(p.goles), 0),
                COALESCE(SUM(p.asistencias), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 3 THEN 1 ELSE 0 END), 0)
           FROM partido p {FILTER_SQL}"
    );
    let (desde, hasta) = fechas_filtro(filter);
    conn.query_row(
        &sql,
        params![
            filter.texto.trim(),
            filter.cancha_id.unwrap_or(0),
            filter.camiseta_id.unwrap_or(0),
            filter.resultado.map_or(-1, Resultado::as_i64),
            filter.tipo_partido.map_or(-1, TipoPartido::as_i64),
            desde,
            hasta,
            i64::from(filter.favorito),
        ],
        |row| {
            Ok(PartidoResumen {
                total: row.get(0)?,
                goles: row.get(1)?,
                asistencias: row.get(2)?,
                victorias: row.get(3)?,
                empates: row.get(4)?,
                derrotas: row.get(5)?,
            })
        },
    )
    .map_err(Into::into)
}

/// Obtiene un partido completo.
pub fn get(conn: &Connection, id: i64) -> Result<Partido, DbError> {
    let sql = format!("SELECT {COLUMNS} FROM partido WHERE id = ?1");
    let mut partido: Partido =
        conn.query_row(&sql, [id], map_partido)
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entidad: "Partido",
                    id,
                },
                other => other.into(),
            })?;
    let favorito: Option<i64> = conn
        .query_row(
            "SELECT favorito FROM partido_meta WHERE partido_id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    partido.favorito = favorito.unwrap_or(0) != 0;

    let tags_tabla = list_tags(conn, id)?;
    if !tags_tabla.is_empty() {
        partido.tags = tags_tabla;
    }
    Ok(partido)
}

/// Inserta un partido y su transacción financiera si corresponde.
pub fn insert(conn: &mut Connection, partido: &mut Partido) -> Result<i64, DbError> {
    let tx = conn.transaction()?;
    let id = insert_row(&tx, partido)?;
    partido.id = id;
    replace_tags(&tx, id, &partido.tags)?;
    sync_financiamiento(&tx, partido, id)?;
    tx.commit()?;
    Ok(id)
}

/// Actualiza un partido completo y sincroniza la transacción automática.
pub fn update(conn: &mut Connection, partido: &Partido) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    let updated = tx.execute(
        "UPDATE partido SET
            cancha_id = ?1, camiseta_id = ?2, botin_id = ?3, fecha_hora = ?4,
            mes_anio = ?5, goles = ?6, asistencias = ?7, resultado = ?8,
            rendimiento_general = ?9, cansancio = ?10, estado_animo = ?11,
            comentario_personal = ?12, clima = ?13, dia = ?14, precio_centavos = ?15,
            tipo_partido = ?16, rival_nombre = ?17, tipo_rival = ?18,
            posicion_jugada = ?19, minutos_jugados = ?20, intensidad = ?21,
            condicion_cancha = ?22, arbitraje = ?23, eventos_clave = ?24,
            rating_tecnico = ?25, rating_fisico = ?26, rating_mental = ?27,
            estado_cancha = ?28, goles_equipo = ?29, goles_rival = ?30,
            formato_partido = ?31, tarjeta = ?32, goles_en_contra = ?33,
            dolor_fisico = ?34, temperatura_c = ?35, arbitraje_score = ?36,
            lo_mejor = ?37, que_mejorar = ?38, tags = ?39, goles_detalle = ?40,
            asistencias_detalle = ?41, atajaste_todo_el_partido = ?42,
            diferencia_gol = ?43, updated_at = datetime('now','localtime')
          WHERE id = ?44",
        params![
            partido.cancha_id,
            partido.camiseta_id,
            partido.botin_id,
            dates::to_iso_datetime(partido.fecha_hora),
            partido.mes_anio,
            partido.goles,
            partido.asistencias,
            partido.resultado.as_i64(),
            partido.rendimiento_general,
            partido.cansancio,
            partido.estado_animo,
            partido.comentario_personal,
            partido.clima.as_i64(),
            partido.dia.as_i64(),
            partido.precio.0,
            partido.tipo_partido.as_i64(),
            partido.rival_nombre,
            partido.tipo_rival,
            partido.posicion_jugada,
            partido.minutos_jugados,
            partido.intensidad,
            partido.condicion_cancha,
            partido.arbitraje,
            partido.eventos_clave,
            partido.rating_tecnico,
            partido.rating_fisico,
            partido.rating_mental,
            partido.estado_cancha.as_i64(),
            partido.goles_equipo,
            partido.goles_rival,
            partido.formato_partido.as_str(),
            partido.tarjeta.as_i64(),
            partido.goles_en_contra,
            partido.dolor_fisico.as_i64(),
            partido.temperatura_c,
            partido.arbitraje_score,
            partido.lo_mejor,
            partido.que_mejorar,
            partido.tags.join(", "),
            partido.goles_detalle,
            partido.asistencias_detalle,
            partido.atajaste_todo_el_partido.as_i64(),
            partido.diferencia_gol,
            partido.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Partido",
            id: partido.id,
        });
    }
    replace_tags(&tx, partido.id, &partido.tags)?;
    sync_financiamiento(&tx, partido, partido.id)?;
    tx.commit()?;
    Ok(())
}

/// Elimina el partido y sus dependencias lógicas.
pub fn delete(conn: &mut Connection, id: i64) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM financiamiento WHERE partido_id = ?1", [id])?;
    tx.execute("DELETE FROM partido_tag WHERE partido_id = ?1", [id])?;
    tx.execute("DELETE FROM partido_meta WHERE partido_id = ?1", [id])?;
    let deleted = tx.execute("DELETE FROM partido WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Partido",
            id,
        });
    }
    tx.commit()?;
    Ok(())
}

/// Marca/desmarca un partido como favorito.
pub fn set_favorito(conn: &Connection, id: i64, favorito: bool) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO partido_meta (partido_id, favorito) VALUES (?1, ?2)
         ON CONFLICT(partido_id) DO UPDATE SET favorito = excluded.favorito",
        params![id, i64::from(favorito)],
    )?;
    Ok(())
}

fn fechas_filtro(filter: &PartidoFilter) -> (String, String) {
    (
        filter.desde.map(dates::to_iso_date).unwrap_or_default(),
        filter.hasta.map(dates::to_iso_date).unwrap_or_default(),
    )
}

fn insert_row(tx: &Transaction<'_>, partido: &Partido) -> Result<i64, DbError> {
    tx.execute(
        "INSERT INTO partido
            (cancha_id, camiseta_id, botin_id, fecha_hora, mes_anio, goles, asistencias,
             resultado, rendimiento_general, cansancio, estado_animo, comentario_personal,
             clima, dia, precio_centavos, tipo_partido, rival_nombre, tipo_rival,
             posicion_jugada, minutos_jugados, intensidad, condicion_cancha, arbitraje,
             eventos_clave, rating_tecnico, rating_fisico, rating_mental, estado_cancha,
             goles_equipo, goles_rival, formato_partido, tarjeta, goles_en_contra,
             dolor_fisico, temperatura_c, arbitraje_score, lo_mejor, que_mejorar, tags,
             goles_detalle, asistencias_detalle, atajaste_todo_el_partido, diferencia_gol,
             updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                 ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28,
                 ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41,
                 ?42, ?43, datetime('now','localtime'))",
        params![
            partido.cancha_id,
            partido.camiseta_id,
            partido.botin_id,
            dates::to_iso_datetime(partido.fecha_hora),
            partido.mes_anio,
            partido.goles,
            partido.asistencias,
            partido.resultado.as_i64(),
            partido.rendimiento_general,
            partido.cansancio,
            partido.estado_animo,
            partido.comentario_personal,
            partido.clima.as_i64(),
            partido.dia.as_i64(),
            partido.precio.0,
            partido.tipo_partido.as_i64(),
            partido.rival_nombre,
            partido.tipo_rival,
            partido.posicion_jugada,
            partido.minutos_jugados,
            partido.intensidad,
            partido.condicion_cancha,
            partido.arbitraje,
            partido.eventos_clave,
            partido.rating_tecnico,
            partido.rating_fisico,
            partido.rating_mental,
            partido.estado_cancha.as_i64(),
            partido.goles_equipo,
            partido.goles_rival,
            partido.formato_partido.as_str(),
            partido.tarjeta.as_i64(),
            partido.goles_en_contra,
            partido.dolor_fisico.as_i64(),
            partido.temperatura_c,
            partido.arbitraje_score,
            partido.lo_mejor,
            partido.que_mejorar,
            partido.tags.join(", "),
            partido.goles_detalle,
            partido.asistencias_detalle,
            partido.atajaste_todo_el_partido.as_i64(),
            partido.diferencia_gol,
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

fn sync_financiamiento(
    tx: &Transaction<'_>,
    partido: &Partido,
    partido_id: i64,
) -> Result<(), DbError> {
    let existente: Option<i64> = tx
        .query_row(
            "SELECT id FROM financiamiento WHERE partido_id = ?1 AND categoria = 5 LIMIT 1",
            [partido_id],
            |row| row.get(0),
        )
        .optional()?;
    if partido.precio.0 > 0 {
        let fecha = dates::to_iso_date(partido.fecha_hora.date());
        let item = format!("Partido ID: {partido_id}");
        match existente {
            Some(fin_id) => {
                tx.execute(
                    "UPDATE financiamiento
                        SET fecha = ?1, monto_centavos = ?2, item_especifico = ?3
                      WHERE id = ?4",
                    params![fecha, partido.precio.0, item, fin_id],
                )?;
            }
            None => {
                tx.execute(
                    "INSERT INTO financiamiento
                        (fecha, tipo, categoria, descripcion, monto_centavos, item_especifico, partido_id)
                     VALUES (?1, 1, 5, 'Pago por alquiler de cancha', ?2, ?3, ?4)",
                    params![fecha, partido.precio.0, item, partido_id],
                )?;
            }
        }
    } else if let Some(fin_id) = existente {
        tx.execute("DELETE FROM financiamiento WHERE id = ?1", [fin_id])?;
    }
    Ok(())
}

fn replace_tags(tx: &Transaction<'_>, id: i64, tags: &[String]) -> Result<(), DbError> {
    tx.execute("DELETE FROM partido_tag WHERE partido_id = ?1", [id])?;
    for tag in tags {
        let limpio = tag.trim();
        if !limpio.is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO partido_tag (partido_id, tag) VALUES (?1, ?2)",
                params![id, limpio],
            )?;
        }
    }
    Ok(())
}

fn list_tags(conn: &Connection, id: i64) -> Result<Vec<String>, DbError> {
    let mut stmt =
        conn.prepare("SELECT tag FROM partido_tag WHERE partido_id = ?1 ORDER BY tag")?;
    let tags = stmt
        .query_map([id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

fn parse_fecha(row: &Row<'_>, index: usize) -> rusqlite::Result<chrono::NaiveDateTime> {
    let texto: String = row.get(index)?;
    dates::parse_datetime(&texto).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(
            index,
            "fecha_hora".to_string(),
            rusqlite::types::Type::Text,
        )
    })
}

fn map_partido(row: &Row<'_>) -> rusqlite::Result<Partido> {
    let tags_texto: String = row.get(39)?;
    let tags = tags_texto
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect();
    Ok(Partido {
        id: row.get(0)?,
        cancha_id: row.get(1)?,
        camiseta_id: row.get(2)?,
        botin_id: row.get(3)?,
        fecha_hora: parse_fecha(row, 4)?,
        mes_anio: row.get(5)?,
        goles: row.get(6)?,
        asistencias: row.get(7)?,
        resultado: Resultado::try_from(row.get::<_, i64>(8)?).unwrap_or(Resultado::NoAplica),
        rendimiento_general: row.get(9)?,
        cansancio: row.get(10)?,
        estado_animo: row.get(11)?,
        comentario_personal: row.get(12)?,
        clima: Clima::try_from(row.get::<_, i64>(13)?).unwrap_or(Clima::Despejado),
        dia: FranjaHoraria::try_from(row.get::<_, i64>(14)?).unwrap_or(FranjaHoraria::NoDefinida),
        precio: Cents(row.get(15)?),
        tipo_partido: TipoPartido::try_from(row.get::<_, i64>(16)?)
            .unwrap_or(TipoPartido::Amistoso),
        rival_nombre: row.get(17)?,
        tipo_rival: row.get(18)?,
        posicion_jugada: row.get(19)?,
        minutos_jugados: row.get(20)?,
        intensidad: row.get(21)?,
        condicion_cancha: row.get(22)?,
        arbitraje: row.get(23)?,
        eventos_clave: row.get(24)?,
        rating_tecnico: row.get(25)?,
        rating_fisico: row.get(26)?,
        rating_mental: row.get(27)?,
        estado_cancha: EstadoCancha::try_from(row.get::<_, i64>(28)?)
            .unwrap_or(EstadoCancha::NoDefinido),
        goles_equipo: row.get(29)?,
        goles_rival: row.get(30)?,
        formato_partido: FormatoPartido::from_str(&row.get::<_, String>(31)?)
            .unwrap_or(FormatoPartido::F5),
        tarjeta: Tarjeta::try_from(row.get::<_, i64>(32)?).unwrap_or(Tarjeta::No),
        goles_en_contra: row.get(33)?,
        dolor_fisico: DolorFisico::try_from(row.get::<_, i64>(34)?).unwrap_or(DolorFisico::Ninguna),
        temperatura_c: row.get(35)?,
        arbitraje_score: row.get(36)?,
        lo_mejor: row.get(37)?,
        que_mejorar: row.get(38)?,
        tags,
        goles_detalle: row.get(40)?,
        asistencias_detalle: row.get(41)?,
        atajaste_todo_el_partido: Atajaste::try_from(row.get::<_, i64>(42)?)
            .unwrap_or(Atajaste::NoEspecificado),
        diferencia_gol: row.get(43)?,
        favorito: false,
    })
}
