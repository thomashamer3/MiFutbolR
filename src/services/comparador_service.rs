//! Comparador A/B (IMP-14).

use rusqlite::Connection;

use crate::db::comparador_repo;
use crate::db::error::DbError;
use crate::domain::comparador::{
    AgregadoComparacion, Comparacion, FilaComparacion, Lado, OpcionComparador, TipoComparacion,
};

/// Opciones del tipo activo.
pub fn opciones(
    conn: &Connection,
    tipo: TipoComparacion,
) -> Result<Vec<OpcionComparador>, DbError> {
    match tipo {
        TipoComparacion::Camiseta => comparador_repo::camisetas(conn),
        TipoComparacion::Cancha => comparador_repo::canchas(conn),
    }
}

/// Compara dos entidades y arma las filas.
pub fn comparar(
    conn: &Connection,
    tipo: TipoComparacion,
    a: i64,
    b: i64,
) -> Result<Comparacion, DbError> {
    let (agregado_a, agregado_b) = match tipo {
        TipoComparacion::Camiseta => (
            comparador_repo::agregado_camiseta(conn, a)?,
            comparador_repo::agregado_camiseta(conn, b)?,
        ),
        TipoComparacion::Cancha => (
            comparador_repo::agregado_cancha(conn, a)?,
            comparador_repo::agregado_cancha(conn, b)?,
        ),
    };
    Ok(armar(agregado_a, agregado_b))
}

/// Arma la comparación (pura, testeable).
#[must_use]
pub fn armar(a: AgregadoComparacion, b: AgregadoComparacion) -> Comparacion {
    let mut filas = Vec::new();
    fila_entera(&mut filas, "Partidos", a.partidos, b.partidos);
    fila_entera(&mut filas, "Goles", a.goles, b.goles);
    fila_entera(&mut filas, "Asistencias", a.asistencias, b.asistencias);
    fila_entera(&mut filas, "Victorias", a.victorias, b.victorias);
    fila_entera(&mut filas, "Empates", a.empates, b.empates);
    fila_entera(&mut filas, "Derrotas", a.derrotas, b.derrotas);
    fila_decimal(&mut filas, "Winrate %", winrate(&a), winrate(&b));
    fila_decimal(&mut filas, "Rendimiento", a.rendimiento, b.rendimiento);
    fila_decimal(
        &mut filas,
        "Goles + asistencias / partido",
        aporte(&a),
        aporte(&b),
    );
    Comparacion {
        titulo_a: a.nombre,
        titulo_b: b.nombre,
        filas,
    }
}

fn winrate(agregado: &AgregadoComparacion) -> f64 {
    if agregado.partidos == 0 {
        0.0
    } else {
        agregado.victorias as f64 * 100.0 / agregado.partidos as f64
    }
}

fn aporte(agregado: &AgregadoComparacion) -> f64 {
    if agregado.partidos == 0 {
        0.0
    } else {
        (agregado.goles + agregado.asistencias) as f64 / agregado.partidos as f64
    }
}

fn fila_entera(filas: &mut Vec<FilaComparacion>, metrica: &'static str, a: i64, b: i64) {
    filas.push(FilaComparacion {
        metrica,
        a: a.to_string(),
        b: b.to_string(),
        mejor: if a > b {
            Lado::A
        } else if b > a {
            Lado::B
        } else {
            Lado::Empate
        },
    });
}

fn fila_decimal(filas: &mut Vec<FilaComparacion>, metrica: &'static str, a: f64, b: f64) {
    filas.push(FilaComparacion {
        metrica,
        a: format!("{a:.2}"),
        b: format!("{b:.2}"),
        mejor: if (a - b).abs() < f64::EPSILON {
            Lado::Empate
        } else if a > b {
            Lado::A
        } else {
            Lado::B
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agregado(nombre: &str, partidos: i64, goles: i64, victorias: i64) -> AgregadoComparacion {
        AgregadoComparacion {
            nombre: nombre.to_string(),
            partidos,
            goles,
            victorias,
            ..AgregadoComparacion::default()
        }
    }

    #[test]
    fn compares_entities() {
        let comparacion = armar(agregado("A", 10, 20, 6), agregado("B", 8, 10, 2));
        assert_eq!(comparacion.titulo_a, "A");
        assert_eq!(comparacion.filas.len(), 9);
        let goles = comparacion
            .filas
            .iter()
            .find(|fila| fila.metrica == "Goles")
            .unwrap();
        assert_eq!(goles.mejor, Lado::A);
        let partidos = comparacion
            .filas
            .iter()
            .find(|fila| fila.metrica == "Partidos")
            .unwrap();
        assert_eq!(partidos.a, "10");
    }
}
