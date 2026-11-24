//! Reportes de Estadísticas que faltaban respecto de MiFutbolC: tabla de
//! posiciones global (`posiciones.c`), filtros avanzados (`filtros.c`),
//! reportes automáticos (`reportes.c`) y las opciones de contexto de
//! `estadisticas*.c` (goles/asistencias por ánimo, clima y momento, cansancio).

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db::DbError;
use crate::db::reportes_repo::{self, FiltroPartidos, Totales};
use crate::domain::models::{Clima, FranjaHoraria};
use crate::domain::reporte::Reporte;

/// Nivel 1-3 / 4-7 / 8-10 de `get_nivel_case_sql` de C.
fn nivel(columna: &str) -> String {
    format!(
        "CASE WHEN {columna} <= 3 THEN 'Bajo (1-3)' WHEN {columna} <= 7 THEN 'Medio (4-7)' \
         ELSE 'Alto (8-10)' END"
    )
}

fn etiqueta_clima(codigo: &str) -> String {
    codigo
        .parse::<i64>()
        .ok()
        .and_then(|c| Clima::try_from(c).ok())
        .map_or_else(|| codigo.to_string(), |c| c.label().to_string())
}

fn etiqueta_franja(codigo: &str) -> String {
    codigo
        .parse::<i64>()
        .ok()
        .and_then(|c| FranjaHoraria::try_from(c).ok())
        .map_or_else(|| codigo.to_string(), |f| f.label().to_string())
}

fn resultado(codigo: i64) -> &'static str {
    match codigo {
        1 => "Victoria",
        2 => "Empate",
        3 => "Derrota",
        _ => "—",
    }
}

fn fila_totales(etiqueta: &str, t: &Totales) -> String {
    let victorias = if t.pj > 0 {
        100.0 * t.pg as f64 / t.pj as f64
    } else {
        0.0
    };
    format!(
        "{etiqueta:<6} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>+5} {:>4} {victorias:>5.0}%",
        t.pj,
        t.pg,
        t.pe,
        t.pp,
        t.gf,
        t.gc,
        t.gf - t.gc,
        t.puntos()
    )
}

/// Tabla de posiciones global con desglose por año (`mostrar_tabla_posiciones_global`).
pub fn tabla_posiciones(conn: &Connection) -> Result<Reporte, DbError> {
    let anios = reportes_repo::posiciones_por_anio(conn)?;
    let mut r = Reporte::new("Tabla de posiciones global");
    if anios.is_empty() {
        r.linea("No hay partidos con resultado registrados.");
        return Ok(r);
    }
    let mut total = Totales::default();
    for (_, t) in &anios {
        total.pj += t.pj;
        total.pg += t.pg;
        total.pe += t.pe;
        total.pp += t.pp;
        total.gf += t.gf;
        total.gc += t.gc;
    }
    r.seccion("Totales");
    r.linea(format!(
        "Partidos {}  ·  Puntos {}",
        total.pj,
        total.puntos()
    ));
    r.linea(format!(
        "Promedio goles a favor {:.2}  ·  en contra {:.2}",
        total.gf as f64 / total.pj as f64,
        total.gc as f64 / total.pj as f64
    ));
    r.seccion("Desglose por año");
    r.linea("Año      PJ   PG   PE   PP   GF   GC    DG  Pts    V%");
    for (anio, t) in &anios {
        r.linea(fila_totales(anio, t));
    }
    r.linea(fila_totales("Total", &total));
    Ok(r)
}

fn seccion_agrupada(
    r: &mut Reporte,
    conn: &Connection,
    titulo: &str,
    clave_sql: &str,
    metrica: &str,
    etiqueta: fn(&str) -> String,
) -> Result<(), DbError> {
    r.seccion(titulo);
    let filas = reportes_repo::agrupado(conn, clave_sql, metrica)?;
    if filas.is_empty() {
        r.linea("  Sin datos");
    }
    for (clave, partidos, suma, promedio) in filas {
        r.linea(format!(
            "  {:<18} total {suma:>4} · promedio {promedio:>5.2} · {partidos} partidos",
            etiqueta(&clave)
        ));
    }
    Ok(())
}

