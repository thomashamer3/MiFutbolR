//! Entrenador IA: reglas de MiFutbolC (`entrenador_ia.c`) como funciones puras.
//!
//! Correcciones respecto de MiFutbolC: la derrota es `resultado = 3` (allá se
//! contaba el 0, "sin cargar") y las fechas se comparan con `fecha_hora` ISO.
//! El aviso previo a crear un partido es [`alerta_antes_partido`].

use jiff::civil::Date;
use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::entrenador_repo;
use crate::domain::dates;
use crate::domain::entrenador::{
    Agregados, Analisis, Consejo, EstadoJugador, PartidoIa, Severidad,
};

/// Error del servicio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Validación.
    #[error("{0}")]
    Message(String),
}

/// Estado del jugador a partir de los últimos partidos (más recientes primero).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn evaluar(partidos: &[PartidoIa], hoy: Date) -> EstadoJugador {
    let muestra = &partidos[..partidos.len().min(10)];
    if muestra.is_empty() {
        return EstadoJugador::default();
    }
    let n = muestra.len() as f64;
    let promedio = |f: fn(&PartidoIa) -> i64| muestra.iter().map(f).sum::<i64>() as f64 / n;
    let dias = |p: &PartidoIa| {
        dates::parse_datetime(&p.fecha_hora)
            .map_or(0, |fecha| dates::dias_entre(fecha.date(), hoy).max(0))
    };
    let dias_descanso = dias(&muestra[0]);
    let consecutivos = muestra.iter().take(7).filter(|p| dias(p) <= 7).count() as i64;
    let derrotas = muestra
        .iter()
        .take(5)
        .take_while(|p| p.resultado == 3)
        .count() as i64;
    let d = dias_descanso as f64;
    let factor = if d >= 14.0 {
        0.0
    } else if d >= 7.0 {
        0.5
    } else if d > 0.0 {
        1.0 - d / 14.0
    } else {
        1.0
    };
    let cansancio = promedio(|p| p.cansancio);
    EstadoJugador {
        partidos: muestra.len(),
        rendimiento: promedio(|p| p.rendimiento),
        cansancio,
        animo: promedio(|p| p.animo),
        dias_descanso,
        consecutivos,
        derrotas,
        factor,
        riesgo: cansancio * factor / 10.0 + consecutivos as f64 / 3.0 + derrotas as f64 / 2.0,
    }
}

/// Consejos R1–R5 (o el consejo por defecto).
#[must_use]
pub fn consejos(e: &EstadoJugador) -> Vec<Consejo> {
    let mut lista = Vec::new();
    let mut agregar = |condicion: bool, texto, severidad, area| {
        if condicion {
            lista.push(Consejo {
                texto,
                severidad,
                area,
            });
        }
    };
    agregar(
        e.cansancio > 8.0 && e.consecutivos >= 3,
        "Se recomienda descanso para reducir riesgo de lesión",
        Severidad::Advertencia,
        "Físico",
    );
    agregar(
        e.rendimiento < 3.0,
        "Rendimiento bajo detectado. Considerar rotación de jugadores",
        Severidad::Advertencia,
        "Deportivo",
    );
    agregar(
        e.animo < 3.0 && e.derrotas >= 2,
        "Confianza baja por racha negativa. Motivar al equipo",
        Severidad::Advertencia,
        "Mental",
    );
    agregar(
        e.riesgo > 3.0,
        "Riesgo de lesión muy elevado. Descanso obligatorio",
        Severidad::Critico,
        "Salud",
    );
    agregar(
        e.dias_descanso > 14,
        "Demasiado tiempo sin jugar. Considerar partido amistoso",
        Severidad::Info,
        "Deportivo",
    );
    if lista.is_empty() {
        lista.push(Consejo {
            texto: "Estado general bueno. Mantener rutina actual",
            severidad: Severidad::Info,
            area: "Físico",
        });
    }
    lista
}

/// Probabilidades (victoria, empate, derrota) en %.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn prediccion(e: &EstadoJugador) -> (f64, f64, f64) {
    let victoria = (50.0 + (e.rendimiento - 5.0) * 5.0 - (e.cansancio - 5.0) * 3.0
        + (e.animo - 5.0) * 4.0
        - e.derrotas as f64 * 5.0)
        .clamp(5.0, 95.0);
    let empate = (100.0 - victoria) * 0.3;
    (victoria, empate, 100.0 - victoria - empate)
}

