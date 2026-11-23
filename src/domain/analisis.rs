//! Análisis básico y evolución temporal (port de `analisis.c` de MiFutbolC).
//!
//! Cálculos puros sobre la serie cronológica de partidos.

/// Partido mínimo para el análisis.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartidoAnalisis {
    /// Id.
    pub id: i64,
    /// Fecha ISO `YYYY-MM-DD HH:MM:SS`.
    pub fecha_hora: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// 1 victoria, 2 empate, 3 derrota.
    pub resultado: i64,
    /// Rendimiento general (0–10).
    pub rendimiento: i64,
    /// Cansancio (0–10).
    pub cansancio: i64,
    /// Estado de ánimo (0–10).
    pub animo: i64,
}

/// Promedios de un grupo de partidos.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Promedios {
    /// Partidos del grupo.
    pub partidos: usize,
    /// Goles por partido.
    pub goles: f64,
    /// Asistencias por partido.
    pub asistencias: f64,
    /// Rendimiento medio.
    pub rendimiento: f64,
    /// Cansancio medio.
    pub cansancio: f64,
    /// Ánimo medio.
    pub animo: f64,
}

impl Promedios {
    /// Promedia los partidos dados (todo 0 si no hay).
    #[must_use]
    pub fn de<'a>(partidos: impl IntoIterator<Item = &'a PartidoAnalisis>) -> Self {
        let mut total = Self::default();
        for p in partidos {
            total.partidos += 1;
            total.goles += p.goles as f64;
            total.asistencias += p.asistencias as f64;
            total.rendimiento += p.rendimiento as f64;
            total.cansancio += p.cansancio as f64;
            total.animo += p.animo as f64;
        }
        if total.partidos > 0 {
            let n = total.partidos as f64;
            total.goles /= n;
            total.asistencias /= n;
            total.rendimiento /= n;
            total.cansancio /= n;
            total.animo /= n;
        }
        total
    }
}

/// Tendencia (umbral ±0,5 como en C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tendencia {
    /// Mejora.
    Ascendente,
    /// Empeora.
    Descendente,
    /// Sin cambios relevantes.
    Estable,
}

impl Tendencia {
    /// Clasifica una diferencia.
    #[must_use]
    pub fn de(diferencia: f64) -> Self {
        if diferencia > 0.5 {
            Self::Ascendente
        } else if diferencia < -0.5 {
            Self::Descendente
        } else {
            Self::Estable
        }
    }

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ascendente => "ASCENDENTE",
            Self::Descendente => "DESCENDENTE",
            Self::Estable => "ESTABLE",
        }
    }
}

/// Resultado de "Análisis básico" (`mostrar_analisis_basico`).
#[derive(Debug, Clone, PartialEq)]
pub struct AnalisisBasico {
    /// Últimos 5 partidos, el más reciente primero.
    pub ultimos: Vec<PartidoAnalisis>,
    /// Promedios de los últimos 5.
    pub ultimos5: Promedios,
    /// Promedios de todos.
    pub general: Promedios,
    /// Mejor racha de victorias.
    pub mejor_racha_victorias: usize,
    /// Peor racha de derrotas.
    pub peor_racha_derrotas: usize,
    /// Mensaje motivacional.
    pub mensaje: &'static str,
}

/// Mensaje motivacional de C: sube si goles y rendimiento suben más de 0,5;
/// baja si alguno cae más de 0,5.
#[must_use]
pub fn mensaje_motivacional(ultimos5: &Promedios, general: &Promedios) -> &'static str {
    let goles = ultimos5.goles - general.goles;
    let rendimiento = ultimos5.rendimiento - general.rendimiento;
    if goles > 0.5 && rendimiento > 0.5 {
        "¡Excelente! Estás en racha ascendente: tu esfuerzo está dando frutos. Mantené la consistencia."
    } else if goles < -0.5 || rendimiento < -0.5 {
        "No te desanimes: revisá tu preparación física y técnica, hablá con tu entrenador y recordá que el fútbol es perseverancia."
    } else {
        "Buen trabajo manteniendo el nivel. La consistencia es clave: cada partido es una oportunidad."
    }
}

/// Mejor racha de victorias y peor de derrotas (`actualizar_rachas`: el empate corta ambas).
#[must_use]
pub fn rachas(partidos: &[PartidoAnalisis]) -> (usize, usize) {
    let (mut v, mut d, mut max_v, mut max_d) = (0, 0, 0, 0);
    for p in partidos {
        match p.resultado {
            1 => {
                v += 1;
                d = 0;
            }
            3 => {
                d += 1;
                v = 0;
            }
            _ => (v, d) = (0, 0),
        }
        max_v = max_v.max(v);
        max_d = max_d.max(d);
    }
    (max_v, max_d)
}

