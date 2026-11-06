//! Reglas de negocio del módulo Financiamiento (§15.5).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::financiamiento_repo;
use crate::domain::Cents;
use crate::domain::dates;
use crate::domain::models::{
    CategoriaFinanciera, CategoriaTotal, FinanciamientoFilter, FinanzasResumen, PresupuestoMensual,
    PresupuestoVista, TransaccionFinanciera,
};
use crate::domain::validation::{self, DomainError};

/// Error de negocio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Validación.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Id inexistente.
    #[error("No existe la transacción id={0}")]
    NotFound(i64),
}

/// Valida una transacción.
pub fn validar(transaccion: &TransaccionFinanciera) -> Result<(), DomainError> {
    validation::required(&transaccion.descripcion, "descripción")?;
    validation::max_len(&transaccion.descripcion, "descripción", 200)?;
    validation::max_len(&transaccion.item_especifico, "ítem específico", 120)?;
    validation::non_negative(transaccion.monto.0, "monto")?;
    let year = chrono::Datelike::year(&transaccion.fecha);
    validation::range_i64(i64::from(year), 1900, 2100, "fecha")?;
    Ok(())
}

/// Valida una clave `YYYY-MM`.
pub fn validar_mes(mes: &str) -> Result<(), DomainError> {
    let bytes = mes.as_bytes();
    let valido = bytes.len() == 7
        && bytes[4] == b'-'
        && mes[..4].chars().all(|ch| ch.is_ascii_digit())
        && mes[5..].chars().all(|ch| ch.is_ascii_digit())
        && mes[5..]
            .parse::<u32>()
            .is_ok_and(|month| (1..=12).contains(&month));
    if valido {
        Ok(())
    } else {
        Err(DomainError::Message(format!(
            "Mes inválido: «{mes}» (use YYYY-MM)"
        )))
    }
}

/// Crea una transacción.
pub fn crear(
    conn: &Connection,
    transaccion: &mut TransaccionFinanciera,
) -> Result<i64, ServiceError> {
    validar(transaccion)?;
    if let Some(partido_id) = transaccion.partido_id
        && transaccion.categoria == CategoriaFinanciera::Canchas
        && financiamiento_repo::existe_auto_partido(conn, partido_id, None)?
    {
        return Err(DomainError::Message(
            "Ese partido ya tiene su transacción automática de cancha".to_string(),
        )
        .into());
    }
    let id = financiamiento_repo::insert(conn, transaccion)?;
    transaccion.id = id;
    Ok(id)
}

/// Actualiza una transacción.
pub fn actualizar(
    conn: &Connection,
    transaccion: &TransaccionFinanciera,
) -> Result<(), ServiceError> {
    if transaccion.id <= 0 {
        return Err(ServiceError::NotFound(transaccion.id));
    }
    validar(transaccion)?;
    financiamiento_repo::get(conn, transaccion.id)?;
    if let Some(partido_id) = transaccion.partido_id
        && transaccion.categoria == CategoriaFinanciera::Canchas
        && financiamiento_repo::existe_auto_partido(conn, partido_id, Some(transaccion.id))?
    {
        return Err(DomainError::Message(
            "Ese partido ya tiene su transacción automática de cancha".to_string(),
        )
        .into());
    }
    financiamiento_repo::update(conn, transaccion)?;
    Ok(())
}

/// Elimina una transacción (no toca el partido).
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    financiamiento_repo::delete(conn, id)?;
    Ok(())
}

/// Lista transacciones.
pub fn listar(
    conn: &Connection,
    filter: &FinanciamientoFilter,
) -> Result<Vec<TransaccionFinanciera>, ServiceError> {
    Ok(financiamiento_repo::list(conn, filter)?)
}

/// Resumen del filtro.
pub fn resumen(
    conn: &Connection,
    filter: &FinanciamientoFilter,
) -> Result<FinanzasResumen, ServiceError> {
    Ok(financiamiento_repo::resumen(conn, filter)?)
}

/// Totales por categoría.
pub fn por_categoria(conn: &Connection) -> Result<Vec<CategoriaTotal>, ServiceError> {
    Ok(financiamiento_repo::por_categoria(conn)?)
}

/// Obtiene una transacción.
pub fn obtener(conn: &Connection, id: i64) -> Result<TransaccionFinanciera, ServiceError> {
    Ok(financiamiento_repo::get(conn, id)?)
}

/// Presupuesto de un mes con alerta.
pub fn presupuesto(
    conn: &Connection,
    mes_anio: &str,
) -> Result<Option<PresupuestoVista>, ServiceError> {
    validar_mes(mes_anio)?;
    Ok(financiamiento_repo::presupuesto_vista(conn, mes_anio)?)
}

/// Guarda el presupuesto del mes.
pub fn guardar_presupuesto(
    conn: &Connection,
    presupuesto: &PresupuestoMensual,
) -> Result<(), ServiceError> {
    validar_mes(&presupuesto.mes_anio)?;
    validation::non_negative(presupuesto.presupuesto_total.0, "presupuesto total")?;
    validation::non_negative(presupuesto.limite_gasto.0, "límite de gasto")?;
    financiamiento_repo::upsert_presupuesto(conn, presupuesto)?;
    Ok(())
}

/// Mes actual `YYYY-MM`.
#[must_use]
pub fn mes_actual() -> String {
    dates::month_key(dates::now_local())
}

