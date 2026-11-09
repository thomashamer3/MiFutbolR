//! Previsualización de imágenes con ratatui-image (protocolo halfblocks).
//!
//! Se usa `Picker::halfblocks`, que funciona en cualquier terminal y también
//! bajo `TestBackend`; no requiere consultar el terminal al arrancar.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::{Rect, Size};
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui_image::protocol::Protocol;
use ratatui_image::{Image, Resize, picker::Picker};

use crate::ui::theme::Palette;

/// Logo incrustado (PNG de 256 px) para el menú principal.
static LOGO: &[u8] = include_bytes!("../../images/icon-256x256.png");

/// Tamaño de celda objetivo para las miniaturas.
const MINIATURA: Size = Size::new(28, 10);

/// Caché de protocolos de imagen ya decodificados.
pub struct ImageCache {
    picker: Picker,
    cache: HashMap<(PathBuf, u16, u16), Option<Protocol>>,
    logo: HashMap<(u16, u16), Option<Protocol>>,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            picker: Picker::halfblocks(),
            cache: HashMap::new(),
            logo: HashMap::new(),
        }
    }
}

impl ImageCache {
    /// Resuelve la ruta de una imagen legacy relativa al directorio de datos.
    #[must_use]
    pub fn resolver(ruta: &str, imagenes_dir: &Path, data_dir: &Path) -> Option<PathBuf> {
        let original = Path::new(ruta);
        if original.as_os_str().is_empty() {
            return None;
        }
        if original.is_file() {
            return Some(original.to_path_buf());
        }
        let candidatos = [
            data_dir.join(original),
            imagenes_dir.join(original.file_name()?),
        ];
        candidatos.into_iter().find(|candidato| candidato.is_file())
    }

    /// Devuelve el protocolo de una imagen, cargándola la primera vez.
    pub fn protocolo(&mut self, ruta: &Path) -> Option<&Protocol> {
        self.protocolo_tamano(ruta, MINIATURA)
    }

    /// Devuelve el protocolo de una imagen para un tamaño de celdas concreto.
    pub fn protocolo_tamano(&mut self, ruta: &Path, tamano: Size) -> Option<&Protocol> {
        let clave = (ruta.to_path_buf(), tamano.width, tamano.height);
        if !self.cache.contains_key(&clave) {
            let protocolo = self.cargar(ruta, tamano);
            self.cache.insert(clave.clone(), protocolo);
        }
        self.cache.get(&clave).and_then(Option::as_ref)
    }

    fn cargar(&mut self, ruta: &Path, tamano: Size) -> Option<Protocol> {
        let imagen = image::ImageReader::open(ruta).ok()?.decode().ok()?;
        self.picker
            .new_protocol(imagen, tamano, Resize::Fit(None))
            .ok()
    }

    /// Limpia la caché (por ejemplo al cambiar de tema o recargar datos).
    pub fn limpiar(&mut self) {
        self.cache.clear();
        self.logo.clear();
    }

    /// Logo de la aplicación embebido en el binario.
    pub fn logo(&mut self, tamano: Size) -> Option<&Protocol> {
        let clave = (tamano.width, tamano.height);
        if !self.logo.contains_key(&clave) {
            let protocolo = image::load_from_memory(LOGO).ok().and_then(|imagen| {
                self.picker
                    .new_protocol(imagen, tamano, Resize::Fit(None))
                    .ok()
            });
            self.logo.insert(clave, protocolo);
        }
        self.logo.get(&clave).and_then(Option::as_ref)
    }
}

/// Dibuja un panel con la imagen ajustada al área disponible.
pub fn dibujar(
    frame: &mut Frame,
    area: Rect,
    cache: &mut ImageCache,
    ruta: &Path,
    colors: &Palette,
) {
    let bloque = Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Imagen ")
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    if let Some(protocolo) = cache.protocolo(ruta) {
        frame.render_widget(Image::new(protocolo), interior);
    }
}

/// Dibuja la imagen en modo visor, ocupando todo el panel disponible.
pub fn dibujar_grande(
    frame: &mut Frame,
    area: Rect,
    cache: &mut ImageCache,
    ruta: &Path,
    colors: &Palette,
) {
    let bloque = Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Imagen · Esc para volver ")
        .border_style(Style::default().fg(colors.accent))
        .style(Style::default().bg(colors.bg));
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let tamano = Size::new(interior.width.max(1), interior.height.max(1));
    if let Some(protocolo) = cache.protocolo_tamano(ruta, tamano) {
        let tam = protocolo.size();
        let ancho = tam.width.min(interior.width);
        let alto = tam.height.min(interior.height);
        let destino = Rect {
            x: interior.x + (interior.width.saturating_sub(ancho)) / 2,
            y: interior.y + (interior.height.saturating_sub(alto)) / 2,
            width: ancho,
            height: alto,
        };
        frame.render_widget(Image::new(protocolo), destino);
    }
}

/// Dibuja una miniatura sin borde, centrada.
pub fn dibujar_miniatura(
    frame: &mut Frame,
    area: Rect,
    cache: &mut ImageCache,
    ruta: &Path,
    _colors: &Palette,
) {
    if area.width < 8 || area.height < 4 {
        return;
    }
    let objetivo = Size::new(area.width.min(24), area.height.min(8));
    if let Some(protocolo) = cache.protocolo_tamano(ruta, objetivo) {
        let tam = protocolo.size();
        let ancho = tam.width.min(area.width);
        let alto = tam.height.min(area.height);
        let destino = Rect {
            x: area.x + (area.width.saturating_sub(ancho)) / 2,
            y: area.y + (area.height.saturating_sub(alto)) / 2,
            width: ancho,
            height: alto,
        };
        frame.render_widget(Image::new(protocolo), destino);
    }
}

/// Dibuja el logo embebido centrado, sin borde.
pub fn dibujar_logo(frame: &mut Frame, area: Rect, cache: &mut ImageCache, colors: &Palette) {
    if area.width < 10 || area.height < 4 {
        return;
    }
    let objetivo = Size::new(area.width.min(22), area.height.min(8));
    if let Some(protocolo) = cache.logo(objetivo) {
        let tam = protocolo.size();
        let ancho = tam.width.min(area.width);
        let alto = tam.height.min(area.height);
        let destino = Rect {
            x: area.x + (area.width.saturating_sub(ancho)) / 2,
            y: area.y + (area.height.saturating_sub(alto)) / 2,
            width: ancho,
            height: alto,
        };
        frame.render_widget(Image::new(protocolo), destino);
        let _ = colors;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_icon_as_halfblocks() {
        let mut cache = ImageCache::default();
        let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icono.png");
        assert!(ruta.is_file());
        assert!(cache.protocolo(&ruta).is_some());
        // La segunda llamada usa la caché.
        assert!(cache.protocolo(&ruta).is_some());
    }

    #[test]
    fn resolver_finds_relative_image() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"));
        let absoluta = base.join("assets/icono.png");
        let encontrada = ImageCache::resolver(absoluta.to_str().unwrap(), base, base);
        assert_eq!(encontrada, Some(absoluta));
        assert!(ImageCache::resolver("", base, base).is_none());
    }
}
