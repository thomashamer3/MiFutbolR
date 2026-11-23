//! Clima en segundo plano: el bucle de eventos llama a `tick_clima` en cada vuelta.

use std::time::{Duration, Instant};

use crate::app::{App, Screen};
use crate::services::clima_service;

/// Cada cuánto se mira la cola cuando no hay consulta en curso.
const ESPERA_COLA: Duration = Duration::from_secs(2);

impl App {
    /// Aplica los resultados listos y lanza la siguiente consulta si el hilo está
    /// libre. Nunca bloquea. Devuelve `true` si cambió algo visible.
    pub fn tick_clima(&mut self) -> bool {
        let mut cambio = false;
        while let Some(respuesta) = self.clima.recibir() {
            match clima_service::procesar(self.db.conn_mut(), respuesta) {
                Ok(Some(mensaje)) => {
                    self.status = mensaje;
                    cambio = true;
                }
                Ok(None) => {}
                Err(error) => self.status = format!("Clima: {error}"),
            }
        }
        if cambio
            && matches!(self.current_screen(), Screen::PartidosList)
            && let Err(error) = self.refresh_partidos()
        {
            self.status = format!("No se pudieron recargar los partidos: {error}");
        }
        if !self.clima.ocupado() && self.clima_proxima_consulta <= Instant::now() {
            self.clima_proxima_consulta = Instant::now() + ESPERA_COLA;
            let siguiente = clima_service::pendientes(self.db.conn(), 50)
                .unwrap_or_default()
                .into_iter()
                .find(|pendiente| !self.clima.intentado(pendiente.partido_id));
            if let Some(pendiente) = siguiente {
                self.clima.enviar(pendiente);
            }
        }
        cambio
    }

    /// Encola todos los partidos sin clima (tecla en el listado de Partidos).
    pub(crate) fn encolar_clima_pendiente(&mut self) {
        match clima_service::encolar_sin_clima(self.db.conn()) {
            Ok(0) => {
                self.status =
                    "Sin partidos para completar: faltan coordenadas en la cancha o ya tienen clima"
                        .to_string();
            }
            Ok(cantidad) => {
                self.clima.olvidar_intentos();
                self.clima_proxima_consulta = Instant::now();
                self.status = format!("{cantidad} partido(s) en cola de clima (Open-Meteo)");
            }
            Err(error) => self.status = format!("Clima: {error}"),
        }
    }
}