/// Formación sugerida (primera regla que aplica) y recomendaciones tácticas.
#[must_use]
pub fn formacion(e: &EstadoJugador) -> (&'static str, Vec<&'static str>) {
    let base = if e.cansancio > 7.0 {
        "4-5-1 (Defensiva)"
    } else if e.rendimiento >= 7.0 {
        "4-3-3 (Ofensiva)"
    } else if e.derrotas >= 2 {
        "4-4-2 (Equilibrada)"
    } else if e.animo < 4.0 {
        "5-3-2 (Conservadora)"
    } else {
        "4-2-3-1 (Balanceada)"
    };
    let mut extras = Vec::new();
    if e.cansancio > 6.0 {
        extras.push("Rotar jugadores para dosificar el cansancio");
    }
    if e.riesgo > 2.0 {
        extras.push("Evitar exponer a jugadores con riesgo de lesión");
    }
    if e.animo < 5.0 {
        extras.push("Charla motivacional antes del partido");
    }
    (base, extras)
}

/// Días de descanso sugeridos, prioridad y nota crítica.
#[must_use]
pub fn descanso(e: &EstadoJugador) -> ((i64, &'static str), Option<&'static str>) {
    let sugerencia = if e.cansancio >= 8.0 {
        (7, "URGENTE")
    } else if e.cansancio >= 6.0 || e.consecutivos >= 4 {
        (5, "Recomendado")
    } else if e.consecutivos >= 3 {
        (3, "Sugerido")
    } else if e.dias_descanso >= 10 {
        (0, "Ya descansaste lo suficiente")
    } else {
        (2, "Mantenimiento")
    };
    let nota = (e.riesgo > 2.5).then_some("Riesgo crítico: priorizar el descanso antes de jugar");
    (sugerencia, nota)
}

/// Puntos débiles según los umbrales de MiFutbolC.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn puntos_debiles(a: &Agregados) -> Vec<String> {
    if a.partidos == 0 {
        return Vec::new();
    }
    let mut lista = Vec::new();
    if a.goles < 1.5 {
        lista.push(format!(
            "Pocos goles: {:.2} por partido (objetivo ≥ 1.5)",
            a.goles
        ));
    }
    if a.asistencias < 1.0 {
        lista.push(format!(
            "Pocas asistencias: {:.2} por partido (objetivo ≥ 1.0)",
            a.asistencias
        ));
    }
    let lesiones = a.lesiones as f64 / a.partidos as f64;
    if lesiones > 0.3 {
        lista.push(format!(
            "Muchas lesiones: {lesiones:.2} por partido (límite 0.3)"
        ));
    }
    let rango = a.rendimiento_max - a.rendimiento_min;
    if rango > 6 {
        lista.push(format!(
            "Rendimiento irregular: varía {rango} puntos entre partidos"
        ));
    }
    lista
}

/// Umbral de riesgo para alertar según el nivel de intervención (0 = nunca).
#[must_use]
pub const fn umbral(nivel: i64) -> Option<f64> {
    match nivel {
        1 => Some(4.0),
        2 => Some(3.0),
        3 => Some(2.0),
        _ => None,
    }
}

/// Análisis completo con los datos actuales.
pub fn analizar(conn: &Connection, hoy: Date) -> Result<Analisis, ServiceError> {
    let estado = evaluar(&entrenador_repo::ultimos_partidos(conn, 10)?, hoy);
    let nivel = entrenador_repo::nivel(conn)?;
    let (formacion, tacticas) = formacion(&estado);
    let (descanso, nota_descanso) = descanso(&estado);
    Ok(Analisis {
        consejos: consejos(&estado),
        prediccion: prediccion(&estado),
        formacion,
        tacticas,
        descanso,
        nota_descanso,
        puntos_debiles: puntos_debiles(&entrenador_repo::agregados(conn)?),
        nivel,
        alerta_previa: estado.partidos > 0
            && umbral(nivel)
                .is_some_and(|u| estado.riesgo > u || estado.cansancio * estado.factor > 8.5),
        historial: entrenador_repo::historial(conn)?,
        estado,
    })
}

