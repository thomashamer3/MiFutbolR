//! Fechas, horas y formatos legacy.
//!
//! ADR-08: en la base de datos las fechas son ISO-8601
//! (`YYYY-MM-DD` o `YYYY-MM-DD HH:MM:SS`). La UI muestra `DD/MM/YYYY`.
//! El proyecto C usa formatos mixtos; aquí se toleran al importar.
//!
//! Se usa `jiff` (antes `chrono`, sin mantenimiento: RUSTSEC-2026-0543). Los textos
//! guardados en la BD no cambian: mismos formatos `strftime`.

use jiff::civil::{Date, DateTime, Time};
use jiff::{SignedDuration, ZonedDateTime};

/// Formatos de fecha aceptados en entradas del usuario.
pub const USER_DATE_INPUT: &str = "%d/%m/%Y";

/// Formato canónico de fecha en la BD.
pub const ISO_DATE: &str = "%Y-%m-%d";

/// Formato canónico de fecha-hora en la BD.
pub const ISO_DATETIME: &str = "%Y-%m-%d %H:%M:%S";

const DATE_FORMATS: &[&str] = &["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%Y/%m/%d"];

const DATETIME_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%d %H:%M",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%dT%H:%M",
    "%d/%m/%Y %H:%M:%S",
    "%d/%m/%Y %H:%M",
];

/// Parsea una fecha tolerando ISO y `DD/MM/YYYY`.
#[must_use]
pub fn parse_date(input: &str) -> Option<Date> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }
    DATE_FORMATS
        .iter()
        .find_map(|fmt| Date::strptime(fmt, text).ok())
}

/// Parsea fecha-hora tolerando ISO, `DD/MM/YYYY HH:MM` y fecha sola.
#[must_use]
pub fn parse_datetime(input: &str) -> Option<DateTime> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }
    DATETIME_FORMATS
        .iter()
        .find_map(|fmt| DateTime::strptime(fmt, text).ok())
        .or_else(|| parse_date(text).map(DateTime::from))
}

/// Parsea una hora `HH:MM` (o `HH:MM:SS`).
#[must_use]
pub fn parse_time(input: &str) -> Option<Time> {
    let text = input.trim();
    Time::strptime("%H:%M:%S", text)
        .ok()
        .or_else(|| Time::strptime("%H:%M", text).ok())
}

/// Construye una fecha validada (como `from_ymd_opt`): `None` si no existe.
#[must_use]
pub fn fecha(anio: i32, mes: u32, dia: u32) -> Option<Date> {
    Date::new(
        i16::try_from(anio).ok()?,
        i8::try_from(mes).ok()?,
        i8::try_from(dia).ok()?,
    )
    .ok()
}

/// Suma (o resta, con `dias` negativo) días a una fecha; satura en los límites.
#[must_use]
pub fn sumar_dias(fecha: Date, dias: i64) -> Date {
    fecha.saturating_add(SignedDuration::from_hours(dias.saturating_mul(24)))
}

/// Suma días a una fecha-hora (misma hora del reloj); satura en los límites.
#[must_use]
pub fn sumar_dias_dt(fecha: DateTime, dias: i64) -> DateTime {
    fecha.saturating_add(SignedDuration::from_hours(dias.saturating_mul(24)))
}

/// Suma un mes; si el día no existe en el mes destino se ajusta al último
/// (31/01 → 28/02), igual que `checked_add_months` de chrono.
#[must_use]
pub fn sumar_un_mes(fecha: Date) -> Date {
    fecha.saturating_add(jiff::Span::new().months(1))
}

/// Días completos entre dos fechas (`hasta - desde`); negativo si `hasta` es anterior.
#[must_use]
pub fn dias_entre(desde: Date, hasta: Date) -> i64 {
    i64::from((hasta - desde).get_days())
}

/// Día de la semana con lunes = 0 … domingo = 6 (`num_days_from_monday` de chrono).
#[must_use]
pub fn dia_semana_desde_lunes(fecha: Date) -> u32 {
    fecha
        .weekday()
        .to_monday_zero_offset()
        .unsigned_abs()
        .into()
}

