//! Clima histórico de cada partido con Open-Meteo (archive API), en segundo plano.
//!
//! Port de `openmeteo_client.c` + `clima_diferido.c` de MiFutbolC: una cola
//! (`partido_clima_pendiente`) que se procesa de a una consulta por vez sin
//! bloquear la interfaz. La conexión SQLite solo se usa en el hilo principal;
//! el hilo de trabajo únicamente hace la petición HTTP.

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::db::DbError;

/// Reintentos máximos por partido (igual que MiFutbolC).
pub const MAX_INTENTOS: i64 = 8;
/// Un partido más nuevo que esto todavía puede no estar en el archivo histórico:
/// un fallo no consume intento.
const DIAS_RECIENTE: f64 = 14.0;

/// Clima agregado del día del partido.
#[derive(Debug, Clone, PartialEq)]
pub struct ClimaDia {
    /// Temperatura media (°C).
    pub temperatura_c: f64,
    /// Sensación térmica media (°C).
    pub apparent_temp_c: Option<f64>,
    /// Precipitación total del día (mm).
    pub precip_mm: Option<f64>,
    /// Viento medio (km/h).
    pub wind_kmh: Option<f64>,
    /// Código WMO más frecuente del día.
    pub weather_code: Option<i64>,
    /// Franja horaria (códigos de `FranjaHoraria`) según amanecer/atardecer.
    pub dia: Option<i64>,
    /// Respuesta completa, para consultas futuras.
    pub json: String,
}

/// Partido en cola con lo necesario para consultar y validar al aplicar.
#[derive(Debug, Clone, PartialEq)]
pub struct Pendiente {
    /// Partido.
    pub partido_id: i64,
    /// Cancha al momento de pedir el clima.
    pub cancha_id: i64,
    /// `YYYY-MM-DD HH:MM[:SS]` al momento de pedir el clima.
    pub fecha_hora: String,
    /// Latitud de la cancha.
    pub latitud: f64,
    /// Longitud de la cancha.
    pub longitud: f64,
    /// Partido de los últimos 14 días.
    pub reciente: bool,
}

/// URL de la API histórica para un día.
#[must_use]
pub fn url(latitud: f64, longitud: f64, fecha: &str) -> String {
    format!(
        "https://archive-api.open-meteo.com/v1/archive?latitude={latitud:.6}&longitude={longitud:.6}\
         &start_date={fecha}&end_date={fecha}&daily=sunrise,sunset\
         &hourly=temperature_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m\
         &timezone=auto"
    )
}

/// Agrega la respuesta horaria al clima del día. `hora` = minutos desde las 00:00
/// del partido, para recalcular la franja con amanecer y atardecer.
pub fn parsear(json: &str, hora: Option<u32>) -> Result<ClimaDia, String> {
    let raiz: Value =
        serde_json::from_str(json).map_err(|error| format!("JSON inválido: {error}"))?;
    let serie = |campo: &str| -> Vec<f64> {
        raiz["hourly"][campo]
            .as_array()
            .map(|valores| valores.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default()
    };
    let media = |valores: &[f64]| {
        (!valores.is_empty()).then(|| valores.iter().sum::<f64>() / valores.len() as f64)
    };

    // Sin temperaturas el día todavía no está publicado: es un fallo reintentable.
    let temperatura_c =
        media(&serie("temperature_2m")).ok_or("El día todavía no tiene datos publicados")?;
    let precipitacion = serie("precipitation");
    let mut frecuencia = [0_u32; 100];
    for codigo in serie("weather_code") {
        if (0.0..100.0).contains(&codigo) {
            frecuencia[codigo as usize] += 1;
        }
    }
    let weather_code = frecuencia
        .iter()
        .enumerate()
        .filter(|(_, veces)| **veces > 0)
        // Empates: el código más bajo, para que el resultado sea determinista.
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)))
        .map(|(codigo, _)| codigo as i64);

    let minutos = |campo: &str| -> Option<u32> {
        let texto = raiz["daily"][campo][0].as_str()?;
        let (_, hhmm) = texto.split_once('T')?;
        let (h, m) = hhmm.split_once(':')?;
        Some(h.parse::<u32>().ok()? * 60 + m.get(..2)?.parse::<u32>().ok()?)
    };
    let dia = match (hora, minutos("sunrise"), minutos("sunset")) {
        (Some(hora), Some(amanecer), Some(atardecer)) => Some(franja(hora, amanecer, atardecer)),
        _ => None,
    };

    Ok(ClimaDia {
        temperatura_c,
        apparent_temp_c: media(&serie("apparent_temperature")),
        precip_mm: (!precipitacion.is_empty()).then(|| precipitacion.iter().sum()),
        wind_kmh: media(&serie("wind_speed_10m")),
        weather_code,
        dia,
        json: raiz.to_string(),
    })
}