/// Horas mínimas entre avisos previos a un partido (`IA_HORAS_ENFRIAMIENTO`).
const ENFRIAMIENTO_SEG: i64 = 24 * 60 * 60;
/// Margen sobre el umbral a partir del cual el riesgo es crítico y se avisa siempre.
const MARGEN_CRITICO: f64 = 1.0;

/// Aviso del Entrenador IA al empezar a cargar un partido
/// (`activar_ia_antes_partido` de MiFutbolC): riesgo sobre el umbral del nivel
/// de intervención o fatiga extrema. Como máximo uno cada 24 h salvo riesgo
/// crítico; cuando avisa, guarda `ultima_alerta` y `ultimo_riesgo`.
pub fn alerta_antes_partido(
    conn: &Connection,
    hoy: Date,
    ahora: i64,
) -> Result<Option<String>, ServiceError> {
    let analisis = analizar(conn, hoy)?;
    let Some(umbral) = umbral(analisis.nivel).filter(|_| analisis.alerta_previa) else {
        return Ok(None);
    };
    let riesgo = analisis.estado.riesgo;
    let (ultima, previo) = entrenador_repo::ultima_alerta(conn)?;
    let critico = riesgo >= umbral + MARGEN_CRITICO;
    if !critico && ultima > 0 && ahora - ultima < ENFRIAMIENTO_SEG {
        return Ok(None);
    }
    crate::util::logging::info(&format!(
        "Aviso IA antes de partido: riesgo {riesgo:.2} (umbral {umbral:.2}, nivel {}, aviso previo {previo:.2})",
        analisis.nivel
    ));
    entrenador_repo::registrar_alerta(conn, ahora, riesgo)?;
    let mut texto = format!(
        "Entrenador IA: alto riesgo detectado (riesgo {riesgo:.2}, umbral {umbral:.2}).
         Consejos antes de continuar:"
    );
    for consejo in &analisis.consejos {
        texto.push_str(&format!(
            "
• {}",
            consejo.texto
        ));
    }
    Ok(Some(texto))
}

/// Registra si se siguieron los consejos mostrados.
pub fn registrar(
    conn: &Connection,
    consejos: &[Consejo],
    seguido: bool,
) -> Result<(), ServiceError> {
    for consejo in consejos {
        entrenador_repo::registrar_consejo(conn, consejo.texto, seguido)?;
    }
    Ok(())
}