/// Serializa una fecha a ISO para la BD.
#[must_use]
pub fn to_iso_date(date: Date) -> String {
    date.strftime(ISO_DATE).to_string()
}

/// Serializa una fecha-hora a ISO (con segundos) para la BD.
#[must_use]
pub fn to_iso_datetime(datetime: DateTime) -> String {
    datetime.strftime(ISO_DATETIME).to_string()
}

/// Describe el tiempo transcurrido entre dos fechas.
#[must_use]
pub fn tiempo_transcurrido(desde: DateTime, ahora: DateTime) -> String {
    let segundos = ahora.duration_since(desde).as_secs().max(0);
    if segundos < 60 {
        return "hace instantes".to_string();
    }
    let minutos = segundos / 60;
    if minutos < 60 {
        return format!("hace {minutos} min");
    }
    let horas = minutos / 60;
    if horas < 24 {
        return format!("hace {horas} h");
    }
    let dias = horas / 24;
    let anios = dias / 365;
    let meses = (dias % 365) / 30;
    let dias_restantes = dias % 30;
    let mut partes = Vec::new();
    if anios > 0 {
        partes.push(format!("{anios} a"));
    }
    if meses > 0 {
        partes.push(format!("{meses} m"));
    }
    if dias_restantes > 0 || partes.is_empty() {
        partes.push(format!("{dias_restantes} d"));
    }
    format!("hace {}", partes.join(" "))
}

/// Formato de fecha para la UI: `DD/MM/YYYY`.
#[must_use]
pub fn display_date(date: Date) -> String {
    date.strftime(USER_DATE_INPUT).to_string()
}

/// Formato de fecha-hora para la UI: `DD/MM/YYYY HH:MM`.
#[must_use]
pub fn display_datetime(datetime: DateTime) -> String {
    datetime.strftime("%d/%m/%Y %H:%M").to_string()
}

/// Clave `YYYY-MM` de un partido (columna legacy `mes_anio`).
#[must_use]
pub fn month_key(datetime: DateTime) -> String {
    format!("{:04}-{:02}", datetime.year(), datetime.month())
}

/// Clave `YYYY-MM` a partir de un texto ya normalizado.
#[must_use]
pub fn month_key_from_date(date: Date) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

/// Rango `[inicio, fin)` de un mes `YYYY-MM` para filtrar una columna de fecha ISO
/// con su índice: `col >= inicio AND col < fin` equivale a `substr(col, 1, 7) = mes`.
/// Un mes inválido devuelve un rango vacío (no coincide con nada, como antes).
#[must_use]
pub fn rango_mes(mes: &str) -> (String, String) {
    let siguiente = mes
        .split_once('-')
        .filter(|(anio, numero)| {
            anio.len() == 4
                && numero.len() == 2
                && anio
                    .bytes()
                    .chain(numero.bytes())
                    .all(|b| b.is_ascii_digit())
        })
        .and_then(|(anio, numero)| {
            let anio: i32 = anio.parse().ok()?;
            match numero.parse::<u32>().ok()? {
                12 => Some(format!("{:04}-01", anio + 1)),
                n @ 1..=11 => Some(format!("{anio:04}-{:02}", n + 1)),
                _ => None,
            }
        });
    (
        mes.to_string(),
        siguiente.unwrap_or_else(|| mes.to_string()),
    )
}

/// Franja horaria codificada como en el legacy (`partido.dia`).
///
/// `1` madrugada, `2` mañana, `3` mediodía, `4` tarde, `5` atardecer,
/// `6` noche.
#[must_use]
pub fn franja_code(datetime: DateTime) -> i64 {
    match datetime.hour() {
        0..=5 => 1,
        6..=11 => 2,
        12..=14 => 3,
        15..=18 => 4,
        19..=20 => 5,
        _ => 6,
    }
}

/// Hora actual local como fecha-hora civil (sin zona horaria).
#[must_use]
pub fn now_local() -> DateTime {
    ZonedDateTime::now().datetime()
}