/// Gasto del mes.
pub fn gasto_mes(conn: &Connection, mes_anio: &str) -> Result<Cents, ServiceError> {
    Ok(financiamiento_repo::gasto_mes(conn, mes_anio)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::models::{AlertaPresupuesto, TipoTransaccion};

    fn base(monto: i64) -> TransaccionFinanciera {
        TransaccionFinanciera {
            id: 0,
            fecha: chrono::NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            tipo: TipoTransaccion::Gasto,
            categoria: CategoriaFinanciera::Canchas,
            descripcion: "Alquiler".into(),
            monto: Cents(monto),
            item_especifico: String::new(),
            partido_id: None,
        }
    }

    #[test]
    fn validates_and_lists() {
        let db = Database::open_in_memory().unwrap();
        let mut gasto = base(15_000);
        crear(db.conn(), &mut gasto).unwrap();
        let mut ingreso = base(20_000);
        ingreso.tipo = TipoTransaccion::Ingreso;
        ingreso.categoria = CategoriaFinanciera::Otros;
        crear(db.conn(), &mut ingreso).unwrap();

        let rows = listar(db.conn(), &FinanciamientoFilter::default()).unwrap();
        assert_eq!(rows.len(), 2);

        let resumen = resumen(db.conn(), &FinanciamientoFilter::default()).unwrap();
        assert_eq!(resumen.ingresos, Cents(20_000));
        assert_eq!(resumen.gastos, Cents(15_000));
        assert_eq!(resumen.balance, Cents(5_000));

        let filtro = FinanciamientoFilter {
            tipo: Some(TipoTransaccion::Gasto),
            ..FinanciamientoFilter::default()
        };
        assert_eq!(listar(db.conn(), &filtro).unwrap().len(), 1);

        let por_categoria = por_categoria(db.conn()).unwrap();
        assert_eq!(por_categoria.len(), 1);
        assert_eq!(por_categoria[0].total, Cents(15_000));
    }

    #[test]
    fn rejects_invalid_amount_and_month() {
        let db = Database::open_in_memory().unwrap();
        let mut negativo = base(-1);
        assert!(crear(db.conn(), &mut negativo).is_err());
        assert!(validar_mes("2026-13").is_err());
        assert!(validar_mes("2026-5").is_err());
        assert!(validar_mes("2026-05").is_ok());
    }

    #[test]
    fn budget_alerts() {
        let db = Database::open_in_memory().unwrap();
        let mut gasto = base(30_000);
        gasto.fecha = chrono::NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        crear(db.conn(), &mut gasto).unwrap();

        let config = PresupuestoMensual {
            id: 0,
            mes_anio: "2026-05".into(),
            presupuesto_total: Cents(100_000),
            limite_gasto: Cents(50_000),
            alertas_habilitadas: true,
            fecha_creacion: chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            fecha_modificacion: chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        };
        guardar_presupuesto(db.conn(), &config).unwrap();
        let vista = presupuesto(db.conn(), "2026-05").unwrap().unwrap();
        assert_eq!(vista.gasto_real, Cents(30_000));
        assert_eq!(vista.alerta, AlertaPresupuesto::Ok);

        let mut otro = config.clone();
        otro.limite_gasto = Cents(20_000);
        guardar_presupuesto(db.conn(), &otro).unwrap();
        assert_eq!(
            presupuesto(db.conn(), "2026-05").unwrap().unwrap().alerta,
            AlertaPresupuesto::LimiteSuperado
        );

        let mut fuerte = config.clone();
        fuerte.presupuesto_total = Cents(10_000);
        fuerte.limite_gasto = Cents(50_000);
        guardar_presupuesto(db.conn(), &fuerte).unwrap();
        assert_eq!(
            presupuesto(db.conn(), "2026-05").unwrap().unwrap().alerta,
            AlertaPresupuesto::PresupuestoSuperado
        );

        // Desactivar alertas las silencia.
        let mut sin_alertas = config.clone();
        sin_alertas.alertas_habilitadas = false;
        guardar_presupuesto(db.conn(), &sin_alertas).unwrap();
        assert_eq!(
            presupuesto(db.conn(), "2026-05").unwrap().unwrap().alerta,
            AlertaPresupuesto::Ok
        );
    }

    #[test]
    fn rejects_duplicate_automatic_transaction() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('Sur')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora)
                 VALUES (1, 1, '2026-05-10 20:00:00')",
                [],
            )
            .unwrap();
        let partido_id = db.conn().last_insert_rowid();
        db.conn()
            .execute(
                "INSERT INTO financiamiento
                    (fecha, tipo, categoria, descripcion, monto_centavos, partido_id)
                 VALUES ('2026-05-10', 1, 5, 'Pago por alquiler de cancha', 15000, ?1)",
                [partido_id],
            )
            .unwrap();

        let mut duplicada = base(15_000);
        duplicada.partido_id = Some(partido_id);
        assert!(crear(db.conn(), &mut duplicada).is_err());
    }

    #[test]
    fn deleting_keeps_match() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('Sur')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora)
                 VALUES (1, 1, '2026-05-10 20:00:00')",
                [],
            )
            .unwrap();
        let partido_id = db.conn().last_insert_rowid();
        let mut transaccion = base(10_000);
        transaccion.partido_id = Some(partido_id);
        transaccion.descripcion = "Manual".into();
        let id = crear(db.conn(), &mut transaccion).unwrap();
        eliminar(db.conn(), id).unwrap();
        let partidos: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0))
            .unwrap();
        assert_eq!(partidos, 1);
    }
}