/// Avanza el nivel de intervención 0 → 1 → 2 → 3 → 0.
pub fn ciclar_nivel(conn: &Connection) -> Result<i64, ServiceError> {
    let nivel = (entrenador_repo::nivel(conn)? + 1) % 4;
    entrenador_repo::set_nivel(conn, nivel)?;
    Ok(nivel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn hoy() -> Date {
        crate::domain::dates::fecha(2026, 11, 17).unwrap()
    }

    fn partido(dias_atras: i64, rend: i64, cans: i64, animo: i64, resultado: i64) -> PartidoIa {
        let fecha = hoy() - jiff::Span::new().days(dias_atras);
        PartidoIa {
            fecha_hora: format!("{} 20:00:00", dates::to_iso_date(fecha)),
            rendimiento: rend,
            cansancio: cans,
            animo,
            resultado,
        }
    }

    fn estado(cansancio: f64, rendimiento: f64, animo: f64) -> EstadoJugador {
        EstadoJugador {
            partidos: 5,
            rendimiento,
            cansancio,
            animo,
            ..EstadoJugador::default()
        }
    }

    fn textos(e: &EstadoJugador) -> Vec<&'static str> {
        consejos(e).iter().map(|c| c.texto).collect()
    }

    #[test]
    fn evaluates_rest_streaks_and_risk() {
        // Tres partidos en la última semana, el último hoy, dos derrotas al frente.
        let partidos = vec![
            partido(0, 4, 9, 2, 3),
            partido(2, 4, 9, 2, 3),
            partido(5, 4, 9, 2, 1),
            partido(20, 4, 9, 2, 3),
        ];
        let e = evaluar(&partidos, hoy());
        assert_eq!((e.dias_descanso, e.consecutivos, e.derrotas), (0, 3, 2));
        assert!((e.factor - 1.0).abs() < 1e-9);
        // 9*1/10 + 3/3 + 2/2 = 2.9.
        assert!((e.riesgo - 2.9).abs() < 1e-9);

        // Resultado 0 ("sin cargar") no cuenta como derrota.
        assert_eq!(evaluar(&[partido(1, 5, 5, 5, 0)], hoy()).derrotas, 0);
        // Factor: 7 días → 0.5, 14 → 0, 3 → 1 - 3/14.
        assert!((evaluar(&[partido(7, 5, 5, 5, 1)], hoy()).factor - 0.5).abs() < 1e-9);
        assert!(evaluar(&[partido(14, 5, 5, 5, 1)], hoy()).factor.abs() < 1e-9);
        assert!(
            (evaluar(&[partido(3, 5, 5, 5, 1)], hoy()).factor - (1.0 - 3.0 / 14.0)).abs() < 1e-9
        );
        assert_eq!(evaluar(&[], hoy()), EstadoJugador::default());
    }

    #[test]
    fn each_rule_fires_at_its_threshold() {
        let mut e = estado(8.5, 5.0, 5.0);
        e.consecutivos = 3;
        assert!(textos(&e)[0].starts_with("Se recomienda descanso"));
        e.consecutivos = 2;
        assert!(
            !textos(&e)
                .iter()
                .any(|t| t.starts_with("Se recomienda descanso"))
        );

        assert!(textos(&estado(5.0, 2.9, 5.0))[0].starts_with("Rendimiento bajo"));
        assert!(
            !textos(&estado(5.0, 3.0, 5.0))
                .iter()
                .any(|t| t.starts_with("Rendimiento bajo"))
        );

        let mut e = estado(5.0, 5.0, 2.0);
        e.derrotas = 2;
        assert!(textos(&e).iter().any(|t| t.starts_with("Confianza baja")));

        let mut e = estado(5.0, 5.0, 5.0);
        e.riesgo = 3.01;
        let c = consejos(&e);
        assert_eq!(c[0].severidad, Severidad::Critico);

        let mut e = estado(5.0, 5.0, 5.0);
        e.dias_descanso = 15;
        assert!(textos(&e)[0].starts_with("Demasiado tiempo sin jugar"));

        assert_eq!(
            textos(&estado(5.0, 5.0, 5.0)),
            ["Estado general bueno. Mantener rutina actual"]
        );
    }

    #[test]
    fn prediction_formation_rest_and_weak_points() {
        let mut e = estado(5.0, 7.0, 6.0);
        e.derrotas = 1;
        // 50 + 10 - 0 + 4 - 5 = 59; empate 41*0.3 = 12.3.
        let (v, em, d) = prediccion(&e);
        assert!((v - 59.0).abs() < 1e-9 && (em - 12.3).abs() < 1e-9 && (d - 28.7).abs() < 1e-9);
        assert!(
            (prediccion(&estado(10.0, 0.0, 0.0)).0 - 5.0).abs() < 1e-9,
            "mínimo 5%"
        );

        assert_eq!(formacion(&estado(7.5, 9.0, 9.0)).0, "4-5-1 (Defensiva)");
        assert_eq!(formacion(&e).0, "4-3-3 (Ofensiva)");
        assert_eq!(formacion(&estado(5.0, 5.0, 3.0)).0, "5-3-2 (Conservadora)");
        assert_eq!(formacion(&estado(5.0, 5.0, 6.0)).0, "4-2-3-1 (Balanceada)");
        assert_eq!(formacion(&estado(6.5, 5.0, 4.5)).1.len(), 2);

        assert_eq!(descanso(&estado(8.0, 5.0, 5.0)).0, (7, "URGENTE"));
        assert_eq!(descanso(&estado(6.0, 5.0, 5.0)).0.0, 5);
        let mut e = estado(1.0, 5.0, 5.0);
        e.consecutivos = 3;
        e.riesgo = 2.6;
        assert_eq!(descanso(&e).0.0, 3);
        assert!(descanso(&e).1.is_some());
        assert_eq!(descanso(&estado(1.0, 5.0, 5.0)).0.0, 2);

        let a = Agregados {
            partidos: 10,
            goles: 1.4,
            asistencias: 1.0,
            rendimiento_max: 9,
            rendimiento_min: 2,
            lesiones: 4,
        };
        let debiles = puntos_debiles(&a);
        assert_eq!(debiles.len(), 3, "{debiles:?}");
        assert!(puntos_debiles(&Agregados::default()).is_empty());
    }

    #[test]
    fn analysis_reads_db_and_stores_history_and_level() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        conn.execute_batch(
            "INSERT INTO cancha (nombre) VALUES ('Sur');
             INSERT INTO camiseta (nombre) VALUES ('Azul');",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, rendimiento_general,
                                  cansancio, estado_animo, resultado)
             VALUES (1, 1, '2026-11-16 20:00:00', 8, 9, 7, 1)",
            [],
        )
        .unwrap();
        let a = analizar(conn, hoy()).unwrap();
        assert_eq!(a.estado.partidos, 1);
        assert_eq!(a.nivel, 2);
        // cansancio 9 * factor (1 - 1/14) = 8.36 < 8.5 y riesgo 1.17 < 3 → sin alerta.
        assert!(!a.alerta_previa);
        registrar(conn, &a.consejos, true).unwrap();
        assert_eq!(
            entrenador_repo::historial(conn).unwrap(),
            (a.consejos.len() as i64, 0)
        );
        assert_eq!(ciclar_nivel(conn).unwrap(), 3);
        assert_eq!(ciclar_nivel(conn).unwrap(), 0);
        assert!(!analizar(conn, hoy()).unwrap().alerta_previa);
    }

    #[test]
    fn pre_match_alert_respects_level_cooldown_and_critical_risk() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        conn.execute_batch(
            "INSERT INTO cancha (nombre) VALUES ('Sur');
             INSERT INTO camiseta (nombre) VALUES ('Azul');",
        )
        .unwrap();
        let cargar = |fechas: &[&str]| {
            for fecha in fechas {
                conn.execute(
                    "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, rendimiento_general,
                                          cansancio, estado_animo, resultado)
                     VALUES (1, 1, ?1, 5, 8, 5, 3)",
                    [fecha],
                )
                .unwrap();
            }
        };
        let ahora = 1_800_000_000;
        assert_eq!(alerta_antes_partido(conn, hoy(), ahora).unwrap(), None);

        // 2 derrotas (hoy y ayer), cansancio 8: 0.8 + 2/3 + 1 = 2.47 → bajo el umbral moderado (3).
        cargar(&["2026-11-16 20:00:00", "2026-11-17 10:00:00"]);
        assert_eq!(alerta_antes_partido(conn, hoy(), ahora).unwrap(), None);
        // Nivel agresivo (umbral 2): avisa y guarda el aviso.
        entrenador_repo::set_nivel(conn, 3).unwrap();
        let aviso = alerta_antes_partido(conn, hoy(), ahora).unwrap().unwrap();
        assert!(aviso.contains("umbral 2.00"), "{aviso}");
        let (ultima, riesgo) = entrenador_repo::ultima_alerta(conn).unwrap();
        assert_eq!(ultima, ahora);
        assert!((riesgo - (0.8 + 2.0 / 3.0 + 1.0)).abs() < 1e-9);
        // Dentro de las 24 h no se repite; después sí.
        assert_eq!(
            alerta_antes_partido(conn, hoy(), ahora + 3600).unwrap(),
            None
        );
        assert!(
            alerta_antes_partido(conn, hoy(), ahora + 25 * 3600)
                .unwrap()
                .is_some()
        );
        // Riesgo crítico (0.8 + 4/3 + 2 ≥ umbral + 1): avisa aunque haya un aviso reciente.
        cargar(&["2026-11-15 20:00:00", "2026-11-14 20:00:00"]);
        assert!(
            alerta_antes_partido(conn, hoy(), ahora + 25 * 3600 + 60)
                .unwrap()
                .is_some()
        );
        // Silencioso: nunca avisa.
        entrenador_repo::set_nivel(conn, 0).unwrap();
        assert_eq!(
            alerta_antes_partido(conn, hoy(), ahora + 90 * 3600).unwrap(),
            None
        );
    }
}