/// Análisis básico; `None` sin partidos. `partidos` en orden cronológico.
#[must_use]
pub fn analisis_basico(partidos: &[PartidoAnalisis]) -> Option<AnalisisBasico> {
    if partidos.is_empty() {
        return None;
    }
    let ultimos: Vec<PartidoAnalisis> = partidos.iter().rev().take(5).cloned().collect();
    let ultimos5 = Promedios::de(&ultimos);
    let general = Promedios::de(partidos);
    let (mejor_racha_victorias, peor_racha_derrotas) = rachas(partidos);
    Some(AnalisisBasico {
        ultimos,
        ultimos5,
        general,
        mejor_racha_victorias,
        peor_racha_derrotas,
        mensaje: mensaje_motivacional(&ultimos5, &general),
    })
}

/// Métrica de la evolución mensual (opciones 1–3 de C).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Metrica {
    /// Goles por partido.
    #[default]
    Goles,
    /// Asistencias por partido.
    Asistencias,
    /// Rendimiento medio.
    Rendimiento,
}

impl Metrica {
    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Goles => "Goles",
            Self::Asistencias => "Asistencias",
            Self::Rendimiento => "Rendimiento",
        }
    }

    /// Valor de la métrica en unos promedios.
    #[must_use]
    pub const fn valor(self, promedios: &Promedios) -> f64 {
        match self {
            Self::Goles => promedios.goles,
            Self::Asistencias => promedios.asistencias,
            Self::Rendimiento => promedios.rendimiento,
        }
    }

    /// Siguiente métrica.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Goles => Self::Asistencias,
            Self::Asistencias => Self::Rendimiento,
            Self::Rendimiento => Self::Goles,
        }
    }
}

const MESES: [&str; 12] = [
    "Enero",
    "Febrero",
    "Marzo",
    "Abril",
    "Mayo",
    "Junio",
    "Julio",
    "Agosto",
    "Septiembre",
    "Octubre",
    "Noviembre",
    "Diciembre",
];

/// Promedios de un mes.
#[derive(Debug, Clone, PartialEq)]
pub struct MesEvolucion {
    /// Año.
    pub anio: i32,
    /// Mes 1–12.
    pub mes: u32,
    /// Promedios del mes.
    pub promedios: Promedios,
}

impl MesEvolucion {
    /// "Marzo 2026".
    #[must_use]
    pub fn etiqueta(&self) -> String {
        format!("{} {}", MESES[(self.mes as usize - 1) % 12], self.anio)
    }

    /// "03/26" para el gráfico.
    #[must_use]
    pub fn corta(&self) -> String {
        format!("{:02}/{:02}", self.mes, self.anio % 100)
    }
}

/// Progreso total del jugador (`calcular_progreso_total`).
#[derive(Debug, Clone, PartialEq)]
pub struct Progreso {
    /// Promedios de toda la carrera.
    pub general: Promedios,
    /// Primera fecha.
    pub desde: String,
    /// Última fecha.
    pub hasta: String,
    /// Rendimiento de los primeros 5 y de los últimos 5 (solo con 10 o más partidos).
    pub tendencia: Option<(f64, f64)>,
}

/// Evolución temporal: las 8 opciones del submenú de C.
#[derive(Debug, Clone, PartialEq)]
pub struct Evolucion {
    /// Meses, el más reciente primero (como C).
    pub meses: Vec<MesEvolucion>,
    /// Índice en `meses` del mejor mes por rendimiento.
    pub mejor_mes: usize,
    /// Índice en `meses` del peor mes por rendimiento.
    pub peor_mes: usize,
    /// Enero–junio.
    pub inicio_anio: Promedios,
    /// Julio–diciembre.
    pub fin_anio: Promedios,
    /// Junio–septiembre.
    pub frios: Promedios,
    /// Diciembre–abril.
    pub calidos: Promedios,
    /// Progreso total.
    pub progreso: Progreso,
}

fn anio_mes(fecha: &str) -> Option<(i32, u32)> {
    let anio = fecha.get(0..4)?.parse().ok()?;
    let mes: u32 = fecha.get(5..7)?.parse().ok()?;
    (1..=12).contains(&mes).then_some((anio, mes))
}

