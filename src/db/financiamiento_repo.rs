//! Repositorio de finanzas: transacciones y presupuestos (§15.4).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::Cents;
use crate::domain::dates;
use crate::domain::models::{
    AlertaPresupuesto, CategoriaFinanciera, CategoriaTotal, FinanciamientoFilter, FinanzasResumen,
    PresupuestoMensual, PresupuestoVista, TipoTransaccion, TransaccionFinanciera,
};

const FILTRO_SQL: &str = "WHERE (?1 = -1 OR f.tipo = ?1)
       AND (?2 = -1 OR f.categoria = ?2)
       AND (?3 = '' OR f.fecha >= ?3)
       AND (?4 = '' OR f.fecha <= ?4)
       AND (?5 = '' OR f.descripcion LIKE '%' || ?5 || '%' COLLATE NOCASE
                   OR f.item_especifico LIKE '%' || ?5 || '%' COLLATE NOCASE)";

/// Lista transacciones con filtro.
pub fn list(
    conn: &Connection,
    filter: &FinanciamientoFilter,
) -> Result<Vec<TransaccionFinanciera>, DbError> {
    let sql = format!(
        "SELECT f.id, f.fecha, f.tipo, f.categoria, f.descripcion, f.monto_centavos,
                f.item_especifico, f.partido_id
           FROM financiamiento f
           {FILTRO_SQL}
          ORDER BY f.fecha DESC, f.id DESC"
    );
    let desde = filter.desde.map(dates::to_iso_date).unwrap_or_default();
    let hasta = filter.hasta.map(dates::to_iso_date).unwrap_or_default();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(
            params![
                filter.tipo.map_or(-1, TipoTransaccion::as_i64),
                filter.categoria.map_or(-1, CategoriaFinanciera::as_i64),
                desde,
                hasta,
                filter.texto.trim(),
            ],
            |row| {
                let fecha_texto: String = row.get(1)?;
                let fecha = dates::parse_date(&fecha_texto).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        1,
                        "fecha".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?;
                Ok(TransaccionFinanciera {
                    id: row.get(0)?,
                    fecha,
                    tipo: TipoTransaccion::try_from(row.get::<_, i64>(2)?)
                        .unwrap_or(TipoTransaccion::Gasto),
                    categoria: CategoriaFinanciera::try_from(row.get::<_, i64>(3)?)
                        .unwrap_or(CategoriaFinanciera::Otros),
                    descripcion: row.get(4)?,
                    monto: Cents(row.get(5)?),
                    item_especifico: row.get(6)?,
                    partido_id: row.get(7)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Obtiene una transacción.
pub fn get(conn: &Connection, id: i64) -> Result<TransaccionFinanciera, DbError> {
    conn.query_row(
        "SELECT id, fecha, tipo, categoria, descripcion, monto_centavos, item_especifico, partido_id
           FROM financiamiento WHERE id = ?1",
        [id],
        |row| {
            let fecha_texto: String = row.get(1)?;
            let fecha = dates::parse_date(&fecha_texto).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(
                    1,
                    "fecha".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;
            Ok(TransaccionFinanciera {
                id: row.get(0)?,
                fecha,
                tipo: TipoTransaccion::try_from(row.get::<_, i64>(2)?)
                    .unwrap_or(TipoTransaccion::Gasto),
                categoria: CategoriaFinanciera::try_from(row.get::<_, i64>(3)?)
                    .unwrap_or(CategoriaFinanciera::Otros),
                descripcion: row.get(4)?,
                monto: Cents(row.get(5)?),
                item_especifico: row.get(6)?,
                partido_id: row.get(7)?,
            })
        },
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
            entidad: "Financiamiento",
            id,
        },
        other => other.into(),
    })
}

/// Inserta una transacción.
pub fn insert(conn: &Connection, transaccion: &TransaccionFinanciera) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO financiamiento
            (fecha, tipo, categoria, descripcion, monto_centavos, item_especifico, partido_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            dates::to_iso_date(transaccion.fecha),
            transaccion.tipo.as_i64(),
            transaccion.categoria.as_i64(),
            transaccion.descripcion,
            transaccion.monto.0,
            transaccion.item_especifico,
            transaccion.partido_id,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza una transacción.
pub fn update(conn: &Connection, transaccion: &TransaccionFinanciera) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE financiamiento
            SET fecha = ?1, tipo = ?2, categoria = ?3, descripcion = ?4,
                monto_centavos = ?5, item_especifico = ?6, partido_id = ?7
          WHERE id = ?8",
        params![
            dates::to_iso_date(transaccion.fecha),
            transaccion.tipo.as_i64(),
            transaccion.categoria.as_i64(),
            transaccion.descripcion,
            transaccion.monto.0,
            transaccion.item_especifico,
            transaccion.partido_id,
            transaccion.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Financiamiento",
            id: transaccion.id,
        });
    }
    Ok(())
}

/// Elimina una transacción.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM financiamiento WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Financiamiento",
            id,
        });
    }
    Ok(())
}

