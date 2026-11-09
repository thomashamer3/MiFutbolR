//! Referencias multimedia (IMP-21).

/// Tipo de referencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TipoMedia {
    /// Video.
    #[default]
    Video,
    /// Foto.
    Foto,
    /// Artículo.
    Articulo,
    /// Audio.
    Audio,
}

impl TipoMedia {
    /// Todos.
    pub const ALL: [Self; 4] = [Self::Video, Self::Foto, Self::Articulo, Self::Audio];

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Foto => "Foto",
            Self::Articulo => "Artículo",
            Self::Audio => "Audio",
        }
    }

    /// Código persistido.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Foto => "foto",
            Self::Articulo => "articulo",
            Self::Audio => "audio",
        }
    }

    /// Desde texto.
    #[must_use]
    pub fn from_codigo(texto: &str) -> Self {
        match texto {
            "foto" => Self::Foto,
            "articulo" => Self::Articulo,
            "audio" => Self::Audio,
            _ => Self::Video,
        }
    }
}

/// Referencia multimedia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaItem {
    /// Id.
    pub id: i64,
    /// Partido asociado.
    pub partido_id: Option<i64>,
    /// Tipo.
    pub tipo: TipoMedia,
    /// Título.
    pub titulo: String,
    /// URL.
    pub url: String,
    /// Descripción.
    pub descripcion: String,
    /// Fecha.
    pub fecha: String,
}