/// Franja de MiFutbolC (1 Madrugada … 6 Noche), mismos códigos que `FranjaHoraria`.
fn franja(hora: u32, amanecer: u32, atardecer: u32) -> i64 {
    if hora < amanecer {
        1
    } else if hora < 12 * 60 {
        2
    } else if hora < 15 * 60 {
        3
    } else if hora + 60 < atardecer {
        4
    } else if hora < atardecer + 60 {
        5
    } else {
        6
    }
}

/// Minutos desde las 00:00 de `YYYY-MM-DD HH:MM[:SS]`.
fn minutos_de(fecha_hora: &str) -> Option<u32> {
    let h = fecha_hora.get(11..13)?.parse::<u32>().ok()?;
    let m = fecha_hora.get(14..16)?.parse::<u32>().ok()?;
    Some(h * 60 + m)
}

/// Encola un partido si su cancha tiene coordenadas y aún no tiene clima.
pub fn encolar(conn: &Connection, partido_id: i64) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO partido_clima_pendiente (partido_id)
         SELECT p.id FROM partido p JOIN cancha c ON c.id = p.cancha_id
          WHERE p.id = ?1 AND p.temperatura_c IS NULL
            AND c.latitud IS NOT NULL AND c.longitud IS NOT NULL
            AND NOT (c.latitud = 0 AND c.longitud = 0)
         ON CONFLICT(partido_id) DO UPDATE SET intentos = 0",
        params![partido_id],
    )?;
    Ok(())
}

/// Encola todos los partidos sin clima cuya cancha tiene coordenadas.
pub fn encolar_sin_clima(conn: &Connection) -> Result<usize, DbError> {
    Ok(conn.execute(
        "INSERT INTO partido_clima_pendiente (partido_id)
         SELECT p.id FROM partido p JOIN cancha c ON c.id = p.cancha_id
          WHERE p.temperatura_c IS NULL
            AND c.latitud IS NOT NULL AND c.longitud IS NOT NULL
            AND NOT (c.latitud = 0 AND c.longitud = 0)
         ON CONFLICT(partido_id) DO UPDATE SET intentos = 0",
        [],
    )?)
}

/// Descarta lo que ya no tiene sentido consultar y devuelve los próximos pendientes
/// (sin intentos agotados ni fechas futuras), en orden de partido.
pub fn pendientes(conn: &Connection, limite: usize) -> Result<Vec<Pendiente>, DbError> {
    conn.execute(
        "DELETE FROM partido_clima_pendiente
          WHERE partido_id NOT IN (
                SELECT p.id FROM partido p JOIN cancha c ON c.id = p.cancha_id
                 WHERE p.temperatura_c IS NULL
                   AND c.latitud IS NOT NULL AND c.longitud IS NOT NULL
                   AND NOT (c.latitud = 0 AND c.longitud = 0)
                   AND date(p.fecha_hora) IS NOT NULL)",
        [],
    )?;
    let mut stmt = conn.prepare(
        "SELECT p.id, p.cancha_id, p.fecha_hora, c.latitud, c.longitud,
                julianday('now','localtime') - julianday(p.fecha_hora) < ?2
           FROM partido_clima_pendiente pc
           JOIN partido p ON p.id = pc.partido_id
           JOIN cancha c ON c.id = p.cancha_id
          WHERE pc.intentos < ?1 AND date(p.fecha_hora) <= date('now','localtime')
          ORDER BY pc.partido_id
          LIMIT ?3",
    )?;
    let filas = stmt.query_map(params![MAX_INTENTOS, DIAS_RECIENTE, limite as i64], |row| {
        Ok(Pendiente {
            partido_id: row.get(0)?,
            cancha_id: row.get(1)?,
            fecha_hora: row.get(2)?,
            latitud: row.get(3)?,
            longitud: row.get(4)?,
            reciente: row.get(5)?,
        })
    })?;
    Ok(filas.collect::<Result<_, _>>()?)
}