/// Fecha de hoy local.
#[must_use]
pub fn today() -> Date {
    ZonedDateTime::now().date()
}

/// Hora local actual con un formato `strftime` (sellos de archivo, informes…).
#[must_use]
pub fn ahora_fmt(formato: &str) -> String {
    ZonedDateTime::now().strftime(formato).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn rango_mes_cruza_el_anio() {
        let rango = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(rango_mes("2025-12"), rango("2025-12", "2026-01"));
        assert_eq!(rango_mes("2026-01"), rango("2026-01", "2026-02"));
        assert_eq!(rango_mes("2026-09"), rango("2026-09", "2026-10"));
        // Inválidos: rango vacío.
        for malo in ["2026-13", "2026-00", "2026-1", "abcd-ef", ""] {
            let (inicio, fin) = rango_mes(malo);
            assert_eq!(inicio, fin, "{malo}");
        }
    }

    #[test]
    fn parses_iso_and_local_dates() {
        let expected = date(2026, 11, 6);
        assert_eq!(parse_date("2026-11-06"), Some(expected));
        assert_eq!(parse_date("06/11/2026"), Some(expected));
        assert_eq!(parse_date("06-11-2026"), Some(expected));
        assert_eq!(parse_date("2026/11/06"), Some(expected));
        // Como chrono: día y mes de un dígito y espacios alrededor.
        assert_eq!(parse_date(" 6/11/2026 "), Some(expected));
        assert_eq!(parse_date("2026-1-5"), Some(date(2026, 1, 5)));
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(parse_date("2026-02-30"), None);
        assert_eq!(parse_date("2026-11-06 21:30"), None);
    }

    #[test]
    fn parses_legacy_datetimes() {
        let expected = date(2026, 11, 6).at(21, 30, 0, 0);
        assert_eq!(parse_datetime("2026-11-06 21:30"), Some(expected));
        assert_eq!(parse_datetime("2026-11-06 21:30:00"), Some(expected));
        assert_eq!(parse_datetime("2026-11-06T21:30"), Some(expected));
        assert_eq!(parse_datetime("2026-11-06T21:30:00"), Some(expected));
        assert_eq!(parse_datetime("06/11/2026 21:30"), Some(expected));
        assert_eq!(parse_datetime("06/11/2026 21:30:00"), Some(expected));
        let medianoche = date(2026, 11, 6).at(0, 0, 0, 0);
        assert_eq!(parse_datetime("2026-11-06"), Some(medianoche));
        assert_eq!(parse_datetime("06/11/2026"), Some(medianoche));
        assert_eq!(parse_datetime("06-11-2026"), Some(medianoche));
        assert_eq!(parse_datetime("2026-11-06 25:00"), None);
        assert_eq!(parse_datetime("  "), None);
    }

    #[test]
    fn parses_times() {
        assert_eq!(parse_time("21:30"), Some(jiff::civil::time(21, 30, 0, 0)));
        assert_eq!(parse_time("09:05:07"), Some(jiff::civil::time(9, 5, 7, 0)));
        assert_eq!(parse_time("9:05"), Some(jiff::civil::time(9, 5, 0, 0)));
        assert_eq!(parse_time("24:00"), None);
        assert_eq!(parse_time(""), None);
    }

    #[test]
    fn formats_iso_for_db() {
        let dt = date(2026, 1, 2).at(3, 4, 5, 0);
        assert_eq!(to_iso_datetime(dt), "2026-01-02 03:04:05");
        assert_eq!(to_iso_date(dt.date()), "2026-01-02");
        assert_eq!(display_datetime(dt), "02/01/2026 03:04");
        assert_eq!(display_date(dt.date()), "02/01/2026");
        // `Display` de jiff para fechas coincide con el ISO de la BD.
        assert_eq!(dt.date().to_string(), "2026-01-02");
    }

    #[test]
    fn ahora_fmt_usa_los_formatos_de_siempre() {
        let sello = ahora_fmt("%Y%m%d_%H%M%S");
        assert_eq!(sello.len(), 15);
        assert_eq!(sello.as_bytes()[8], b'_');
        let iso = ahora_fmt("%Y-%m-%dT%H:%M:%S");
        assert!(parse_datetime(&iso).is_some(), "{iso}");
        assert_eq!(ahora_fmt("%3f").len(), 3);
        assert_eq!(ahora_fmt("%Y-%m-%d"), to_iso_date(today()));
    }

    #[test]
    fn month_key_matches_legacy() {
        let dt = date(2026, 7, 15).at(0, 0, 0, 0);
        assert_eq!(month_key(dt), "2026-07");
        assert_eq!(month_key_from_date(date(987, 1, 1)), "0987-01");
    }

    #[test]
    fn aritmetica_de_fechas() {
        assert_eq!(fecha(2026, 2, 28), Some(date(2026, 2, 28)));
        assert_eq!(fecha(2026, 2, 29), None);
        assert_eq!(fecha(2026, 13, 1), None);
        assert_eq!(fecha(2026, 0, 1), None);
        assert_eq!(sumar_dias(date(2025, 12, 31), 1), date(2026, 1, 1));
        assert_eq!(sumar_dias(date(2026, 3, 1), -1), date(2026, 2, 28));
        assert_eq!(sumar_dias(date(2026, 1, 1), 7), date(2026, 1, 8));
        assert_eq!(
            sumar_dias_dt(date(2026, 3, 28).at(20, 0, 0, 0), 2),
            date(2026, 3, 30).at(20, 0, 0, 0)
        );
        assert_eq!(sumar_un_mes(date(2026, 1, 31)), date(2026, 2, 28));
        assert_eq!(sumar_un_mes(date(2028, 1, 31)), date(2028, 2, 29));
        assert_eq!(sumar_un_mes(date(2026, 12, 15)), date(2027, 1, 15));
        assert_eq!(dias_entre(date(2026, 1, 1), date(2026, 3, 1)), 59);
        assert_eq!(dias_entre(date(2026, 3, 1), date(2026, 1, 1)), -59);
        assert_eq!(dias_entre(date(2025, 12, 31), date(2026, 1, 1)), 1);
    }

    #[test]
    fn dia_semana_lunes_cero() {
        // 2026-11-23 es lunes; 2026-11-29 domingo.
        assert_eq!(dia_semana_desde_lunes(date(2026, 11, 23)), 0);
        assert_eq!(dia_semana_desde_lunes(date(2026, 11, 25)), 2);
        assert_eq!(dia_semana_desde_lunes(date(2026, 11, 29)), 6);
    }

    #[test]
    fn franja_boundaries() {
        let at = |h: i8| date(2026, 1, 1).at(h, 0, 0, 0);
        assert_eq!(franja_code(at(0)), 1);
        assert_eq!(franja_code(at(5)), 1);
        assert_eq!(franja_code(at(6)), 2);
        assert_eq!(franja_code(at(11)), 2);
        assert_eq!(franja_code(at(12)), 3);
        assert_eq!(franja_code(at(14)), 3);
        assert_eq!(franja_code(at(15)), 4);
        assert_eq!(franja_code(at(18)), 4);
        assert_eq!(franja_code(at(19)), 5);
        assert_eq!(franja_code(at(20)), 5);
        assert_eq!(franja_code(at(21)), 6);
        assert_eq!(franja_code(at(23)), 6);
    }
}

#[cfg(test)]
mod tiempo_tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn elapsed_text() {
        let base = date(2026, 1, 1).at(12, 0, 0, 0);
        assert_eq!(tiempo_transcurrido(base, base), "hace instantes");
        assert_eq!(
            tiempo_transcurrido(base, base + SignedDuration::from_mins(5)),
            "hace 5 min"
        );
        assert_eq!(
            tiempo_transcurrido(base, base + SignedDuration::from_hours(3)),
            "hace 3 h"
        );
        let texto = tiempo_transcurrido(base, sumar_dias_dt(base, 400));
        assert!(texto.contains("1 a"));
    }
}