/// Resumen del filtro.
pub fn resumen(
    conn: &Connection,
    filter: &FinanciamientoFilter,
) -> Result<FinanzasResumen, DbError> {
    let sql = format!(
        "SELECT COALESCE(SUM(CASE WHEN f.tipo = 0 THEN f.monto_centavos ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN f.tipo = 1 THEN f.monto_centavos ELSE 0 END), 0)
           FROM financiamiento f
           {FILTRO_SQL}"
    );
    let desde = filter.desde.map(dates::to_iso_date).unwrap_or_default();
    let hasta = filter.hasta.map(dates::to_iso_date).unwrap_or_default();
    let (ingresos, gastos): (i64, i64) = conn.query_row(
        &sql,
        params![
            filter.tipo.map_or(-1, TipoTransaccion::as_i64),
            filter.categoria.map_or(-1, CategoriaFinanciera::as_i64),
            desde,
            hasta,
            filter.texto.trim(),
        ],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(FinanzasResumen {
        ingresos: Cents(ingresos),
        gastos: Cents(gastos),
        balance: Cents(ingresos - gastos),
    })
}

/// Totales de gasto por categoría.
pub fn por_categoria(conn: &Connection) -> Result<Vec<CategoriaTotal>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT categoria, COUNT(*), COALESCE(SUM(monto_centavos), 0)
           FROM financiamiento
          WHERE tipo = 1
          GROUP BY categoria
          ORDER BY SUM(monto_centavos) DESC, categoria ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CategoriaTotal {
                categoria: CategoriaFinanciera::try_from(row.get::<_, i64>(0)?)
                    .unwrap_or(CategoriaFinanciera::Otros),
                cantidad: row.get(1)?,
                total: Cents(row.get(2)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Presupuesto de un mes con gasto real y alerta.
pub fn presupuesto_vista(
    conn: &Connection,
    mes_anio: &str,
) -> Result<Option<PresupuestoVista>, DbError> {
    let presupuesto: Option<PresupuestoMensual> = conn
        .query_row(
            "SELECT id, mes_anio, presupuesto_total_centavos, limite_gasto_centavos,
                    alertas_habilitadas, fecha_creacion, fecha_modificacion
               FROM presupuesto_mensual WHERE mes_anio = ?1",
            [mes_anio],
            |row| {
                let creacion: String = row.get(5)?;
                let modificacion: String = row.get(6)?;
                Ok(PresupuestoMensual {
                    id: row.get(0)?,
                    mes_anio: row.get(1)?,
                    presupuesto_total: Cents(row.get(2)?),
                    limite_gasto: Cents(row.get(3)?),
                    alertas_habilitadas: row.get::<_, i64>(4)? != 0,
                    fecha_creacion: dates::parse_date(&creacion).unwrap_or_else(dates::today),
                    fecha_modificacion: dates::parse_date(&modificacion)
                        .unwrap_or_else(dates::today),
                })
            },
        )
        .optional()?;

    let Some(presupuesto) = presupuesto else {
        return Ok(None);
    };
    let gasto_real = gasto_mes(conn, mes_anio)?;
    let alerta =
        if presupuesto.alertas_habilitadas && gasto_real.0 > presupuesto.presupuesto_total.0 {
            AlertaPresupuesto::PresupuestoSuperado
        } else if presupuesto.alertas_habilitadas && gasto_real.0 > presupuesto.limite_gasto.0 {
            AlertaPresupuesto::LimiteSuperado
        } else {
            AlertaPresupuesto::Ok
        };
    Ok(Some(PresupuestoVista {
        presupuesto,
        gasto_real,
        alerta,
    }))
}

/// Gasto de un mes `YYYY-MM`.
pub fn gasto_mes(conn: &Connection, mes_anio: &str) -> Result<Cents, DbError> {
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(monto_centavos), 0)
           FROM financiamiento
          WHERE tipo = 1 AND substr(fecha, 1, 7) = ?1",
        [mes_anio],
        |row| row.get(0),
    )?;
    Ok(Cents(total))
}

/// Inserta o actualiza el presupuesto del mes.
pub fn upsert_presupuesto(
    conn: &Connection,
    presupuesto: &PresupuestoMensual,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO presupuesto_mensual
            (mes_anio, presupuesto_total_centavos, limite_gasto_centavos,
             alertas_habilitadas, fecha_creacion, fecha_modificacion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(mes_anio) DO UPDATE SET
            presupuesto_total_centavos = excluded.presupuesto_total_centavos,
            limite_gasto_centavos = excluded.limite_gasto_centavos,
            alertas_habilitadas = excluded.alertas_habilitadas,
            fecha_modificacion = excluded.fecha_modificacion",
        params![
            presupuesto.mes_anio,
            presupuesto.presupuesto_total.0,
            presupuesto.limite_gasto.0,
            i64::from(presupuesto.alertas_habilitadas),
            dates::to_iso_date(presupuesto.fecha_modificacion),
        ],
    )?;
    Ok(())
}

/// `true` si ya existe una transacción automática para ese partido.
pub fn existe_auto_partido(
    conn: &Connection,
    partido_id: i64,
    exclude_id: Option<i64>,
) -> Result<bool, DbError> {
    let exclude = exclude_id.unwrap_or(0);
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM financiamiento
             WHERE partido_id = ?1
               AND descripcion = 'Pago por alquiler de cancha'
               AND (?2 = 0 OR id <> ?2)
        )",
        params![partido_id, exclude],
        |row| row.get(0),
    )?;
    Ok(exists)
}