/// Guarda el clima si el partido no cambió de cancha ni de fecha mientras se
/// consultaba. Devuelve `false` (y lo deja en cola) si cambió.
pub fn aplicar(
    conn: &mut Connection,
    pendiente: &Pendiente,
    clima: &ClimaDia,
) -> Result<bool, DbError> {
    let tx = conn.transaction()?;
    // Dato automático: no debe ocupar el historial de deshacer del usuario.
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])?;
    let cambiadas = tx.execute(
        "UPDATE partido
            SET temperatura_c = ?1, apparent_temp_c = ?2, precip_mm = ?3, wind_kmh = ?4,
                weather_code = ?5, clima_json = ?6, dia = COALESCE(NULLIF(?7, 0), dia)
          WHERE id = ?8 AND cancha_id = ?9 AND fecha_hora = ?10",
        params![
            clima.temperatura_c,
            clima.apparent_temp_c,
            clima.precip_mm,
            clima.wind_kmh,
            clima.weather_code,
            clima.json,
            clima.dia,
            pendiente.partido_id,
            pendiente.cancha_id,
            pendiente.fecha_hora
        ],
    )?;
    if cambiadas == 1 {
        tx.execute(
            "DELETE FROM partido_clima_pendiente WHERE partido_id = ?1",
            params![pendiente.partido_id],
        )?;
    }
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(cambiadas == 1)
}

/// Cuenta un intento fallido, salvo en partidos recientes (el archivo histórico
/// tarda unos días en publicarlos).
pub fn registrar_fallo(conn: &Connection, pendiente: &Pendiente) -> Result<(), DbError> {
    if !pendiente.reciente {
        conn.execute(
            "UPDATE partido_clima_pendiente SET intentos = intentos + 1 WHERE partido_id = ?1",
            params![pendiente.partido_id],
        )?;
    }
    Ok(())
}

