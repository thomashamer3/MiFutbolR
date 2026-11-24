//! Reporte de texto de solo lectura: las pantallas de C que solo imprimen
//! (diferencias de lesiones, simulación, comparaciones, tablas) se arman como
//! líneas y se muestran con la misma pantalla `Screen::Reporte`.

/// Reporte listo para mostrar o exportar a TXT.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reporte {
    /// Título de la pantalla.
    pub titulo: String,
    /// Líneas; las que empiezan con `== ` se dibujan como encabezado.
    pub lineas: Vec<String>,
    /// Barras opcionales `(etiqueta, valor)` dibujadas sobre el texto.
    pub barras: Vec<(String, u64)>,
}

impl Reporte {
    /// Reporte vacío con título.
    #[must_use]
    pub fn new(titulo: impl Into<String>) -> Self {
        Self {
            titulo: titulo.into(),
            ..Self::default()
        }
    }

    /// Agrega una línea.
    pub fn linea(&mut self, texto: impl Into<String>) {
        self.lineas.push(texto.into());
    }

    /// Agrega un encabezado de sección (con una línea en blanco antes si hace falta).
    pub fn seccion(&mut self, nombre: &str) {
        if !self.lineas.is_empty() {
            self.lineas.push(String::new());
        }
        self.lineas.push(format!("== {nombre}"));
    }

    /// Texto plano para exportar.
    #[must_use]
    pub fn texto(&self) -> String {
        let mut salida = format!(
            "{}\n{}\n",
            self.titulo,
            "=".repeat(self.titulo.chars().count())
        );
        for linea in &self.lineas {
            salida.push_str(linea.strip_prefix("== ").unwrap_or(linea));
            salida.push('\n');
        }
        salida
    }

    /// HTML simple: secciones como `<h2>` y cada línea como párrafo.
    #[must_use]
    pub fn html(&self) -> String {
        let escapar = |texto: &str| {
            texto
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        };
        let mut salida = format!(
            "<!doctype html><meta charset=\"utf-8\"><title>{0}</title><h1>{0}</h1>\n",
            escapar(&self.titulo)
        );
        for linea in &self.lineas {
            match linea.strip_prefix("== ") {
                Some(seccion) => salida.push_str(&format!("<h2>{}</h2>\n", escapar(seccion))),
                None if linea.is_empty() => {}
                None => salida.push_str(&format!("<p>{}</p>\n", escapar(linea))),
            }
        }
        salida
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texto_quita_marcas_de_seccion() {
        let mut r = Reporte::new("Prueba");
        r.seccion("Uno");
        r.linea("a");
        r.seccion("Dos");
        assert_eq!(r.lineas, ["== Uno", "a", "", "== Dos"]);
        assert_eq!(r.texto(), "Prueba\n======\nUno\na\n\nDos\n");
    }
}
