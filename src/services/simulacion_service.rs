//! "Simular con equipos guardados" (port de `simular_partido_guardados` de
//! `partido.c`): 60 minutos con eventos al azar; no se guarda en Partidos.

use rand::Rng;

use crate::domain::models::Equipo;
use crate::domain::reporte::Reporte;

/// Minutos simulados (C: 60).
pub const MINUTOS: u32 = 60;

/// Resultado de una simulación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Simulacion {
    /// Goles del local.
    pub goles_local: i64,
    /// Goles del visitante.
    pub goles_visitante: i64,
    /// Relato y estadísticas listos para mostrar.
    pub reporte: Reporte,
}

struct Lado<'a> {
    equipo: &'a Equipo,
    goles: i64,
    /// (goles, asistencias) por jugador, en el orden del plantel.
    stats: Vec<(i64, i64)>,
}

impl<'a> Lado<'a> {
    fn new(equipo: &'a Equipo) -> Self {
        Self {
            equipo,
            goles: 0,
            stats: vec![(0, 0); equipo.jugadores.len()],
        }
    }

    /// `manejar_gol` de C: el asistente nunca es el goleador si hay más de uno.
    fn gol<R: Rng + ?Sized>(&mut self, rng: &mut R, minuto: u32, reporte: &mut Reporte) {
        let total = self.equipo.jugadores.len();
        let autor = rng.gen_range(0..total);
        let mut asistente = rng.gen_range(0..total);
        if asistente == autor && total > 1 {
            asistente = (asistente + 1) % total;
        }
        self.goles += 1;
        self.stats[autor].0 += 1;
        let goleador = &self.equipo.jugadores[autor];
        let mut linea = format!(
            "Min {minuto:>2}: ¡GOL! {} (#{}) para {}",
            goleador.nombre, goleador.numero, self.equipo.nombre
        );
        if asistente != autor {
            self.stats[asistente].1 += 1;
            let pasador = &self.equipo.jugadores[asistente];
            linea.push_str(&format!(
                " · asistencia de {} (#{})",
                pasador.nombre, pasador.numero
            ));
        }
        reporte.linea(linea);
    }

    fn estadisticas(&self, reporte: &mut Reporte, titulo: &str) {
        reporte.seccion(&format!("{titulo} ({})", self.equipo.nombre));
        let mut alguno = false;
        for (jugador, (goles, asistencias)) in self.equipo.jugadores.iter().zip(&self.stats) {
            if *goles > 0 || *asistencias > 0 {
                alguno = true;
                reporte.linea(format!(
                    "  {} (#{}): {goles} goles, {asistencias} asistencias",
                    jugador.nombre, jugador.numero
                ));
            }
        }
        if !alguno {
            reporte.linea("  Sin goles ni asistencias");
        }
    }
}

/// Simula el partido. Error si algún equipo no tiene jugadores (C dividía por cero).
pub fn simular<R: Rng + ?Sized>(
    rng: &mut R,
    local: &Equipo,
    visitante: &Equipo,
) -> Result<Simulacion, String> {
    if local.id == visitante.id {
        return Err("El equipo visitante debe ser diferente al local".to_string());
    }
    for equipo in [local, visitante] {
        if equipo.jugadores.is_empty() {
            return Err(format!("El equipo «{}» no tiene jugadores", equipo.nombre));
        }
    }
    let mut reporte = Reporte::new(format!(
        "Simulación · {} vs {}",
        local.nombre, visitante.nombre
    ));
    reporte.seccion("Alineaciones");
    for (titulo, equipo) in [("Local", local), ("Visitante", visitante)] {
        let plantel: Vec<String> = equipo
            .jugadores
            .iter()
            .map(|j| format!("{} (#{})", j.nombre, j.numero))
            .collect();
        reporte.linea(format!(
            "{titulo} {}: {}",
            equipo.nombre,
            plantel.join(", ")
        ));
    }
    reporte.seccion("Relato");
    let mut a = Lado::new(local);
    let mut b = Lado::new(visitante);
    let mut oportunidades = 0;
    for minuto in 1..=MINUTOS {
        // C usaba 25 % de gol por minuto para cada lado (~30 goles por partido);
        // se baja a 3 % para un marcador creíble. El resto de la lógica es la de C.
        match rng.gen_range(0..100) {
            0..=2 => a.gol(rng, minuto, &mut reporte),
            3..=5 => b.gol(rng, minuto, &mut reporte),
            6..=25 => oportunidades += 1,
            _ => {}
        }
    }
    reporte.linea(format!("Oportunidades de gol: {oportunidades}"));
    reporte.seccion(&format!("Resultado final ({MINUTOS} minutos)"));
    reporte.linea(format!(
        "{} {} - {} {}",
        local.nombre, a.goles, b.goles, visitante.nombre
    ));
    reporte.linea(match a.goles.cmp(&b.goles) {
        std::cmp::Ordering::Greater => format!("¡{} gana el partido!", local.nombre),
        std::cmp::Ordering::Less => format!("¡{} gana el partido!", visitante.nombre),
        std::cmp::Ordering::Equal => "¡Empate!".to_string(),
    });
    a.estadisticas(&mut reporte, "Local");
    b.estadisticas(&mut reporte, "Visitante");
    reporte.linea("");
    reporte.linea("Simulación finalizada. Este partido no se guarda en Partidos.");
    Ok(Simulacion {
        goles_local: a.goles,
        goles_visitante: b.goles,
        reporte,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{Jugador, Posicion, TipoEquipo, TipoFutbol};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn equipo(id: i64, nombre: &str, jugadores: usize) -> Equipo {
        Equipo {
            id,
            nombre: nombre.to_string(),
            tipo: TipoEquipo::Fijo,
            tipo_futbol: TipoFutbol::F5,
            num_jugadores: 5,
            partido_id: None,
            activa: true,
            imagen_ruta: String::new(),
            jugadores: (0..jugadores)
                .map(|i| Jugador {
                    id: i as i64 + 1,
                    equipo_id: id,
                    nombre: format!("{nombre}{i}"),
                    numero: i as i64 + 1,
                    posicion: Posicion::Arquero,
                    es_capitan: false,
                    fecha_nacimiento: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn simula_con_marcador_coherente_y_rechaza_equipos_invalidos() {
        let (a, b) = (equipo(1, "Rojo", 5), equipo(2, "Azul", 1));
        let mut rng = StdRng::seed_from_u64(7);
        let mut total = 0;
        for _ in 0..50 {
            let sim = simular(&mut rng, &a, &b).unwrap();
            let goles_en_relato = sim
                .reporte
                .lineas
                .iter()
                .filter(|l| l.contains("¡GOL!"))
                .count() as i64;
            assert_eq!(goles_en_relato, sim.goles_local + sim.goles_visitante);
            // Un solo jugador: nunca se asiste a sí mismo.
            assert!(
                !sim.reporte
                    .lineas
                    .iter()
                    .any(|l| l.contains("asistencia de Azul0"))
            );
            total += goles_en_relato;
        }
        // ~3,6 goles por partido en promedio, nada de los ~30 de C.
        assert!((50..=400).contains(&total), "{total}");
        assert!(simular(&mut rng, &a, &a).is_err());
        assert!(simular(&mut rng, &a, &equipo(3, "Vacío", 0)).is_err());
    }
}