/// Intentos usados por un partido en cola (`None` si no está en cola).
pub fn intentos(conn: &Connection, partido_id: i64) -> Result<Option<i64>, DbError> {
    Ok(conn
        .query_row(
            "SELECT intentos FROM partido_clima_pendiente WHERE partido_id = ?1",
            params![partido_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Descarga una URL (inyectable para tests).
pub type Fetcher = fn(&str) -> Result<String, String>;

/// Petición real a Open-Meteo con los tiempos de MiFutbolC (5 s conexión, 15 s total).
pub fn fetch_http(url: &str) -> Result<String, String> {
    let agente: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    agente
        .get(url)
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())
}

/// Resultado de una consulta terminada.
pub type Respuesta = (Pendiente, Result<String, String>);

/// Hilo de consultas: una petición en vuelo como máximo, cada partido una vez
/// por sesión. El hilo se crea con la primera petición.
pub struct ClimaWorker {
    fetcher: Fetcher,
    canal: Option<(Sender<Pendiente>, Receiver<Respuesta>)>,
    en_vuelo: bool,
    intentados: HashSet<i64>,
}

impl Default for ClimaWorker {
    fn default() -> Self {
        Self::new(fetch_http)
    }
}

impl std::fmt::Debug for ClimaWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClimaWorker")
            .field("en_vuelo", &self.en_vuelo)
            .field("intentados", &self.intentados.len())
            .finish_non_exhaustive()
    }
}

impl ClimaWorker {
    /// Worker con un fetcher concreto.
    #[must_use]
    pub fn new(fetcher: Fetcher) -> Self {
        Self {
            fetcher,
            canal: None,
            en_vuelo: false,
            intentados: HashSet::new(),
        }
    }

    /// `true` si hay una consulta en curso.
    #[must_use]
    pub const fn ocupado(&self) -> bool {
        self.en_vuelo
    }

    /// ¿Ya se intentó este partido en la sesión?
    #[must_use]
    pub fn intentado(&self, partido_id: i64) -> bool {
        self.intentados.contains(&partido_id)
    }

    /// Permite reintentar en esta sesión (acción manual).
    pub fn olvidar_intentos(&mut self) {
        self.intentados.clear();
    }

    /// Lanza la consulta en el hilo de trabajo.
    pub fn enviar(&mut self, pendiente: Pendiente) {
        let fetcher = self.fetcher;
        let (envio, _) = self.canal.get_or_insert_with(|| {
            let (envio, entrada) = mpsc::channel::<Pendiente>();
            let (salida, resultados) = mpsc::channel::<Respuesta>();
            std::thread::spawn(move || {
                for pendiente in entrada {
                    let fecha = pendiente
                        .fecha_hora
                        .get(..10)
                        .unwrap_or_default()
                        .to_string();
                    let resultado = fetcher(&url(pendiente.latitud, pendiente.longitud, &fecha));
                    if salida.send((pendiente, resultado)).is_err() {
                        break;
                    }
                }
            });
            (envio, resultados)
        });
        self.intentados.insert(pendiente.partido_id);
        self.en_vuelo = envio.send(pendiente).is_ok();
    }

    /// Resultado listo, sin esperar.
    pub fn recibir(&mut self) -> Option<Respuesta> {
        let respuesta = self.canal.as_ref()?.1.try_recv().ok()?;
        self.en_vuelo = false;
        Some(respuesta)
    }

    /// Espera un resultado (solo tests).
    pub fn esperar(&mut self, limite: Duration) -> Option<Respuesta> {
        let respuesta = self.canal.as_ref()?.1.recv_timeout(limite).ok()?;
        self.en_vuelo = false;
        Some(respuesta)
    }
}

/// Procesa un resultado: aplica el clima o cuenta el fallo. Devuelve el mensaje
/// para la barra de estado cuando se guardó el clima.
pub fn procesar(
    conn: &mut Connection,
    (pendiente, resultado): Respuesta,
) -> Result<Option<String>, DbError> {
    let clima = resultado.and_then(|json| parsear(&json, minutos_de(&pendiente.fecha_hora)));
    match clima {
        Ok(clima) => Ok(aplicar(conn, &pendiente, &clima)?
            .then(|| format!("Clima actualizado: partido #{}", pendiente.partido_id))),
        Err(_) => {
            registrar_fallo(conn, &pendiente)?;
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    /// Respuesta con la forma de Open-Meteo (3 horas, una con nulos).
    const FIXTURE: &str = r#"{
        "latitude": -34.72, "longitude": -58.39,
        "daily": {"time": ["2025-03-10"], "sunrise": ["2025-03-10T07:05"], "sunset": ["2025-03-10T19:40"]},
        "hourly": {
            "time": ["2025-03-10T19:00", "2025-03-10T20:00", "2025-03-10T21:00"],
            "temperature_2m": [20.0, 22.0, null],
            "apparent_temperature": [19.0, 21.0, null],
            "precipitation": [0.5, 1.5, null],
            "weather_code": [61, 61, 3],
            "wind_speed_10m": [10.0, 14.0, null]
        }
    }"#;

    #[test]
    fn parses_daily_aggregates_like_mifutbolc() {
        let clima = parsear(FIXTURE, Some(20 * 60)).unwrap();
        assert!((clima.temperatura_c - 21.0).abs() < 1e-9);
        assert_eq!(clima.apparent_temp_c, Some(20.0));
        assert_eq!(clima.precip_mm, Some(2.0));
        assert_eq!(clima.wind_kmh, Some(12.0));
        assert_eq!(clima.weather_code, Some(61));
        // 20:00 con atardecer 19:40 → dentro de la hora posterior → Atardecer (5).
        assert_eq!(clima.dia, Some(5));
        assert!(clima.json.contains("archive") || clima.json.contains("hourly"));
    }

    #[test]
    fn franja_matches_c_thresholds() {
        let (amanecer, atardecer) = (7 * 60, 19 * 60 + 40);
        assert_eq!(franja(6 * 60, amanecer, atardecer), 1);
        assert_eq!(franja(10 * 60, amanecer, atardecer), 2);
        assert_eq!(franja(13 * 60, amanecer, atardecer), 3);
        assert_eq!(franja(16 * 60, amanecer, atardecer), 4);
        assert_eq!(franja(20 * 60, amanecer, atardecer), 5);
        assert_eq!(franja(22 * 60, amanecer, atardecer), 6);
    }

    #[test]
    fn missing_temperatures_is_a_retryable_error() {
        let json = r#"{"hourly": {"temperature_2m": [null, null]}}"#;
        assert!(parsear(json, None).is_err());
        assert!(parsear("no es json", None).is_err());
    }

    #[test]
    fn url_has_archive_params() {
        let url = url(-34.7252, -58.3941, "2025-03-10");
        assert!(
            url.starts_with("https://archive-api.open-meteo.com/v1/archive?latitude=-34.725200")
        );
        assert!(url.contains("start_date=2025-03-10&end_date=2025-03-10"));
        assert!(url.contains(
            "hourly=temperature_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m"
        ));
    }

    fn base() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (id, nombre, latitud, longitud) VALUES (1, 'Con GPS', -34.72, -58.39);
                 INSERT INTO cancha (id, nombre) VALUES (2, 'Sin GPS');
                 INSERT INTO camiseta (id, nombre) VALUES (1, 'Azul');
                 INSERT INTO partido (id, cancha_id, camiseta_id, fecha_hora) VALUES
                    (1, 1, 1, '2025-03-10 20:00:00'),
                    (2, 2, 1, '2025-03-10 20:00:00'),
                    (3, 1, 1, '2999-01-01 20:00:00');",
            )
            .unwrap();
        db
    }

    fn fake_ok(_: &str) -> Result<String, String> {
        Ok(FIXTURE.to_string())
    }

    fn fake_error(_: &str) -> Result<String, String> {
        Err("sin red".to_string())
    }

    #[test]
    fn queue_skips_no_coords_and_future_dates() {
        let db = base();
        assert_eq!(
            encolar_sin_clima(db.conn()).unwrap(),
            2,
            "solo canchas con GPS"
        );
        let lista = pendientes(db.conn(), 10).unwrap();
        assert_eq!(lista.len(), 1, "el partido futuro espera sin costo");
        assert_eq!(lista[0].partido_id, 1);
        assert!(!lista[0].reciente);
        assert_eq!(intentos(db.conn(), 3).unwrap(), Some(0));
    }

    #[test]
    fn worker_fetches_in_background_and_applies() {
        let mut db = base();
        encolar(db.conn(), 1).unwrap();
        let mut worker = ClimaWorker::new(fake_ok);
        let siguiente = pendientes(db.conn(), 1).unwrap().remove(0);
        worker.enviar(siguiente);
        assert!(worker.ocupado());
        assert!(worker.intentado(1));
        let respuesta = worker.esperar(Duration::from_secs(5)).expect("respuesta");
        let mensaje = procesar(db.conn_mut(), respuesta).unwrap();
        assert_eq!(mensaje.as_deref(), Some("Clima actualizado: partido #1"));
        let (temp, codigo, dia, undo): (f64, i64, i64, i64) = db
            .conn()
            .query_row(
                "SELECT temperatura_c, weather_code, dia, (SELECT COUNT(*) FROM undo_historial WHERE tabla = 'partido' AND operacion = 'editar')
                   FROM partido WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert!((temp - 21.0).abs() < 1e-9);
        assert_eq!((codigo, dia, undo), (61, 5, 0));
        assert_eq!(intentos(db.conn(), 1).unwrap(), None, "sale de la cola");
    }

    #[test]
    fn failure_costs_attempt_and_changed_match_stays_pending() {
        let mut db = base();
        encolar(db.conn(), 1).unwrap();
        let mut worker = ClimaWorker::new(fake_error);
        worker.enviar(pendientes(db.conn(), 1).unwrap().remove(0));
        let respuesta = worker.esperar(Duration::from_secs(5)).unwrap();
        assert_eq!(procesar(db.conn_mut(), respuesta).unwrap(), None);
        assert_eq!(intentos(db.conn(), 1).unwrap(), Some(1));

        // Cambió la fecha mientras se consultaba: no se aplica y sigue en cola.
        let mut viejo = pendientes(db.conn(), 1).unwrap().remove(0);
        db.conn()
            .execute(
                "UPDATE partido SET fecha_hora = '2025-03-11 20:00:00' WHERE id = 1",
                [],
            )
            .unwrap();
        viejo.reciente = false;
        let clima = parsear(FIXTURE, None).unwrap();
        assert!(!aplicar(db.conn_mut(), &viejo, &clima).unwrap());
        assert!(intentos(db.conn(), 1).unwrap().is_some());
    }

    #[test]
    fn exhausted_attempts_are_not_selected() {
        let db = base();
        encolar(db.conn(), 1).unwrap();
        db.conn()
            .execute(
                "UPDATE partido_clima_pendiente SET intentos = ?1",
                params![MAX_INTENTOS],
            )
            .unwrap();
        assert!(pendientes(db.conn(), 10).unwrap().is_empty());
    }
}