/// Opciones de los submenús de Goles, Asistencias, Partidos y Rendimiento de C
/// que no tenían pantalla en MiFutbolR.
pub fn contexto(conn: &Connection) -> Result<Reporte, DbError> {
    let mut r = Reporte::new("Contexto: ánimo, clima y cansancio");
    let animo = nivel("estado_animo");
    let cansancio = nivel("cansancio");
    let texto: fn(&str) -> String = str::to_string;
    seccion_agrupada(
        &mut r,
        conn,
        "Goles por estado de ánimo",
        &animo,
        "goles",
        texto,
    )?;
    seccion_agrupada(
        &mut r,
        conn,
        "Asistencias por estado de ánimo",
        &animo,
        "asistencias",
        texto,
    )?;
    seccion_agrupada(
        &mut r,
        conn,
        "Asistencias por clima",
        "clima",
        "asistencias",
        etiqueta_clima,
    )?;
    seccion_agrupada(
        &mut r,
        conn,
        "Asistencias por momento del día",
        "dia",
        "asistencias",
        etiqueta_franja,
    )?;
    seccion_agrupada(
        &mut r,
        conn,
        "Asistencias por nivel de cansancio",
        &cansancio,
        "asistencias",
        texto,
    )?;
    seccion_agrupada(
        &mut r,
        conn,
        "Rendimiento por estado de ánimo",
        &animo,
        "rendimiento_general",
        texto,
    )?;

    r.seccion("Lo mejor y lo peor");
    let clima = reportes_repo::agrupado(conn, "clima", "rendimiento_general")?;
    let dias = reportes_repo::agrupado(
        conn,
        "CAST(strftime('%w', fecha_hora) AS INTEGER)",
        "rendimiento_general",
    )?;
    let ideal = reportes_repo::agrupado(conn, &animo, "rendimiento_general")?;
    let dia_semana = |c: &str| {
        [
            "Domingo",
            "Lunes",
            "Martes",
            "Miércoles",
            "Jueves",
            "Viernes",
            "Sábado",
        ]
        .get(c.parse::<usize>().unwrap_or(9))
        .map_or_else(|| c.to_string(), |d| (*d).to_string())
    };
    for (titulo, filas, etiqueta) in [
        (
            "Clima donde rendís mejor",
            &clima,
            etiqueta_clima as fn(&str) -> String,
        ),
        ("Mejor día de la semana", &dias, dia_semana),
        ("Estado de ánimo ideal", &ideal, texto),
    ] {
        if let Some((clave, _, _, promedio)) = filas.first() {
            r.linea(format!("  {titulo}: {} ({promedio:.2})", etiqueta(clave)));
        }
    }
    for (titulo, filas, etiqueta) in [
        (
            "Clima donde rendís peor",
            &clima,
            etiqueta_clima as fn(&str) -> String,
        ),
        ("Peor día de la semana", &dias, dia_semana),
    ] {
        if let Some((clave, _, _, promedio)) = filas.last() {
            r.linea(format!("  {titulo}: {} ({promedio:.2})", etiqueta(clave)));
        }
    }

    r.seccion("Cansancio alto (> 7)");
    let cansados = reportes_repo::filtrar(
        conn,
        &FiltroPartidos {
            cansancio_alto: true,
            ..FiltroPartidos::default()
        },
    )?;
    r.linea(format!("  Partidos con cansancio alto: {}", cansados.len()));
    let promedio =
        |v: Option<f64>| v.map_or_else(|| "sin datos".to_string(), |v| format!("{v:.2}"));
    r.linea(format!(
        "  Caída por cansancio acumulado: últimos 5 {} · primeros 5 {}",
        promedio(reportes_repo::rendimiento_cansado(conn, true)?),
        promedio(reportes_repo::rendimiento_cansado(conn, false)?)
    ));
    for fila in cansados.iter().take(10) {
        r.linea(format!(
            "  #{} {} · G {} A {} · Rend {}",
            fila.id,
            fila.fecha_hora.get(..16).unwrap_or(&fila.fecha_hora),
            fila.goles,
            fila.asistencias,
            fila.rendimiento
        ));
    }
    Ok(r)
}

