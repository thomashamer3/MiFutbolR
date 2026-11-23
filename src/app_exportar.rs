//! Ajustes → Exportar / Importar (Ctrl+D): los menús `menu_exportar` y
//! `menu_importar` de C en una lista. Las teclas Ctrl de Ajustes siguen
//! funcionando como atajos.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Modal, Screen, mover_seleccion};
use crate::services::informe_service;

/// Opciones en pantalla: (título, descripción).
pub const EXPORT_OPCIONES: [(&str, &str); 9] = [
    (
        "Exportar todo (JSON + Markdown)",
        "Todas las tablas a JSON y los listados a Markdown (Ctrl+X).",
    ),
    (
        "Exportar a XLSX",
        "Partidos, equipos, canchas, camisetas, lesiones y torneos.",
    ),
    (
        "Exportar a ODS",
        "Planilla OpenDocument con una hoja por tabla (Ctrl+O).",
    ),
    (
        "Informe PDF",
        "Resumen, partidos, lesiones, finanzas y más (Ctrl+P).",
    ),
    (
        "Copia de la base (.db)",
        "Copia completa de la base (Ctrl+E).",
    ),
    (
        "Exportar para MiFutbolMobile",
        "mifutbolmobile_import.json (Ctrl+W).",
    ),
    (
        "Importar base legacy (.db)",
        "Base de MiFutbolC sobre una base vacía (Ctrl+L).",
    ),
    (
        "Importar partidos del móvil",
        "Importaciones/partidos.json (Ctrl+Y).",
    ),
    (
        "Auto-importar del móvil al iniciar",
        "Activa o desactiva el marcador (Ctrl+A).",
    ),
];

impl App {
    /// Abre la lista de exportar / importar.
    pub(crate) fn open_exportar_importar(&mut self) {
        self.push_screen(Screen::ExportarImportar);
        self.status = format!(
            "Exporta a {} · importa desde {}",
            self.paths.export_dir.display(),
            self.paths.import_dir.display()
        );
    }

    /// Flechas + Enter o el número de la opción.
    pub(crate) fn handle_exportar_importar_key(&mut self, key: KeyEvent) -> Result<bool> {
        let actual = self.export_hub.selected();
        let opcion = match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.export_hub
                    .select(mover_seleccion(actual, EXPORT_OPCIONES.len(), 1));
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.export_hub
                    .select(mover_seleccion(actual, EXPORT_OPCIONES.len(), -1));
                return Ok(true);
            }
            KeyCode::Enter => actual.unwrap_or(0),
            KeyCode::Char(c @ '1'..='9') if (c as usize - '1' as usize) < EXPORT_OPCIONES.len() => {
                c as usize - '1' as usize
            }
            _ => return Ok(false),
        };
        self.export_hub.select(Some(opcion));
        self.ejecutar_opcion_exportar(opcion)?;
        Ok(true)
    }

    fn ejecutar_opcion_exportar(&mut self, opcion: usize) -> Result<()> {
        match opcion {
            0 => self.exportar_todo_ajustes(),
            1 => self.exportar_xlsx_ajustes(),
            2 => self.exportar_ods_ajustes(),
            3 => self.exportar_pdf_ajustes(),
            4 => self.exportar_db_ajustes()?,
            5 => self.exportar_mobile_ajustes(),
            6 => self.open_legacy_import_form()?,
            7 => self.importar_mobile_ajustes(),
            _ => self.alternar_mobile_auto_ajustes(),
        }
        Ok(())
    }

    /// Libro .xlsx con las hojas de `menu_exportar_xlsx` de C.
    pub(crate) fn exportar_xlsx_ajustes(&mut self) {
        match informe_service::exportar_xlsx(self.db.conn(), &self.paths.export_dir) {
            Ok(ruta) => {
                self.status = format!("Libro XLSX: {}", ruta.display());
                self.modal = Some(Modal::Info(format!(
                    "Exportado a XLSX:\n{}",
                    ruta.display()
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error.0)),
        }
    }
}