/// Evolución temporal; `None` sin partidos con fecha válida. `partidos` en orden cronológico.
#[must_use]
pub fn evolucion(partidos: &[PartidoAnalisis]) -> Option<Evolucion> {
    let fechados: Vec<(i32, u32, &PartidoAnalisis)> = partidos
        .iter()
        .filter_map(|p| anio_mes(&p.fecha_hora).map(|(a, m)| (a, m, p)))
        .collect();
    if fechados.is_empty() {
        return None;
    }
    let mut claves: Vec<(i32, u32)> = fechados.iter().map(|(a, m, _)| (*a, *m)).collect();
    claves.sort_unstable();
    claves.dedup();
    let meses: Vec<MesEvolucion> = claves
        .into_iter()
        .rev()
        .map(|(anio, mes)| MesEvolucion {
            anio,
            mes,
            promedios: Promedios::de(
                fechados
                    .iter()
                    .filter(|(a, m, _)| (*a, *m) == (anio, mes))
                    .map(|(_, _, p)| *p),
            ),
        })
        .collect();
    let por_rendimiento = |a: &(usize, &MesEvolucion), b: &(usize, &MesEvolucion)| {
        a.1.promedios
            .rendimiento
            .total_cmp(&b.1.promedios.rendimiento)
    };
    let mejor_mes = meses.iter().enumerate().max_by(por_rendimiento)?.0;
    let peor_mes = meses.iter().enumerate().min_by(por_rendimiento)?.0;
    let grupo = |incluye: &dyn Fn(u32) -> bool| {
        Promedios::de(
            fechados
                .iter()
                .filter(|(_, m, _)| incluye(*m))
                .map(|(_, _, p)| *p),
        )
    };
    let tendencia = (partidos.len() >= 10).then(|| {
        (
            Promedios::de(&partidos[..5]).rendimiento,
            Promedios::de(&partidos[partidos.len() - 5..]).rendimiento,
        )
    });
    Some(Evolucion {
        mejor_mes,
        peor_mes,
        inicio_anio: grupo(&|m| m <= 6),
        fin_anio: grupo(&|m| m > 6),
        frios: grupo(&|m| (6..=9).contains(&m)),
        calidos: grupo(&|m| m == 12 || m <= 4),
        progreso: Progreso {
            general: Promedios::de(partidos),
            desde: partidos.first().map(|p| p.fecha_hora.clone())?,
            hasta: partidos.last().map(|p| p.fecha_hora.clone())?,
            tendencia,
        },
        meses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partido(fecha: &str, goles: i64, resultado: i64, rendimiento: i64) -> PartidoAnalisis {
        PartidoAnalisis {
            fecha_hora: format!("{fecha} 20:00:00"),
            goles,
            resultado,
            rendimiento,
            ..PartidoAnalisis::default()
        }
    }

    #[test]
    fn rachas_cortan_con_empate() {
        let serie: Vec<_> = [1, 1, 1, 2, 1, 3, 3, 1]
            .iter()
            .map(|r| partido("2026-01-01", 0, *r, 5))
            .collect();
        assert_eq!(rachas(&serie), (3, 2));
    }

    #[test]
    fn basico_compara_ultimos_cinco() {
        assert!(analisis_basico(&[]).is_none());
        let mut serie: Vec<_> = (0..5).map(|_| partido("2026-01-01", 0, 3, 2)).collect();
        serie.extend((0..5).map(|_| partido("2026-02-01", 3, 1, 9)));
        let basico = analisis_basico(&serie).unwrap();
        assert_eq!(basico.ultimos.len(), 5);
        assert!((basico.ultimos5.goles - 3.0).abs() < 1e-9);
        assert!((basico.general.goles - 1.5).abs() < 1e-9);
        assert_eq!(
            (basico.mejor_racha_victorias, basico.peor_racha_derrotas),
            (5, 5)
        );
        assert!(basico.mensaje.starts_with("¡Excelente!"));
    }

    #[test]
    fn evolucion_agrupa_por_mes_y_periodos() {
        let mut serie = vec![
            partido("2025-12-10", 1, 1, 4),
            partido("2026-01-05", 2, 1, 8),
            partido("2026-01-20", 0, 3, 6),
            partido("2026-07-01", 4, 1, 2),
        ];
        serie.extend((0..6).map(|_| partido("2026-10-01", 1, 2, 5)));
        let evo = evolucion(&serie).unwrap();
        let etiquetas: Vec<_> = evo.meses.iter().map(MesEvolucion::etiqueta).collect();
        assert_eq!(
            etiquetas,
            ["Octubre 2026", "Julio 2026", "Enero 2026", "Diciembre 2025"]
        );
        assert!((evo.meses[2].promedios.goles - 1.0).abs() < 1e-9);
        assert_eq!(evo.meses[evo.mejor_mes].mes, 1);
        assert_eq!(evo.meses[evo.peor_mes].mes, 7);
        assert_eq!(evo.inicio_anio.partidos, 2);
        assert_eq!(evo.fin_anio.partidos, 8);
        assert_eq!(evo.frios.partidos, 1);
        assert_eq!(evo.calidos.partidos, 3);
        let (primeros, ultimos) = evo.progreso.tendencia.unwrap();
        assert!((primeros - 5.0).abs() < 1e-9 && (ultimos - 5.0).abs() < 1e-9);
        assert_eq!(Tendencia::de(ultimos - primeros), Tendencia::Estable);
        assert!(evolucion(&[partido("sin fecha", 0, 1, 1)]).is_none());
    }
}