/// Resultado de "Búsqueda por filtros" con el resumen de "Estadísticas filtradas".
pub fn filtrar(conn: &Connection, filtro: &FiltroPartidos) -> Result<Reporte, DbError> {
    let filas = reportes_repo::filtrar(conn, filtro)?;
    let mut r = Reporte::new("Filtros avanzados");
    let fecha = |d: Option<chrono::NaiveDate>| {
        d.map_or_else(|| "(no)".to_string(), |d| d.format("%d/%m/%Y").to_string())
    };
    r.linea(format!(
        "Desde {} · hasta {} · resultado {}",
        fecha(filtro.desde),
        fecha(filtro.hasta),
        filtro.resultado.map_or("(todos)", resultado)
    ));
    r.seccion("Partidos encontrados");
    if filas.is_empty() {
        r.linea("No hay partidos con esos filtros.");
        return Ok(r);
    }
    for f in &filas {
        r.linea(format!(
            "#{} {} · G {} A {} · {} · Rend {} · {}",
            f.id,
            f.fecha_hora.get(..16).unwrap_or(&f.fecha_hora),
            f.goles,
            f.asistencias,
            resultado(f.resultado),
            f.rendimiento,
            f.cancha
        ));
    }
    let total = filas.len() as i64;
    let suma = |campo: fn(&reportes_repo::FilaPartido) -> i64| filas.iter().map(campo).sum::<i64>();
    let cuenta = |codigo| filas.iter().filter(|f| f.resultado == codigo).count();
    r.seccion("Resumen");
    r.linea(format!(
        "Total {total} partidos · goles {} · asistencias {} · rendimiento promedio {:.2}",
        suma(|f| f.goles),
        suma(|f| f.asistencias),
        suma(|f| f.rendimiento) as f64 / total as f64
    ));
    r.linea(format!(
        "Victorias {} · empates {} · derrotas {}",
        cuenta(1),
        cuenta(2),
        cuenta(3)
    ));
    Ok(r)
}

/// Reportes que se pueden generar (`menu_generar_reportes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoReporte {
    /// "Reporte General HTML".
    GeneralHtml,
    /// "Reporte de Rendimiento TXT".
    RendimientoTxt,
}

impl TipoReporte {
    /// Nombre que se guarda en `reporte_generado`.
    #[must_use]
    pub const fn nombre(self) -> &'static str {
        match self {
            Self::GeneralHtml => "Reporte General HTML",
            Self::RendimientoTxt => "Reporte Rendimiento TXT",
        }
    }
}

/// Contenido del reporte general (`generar_reporte_general_html`).
pub fn reporte_general(conn: &Connection) -> Result<Reporte, DbError> {
    let mut r = Reporte::new("Reporte general MiFutbolR");
    r.linea(format!(
        "Generado: {}",
        chrono::Local::now().format("%d/%m/%Y %H:%M")
    ));
    r.seccion("Partidos");
    r.linea(format!(
        "Total: {}",
        reportes_repo::contar(conn, "SELECT COUNT(*) FROM partido")?
    ));
    r.seccion("Resultados");
    for codigo in 1..=3 {
        let cantidad = reportes_repo::contar(
            conn,
            &format!("SELECT COUNT(*) FROM partido WHERE resultado = {codigo}"),
        )?;
        r.linea(format!("{}: {cantidad}", resultado(codigo)));
    }
    r.seccion("Goles y asistencias");
    r.linea(format!(
        "Goles: {} · Asistencias: {}",
        reportes_repo::contar(conn, "SELECT COALESCE(SUM(goles), 0) FROM partido")?,
        reportes_repo::contar(conn, "SELECT COALESCE(SUM(asistencias), 0) FROM partido")?
    ));
    r.seccion("Torneos");
    r.linea(format!(
        "Total: {}",
        reportes_repo::contar(conn, "SELECT COUNT(*) FROM torneo")?
    ));
    // C comparaba fechas `DD/MM/YYYY` con `date('now')`; aquí son ISO y en hora local.
    r.seccion("Lesiones (últimos 30 días)");
    r.linea(format!(
        "Total: {}",
        reportes_repo::contar(
            conn,
            "SELECT COUNT(*) FROM lesion WHERE date(fecha) >= date('now', 'localtime', '-30 days')"
        )?
    ));
    Ok(r)
}

