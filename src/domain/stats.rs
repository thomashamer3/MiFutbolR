//! Cálculos estadísticos puros (sin SQL ni UI).
//!
//! El módulo se completa con las métricas del §14; por ahora expone las
//! primitivas compartidas: promedios, rachas y diferencias de gol.

/// Promedio de una serie de enteros; `None` si la serie está vacía.
#[must_use]
pub fn average(values: &[i64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: i64 = values.iter().sum();
    // El cast es seguro para conteos realistas de partidos.
    #[allow(clippy::cast_precision_loss)]
    Some(sum as f64 / values.len() as f64)
}

/// Porcentaje `parte / total` redondeado a un decimal.
#[must_use]
pub fn percentage(parte: i64, total: i64) -> f64 {
    if total <= 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    {
        (parte as f64) * 100.0 / (total as f64)
    }
}

/// Diferencia de gol a partir de los marcadores; `None` si no se cargaron
/// (-1 en cualquiera de los dos).
#[must_use]
pub fn diferencia_gol(goles_equipo: i64, goles_rival: i64) -> Option<i64> {
    if goles_equipo < 0 || goles_rival < 0 {
        None
    } else {
        Some(goles_equipo - goles_rival)
    }
}

/// Resultado legible de un marcador (`1` victoria, `2` empate, `3` derrota).
#[must_use]
pub fn resultado_de_marcador(goles_equipo: i64, goles_rival: i64) -> i64 {
    match goles_equipo.cmp(&goles_rival) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 2,
        std::cmp::Ordering::Less => 3,
    }
}

/// Cuenta la racha actual desde el final de la serie.
///
/// `-1` el partido más reciente y `1` hacia atrás. Devuelve `(tipo, largo)`
/// donde tipo `1` = victorias, `2` = empates, `3` = derrotas.
#[must_use]
pub fn racha_actual(resultados: &[i64]) -> Option<(i64, usize)> {
    let first = *resultados.last()?;
    let len = resultados.iter().rev().take_while(|r| **r == first).count();
    Some((first, len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_handles_empty_and_values() {
        assert_eq!(average(&[]), None);
        assert_eq!(average(&[2, 4]), Some(3.0));
    }

    #[test]
    fn percentage_and_goal_difference() {
        assert_eq!(percentage(1, 0), 0.0);
        assert_eq!(percentage(1, 4), 25.0);
        assert_eq!(diferencia_gol(3, 1), Some(2));
        assert_eq!(diferencia_gol(-1, 1), None);
        assert_eq!(resultado_de_marcador(2, 2), 2);
    }

    #[test]
    fn current_streak_counts_from_the_end() {
        assert_eq!(racha_actual(&[1, 1, 3, 3, 3]), Some((3, 3)));
        assert_eq!(racha_actual(&[]), None);
    }
}