/// Contenido del reporte de rendimiento (`generar_reporte_rendimiento_txt`).
pub fn reporte_rendimiento(conn: &Connection) -> Result<Reporte, DbError> {
    let mut r = Reporte::new("Reporte de rendimiento");
    r.linea(chrono::Local::now().format("%d/%m/%Y %H:%M").to_string());
    r.seccion("Rendimiento por mes (últimos 12)");
    r.linea("Mes          PJ    V    E    D");
    let meses = reportes_repo::agrupado(conn, "substr(fecha_hora, 1, 7)", "1")?;
    let mut meses: Vec<String> = meses.into_iter().map(|(mes, ..)| mes).collect();
    meses.sort_by(|a, b| b.cmp(a));
    for mes in meses.into_iter().take(12) {
        let cuenta = |extra: &str| {
            reportes_repo::contar(
                conn,
                &format!(
                    "SELECT COUNT(*) FROM partido WHERE resultado > 0 AND substr(fecha_hora, 1, 7) = '{mes}'{extra}"
                ),
            )
        };
        r.linea(format!(
            "{mes:<10} {:>4} {:>4} {:>4} {:>4}",
            cuenta("")?,
            cuenta(" AND resultado = 1")?,
            cuenta(" AND resultado = 2")?,
            cuenta(" AND resultado = 3")?
        ));
    }
    Ok(r)
}

/// Escribe el reporte en `dir` y lo registra en `reporte_generado`.
pub fn generar(conn: &Connection, dir: &Path, tipo: TipoReporte) -> Result<PathBuf, String> {
    let sello = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let (archivo, contenido) = match tipo {
        TipoReporte::GeneralHtml => (
            format!("reporte_general_{sello}.html"),
            reporte_general(conn).map_err(|e| e.to_string())?.html(),
        ),
        TipoReporte::RendimientoTxt => (
            format!("rendimiento_{sello}.txt"),
            reporte_rendimiento(conn)
                .map_err(|e| e.to_string())?
                .texto(),
        ),
    };
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let ruta = dir.join(&archivo);
    std::fs::write(&ruta, contenido).map_err(|e| e.to_string())?;
    reportes_repo::registrar_generado(conn, tipo.nombre(), &archivo).map_err(|e| e.to_string())?;
    Ok(ruta)
}

/// "Reportes generados" (`listar_reportes_generados`).
pub fn generados(conn: &Connection, dir: &Path) -> Result<Reporte, DbError> {
    let mut r = Reporte::new("Reportes generados");
    let filas = reportes_repo::generados(conn)?;
    if filas.is_empty() {
        r.linea("Todavía no se generó ningún reporte.");
    }
    for (nombre, archivo, fecha) in filas {
        r.linea(format!("{fecha} · {nombre}"));
        r.linea(format!("   {}", dir.join(archivo).display()));
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn preparar() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (nombre) VALUES ('Sur'), ('Norte');
                 INSERT INTO camiseta (nombre) VALUES ('Titular');
                 INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, goles, asistencias,
                    resultado, rendimiento_general, cansancio, estado_animo, clima, dia,
                    goles_equipo, goles_rival, goles_en_contra) VALUES
                 (1, 1, '2025-06-01 20:00:00', 2, 1, 1, 8, 9, 9, 1, 2, 5, 3, 0),
                 (1, 1, '2026-01-10 20:00:00', 0, 2, 3, 4, 2, 2, 2, 2, -1, -1, 4),
                 (2, 1, '2026-01-31 21:00:00', 1, 0, 2, 6, 8, 5, 1, 3, 2, 2, 0);",
            )
            .unwrap();
        db
    }

    #[test]
    fn tabla_de_posiciones_por_anio_con_marcador_del_equipo() {
        let db = preparar();
        let anios = reportes_repo::posiciones_por_anio(db.conn()).unwrap();
        assert_eq!(anios.len(), 2);
        assert_eq!(anios[0].0, "2026");
        // 2026: derrota sin marcador (GF = goles 0, GC = en contra 4) + empate 2-2.
        assert_eq!(
            anios[0].1,
            Totales {
                pj: 2,
                pg: 0,
                pe: 1,
                pp: 1,
                gf: 2,
                gc: 6
            }
        );
        assert_eq!(anios[1].1.gf, 5);
        let r = tabla_posiciones(db.conn()).unwrap();
        assert!(
            r.lineas
                .iter()
                .any(|l| l.starts_with("Total") && l.contains("   3    1    1    1    7    9")),
            "{:?}",
            r.lineas
        );
        assert!(r.lineas.iter().any(|l| l.contains("Puntos 4")));
    }

    #[test]
    fn filtros_incluyen_el_dia_hasta_y_resumen() {
        let db = preparar();
        let hasta = chrono::NaiveDate::from_ymd_opt(2026, 1, 31);
        let filtro = FiltroPartidos {
            desde: chrono::NaiveDate::from_ymd_opt(2026, 1, 1),
            hasta,
            ..FiltroPartidos::default()
        };
        // El partido del 31/01 a las 21:00 entra (C lo dejaba afuera).
        assert_eq!(reportes_repo::filtrar(db.conn(), &filtro).unwrap().len(), 2);
        let solo_derrotas = FiltroPartidos {
            resultado: Some(3),
            cancha_id: Some(1),
            ..filtro
        };
        let r = filtrar(db.conn(), &solo_derrotas).unwrap();
        assert!(
            r.lineas.iter().any(|l| l.starts_with("Total 1 partidos")),
            "{:?}",
            r.lineas
        );
        let vacio = FiltroPartidos {
            cancha_id: Some(99),
            ..filtro
        };
        assert!(
            filtrar(db.conn(), &vacio)
                .unwrap()
                .lineas
                .contains(&"No hay partidos con esos filtros.".to_string())
        );
    }

    #[test]
    fn contexto_agrupa_por_nivel_y_cansancio() {
        let db = preparar();
        let r = contexto(db.conn()).unwrap();
        let texto = r.texto();
        assert!(texto.contains("Alto (8-10)"), "{texto}");
        assert!(texto.contains("Partidos con cansancio alto: 2"), "{texto}");
        assert!(
            texto.contains("Estado de ánimo ideal: Alto (8-10) (8.00)"),
            "{texto}"
        );
        assert!(
            texto.contains("últimos 5 7.00 · primeros 5 7.00"),
            "{texto}"
        );
    }

    #[test]
    fn generar_registra_y_lista_los_reportes() {
        let db = preparar();
        let dir = tempfile::tempdir().unwrap();
        let html = generar(db.conn(), dir.path(), TipoReporte::GeneralHtml).unwrap();
        let contenido = std::fs::read_to_string(&html).unwrap();
        assert!(contenido.contains("<h2>Resultados</h2>"), "{contenido}");
        assert!(contenido.contains("Victoria: 1"));
        let txt = generar(db.conn(), dir.path(), TipoReporte::RendimientoTxt).unwrap();
        assert!(std::fs::read_to_string(&txt).unwrap().contains("2026-01"));
        let lista = generados(db.conn(), dir.path()).unwrap().lineas;
        assert_eq!(lista.len(), 4);
        assert!(lista[0].ends_with("Reporte Rendimiento TXT"));
    }
}
