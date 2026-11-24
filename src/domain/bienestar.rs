//! Bienestar (paridad con MiFutbolC): hábitos, entrenamientos y ejercicios,
//! comidas y día nutricional, sesiones mentales, salud (perfil, controles y
//! estudios), objetivos y planes, imágenes de menú y asistente de preparación.
//!
//! Las secciones son tablas planas; se describen con columnas para compartir
//! un único CRUD, formulario y tabla.

/// Tipo de dato de una columna.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TipoDato {
    /// Fecha ISO `YYYY-MM-DD` (en pantalla `DD/MM/AAAA`; vacío = hoy).
    Fecha,
    /// Entero en rango.
    Entero(i64, i64),
    /// Decimal opcional en rango.
    Decimal(f64, f64),
    /// Booleano guardado como 0/1.
    Booleano,
    /// Una de las opciones (se guarda el texto, como MiFutbolC).
    Opcion(&'static [&'static str]),
    /// Texto libre con longitud máxima.
    Texto(usize),
    /// Id de un registro de otra sección (se elige de una lista).
    Referencia(Referencia),
    /// Ruta de un archivo; al guardar se copia a `Imagenes/` como MiFutbolC.
    Archivo,
}

/// Tabla a la que apunta una [`TipoDato::Referencia`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Referencia {
    /// `bienestar_objetivo`.
    Objetivo,
    /// `bienestar_entrenamiento`.
    Entrenamiento,
    /// `bienestar_ejercicio`.
    Ejercicio,
    /// `bienestar_control_medico`.
    Control,
}

impl Referencia {
    /// Tabla referenciada.
    #[must_use]
    pub const fn tabla(self) -> &'static str {
        match self {
            Self::Objetivo => "bienestar_objetivo",
            Self::Entrenamiento => "bienestar_entrenamiento",
            Self::Ejercicio => "bienestar_ejercicio",
            Self::Control => "bienestar_control_medico",
        }
    }

    /// Expresión SQL (sobre la tabla referenciada) con la etiqueta visible.
    #[must_use]
    pub const fn etiqueta_sql(self) -> &'static str {
        match self {
            Self::Objetivo => "nombre",
            Self::Entrenamiento | Self::Control => "fecha || ' · ' || tipo",
            Self::Ejercicio => {
                "nombre || CASE WHEN grupo_muscular <> '' THEN ' (' || grupo_muscular || ')' ELSE '' END"
            }
        }
    }
}

/// Columna editable de una sección.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Columna {
    /// Nombre de la columna SQL (lista cerrada: nunca viene del usuario).
    pub clave: &'static str,
    /// Etiqueta visible.
    pub etiqueta: &'static str,
    /// Tipo.
    pub tipo: TipoDato,
}

const fn col(clave: &'static str, etiqueta: &'static str, tipo: TipoDato) -> Columna {
    Columna {
        clave,
        etiqueta,
        tipo,
    }
}

const ESCALA: TipoDato = TipoDato::Entero(0, 10);

/// Tipos de entrenamiento (valores de MiFutbolC).
pub const TIPOS_ENTRENAMIENTO: &[&str] = &["Fuerza", "Resistencia", "Tecnica", "Recuperacion"];
/// Tipos de comida.
pub const TIPOS_COMIDA: &[&str] = &["Desayuno", "Almuerzo", "Cena", "Snack"];
/// Calidad de comida.
pub const CALIDADES: &[&str] = &["Mala", "Regular", "Buena"];
/// Tipos de sesión mental.
pub const TIPOS_MENTAL: &[&str] = &["Charla", "Visualizacion", "Reflexion"];
/// Momento de la sesión mental respecto de un partido.
pub const MOMENTOS: &[&str] = &["N/A", "Antes", "Despues"];
/// Estados de un objetivo.
pub const ESTADOS_OBJETIVO: &[&str] = &["Activo", "Logrado", "Abandonado"];
/// Hidratación del día nutricional.
pub const HIDRATACIONES: &[&str] = &["Baja", "Media", "Alta"];
/// Menús con imagen propia (claves de MiFutbolC).
pub const MENUS_IMAGEN: &[&str] = &["entrenamiento", "alimentacion", "salud"];

/// Sección del módulo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seccion {
    /// Mentalidad y hábitos diarios.
    Habitos,
    /// Entrenamientos.
    Entrenamientos,
    /// Catálogo de ejercicios.
    Ejercicios,
    /// Ejercicios asociados a un entrenamiento (series/repeticiones).
    Series,
    /// Comidas.
    Comidas,
    /// Día nutricional (uno por fecha).
    Nutricion,
    /// Sesiones mentales.
    Mental,
    /// Perfil de salud (una sola fila).
    Salud,
    /// Controles médicos.
    Controles,
    /// Archivos de estudios adjuntos a un control.
    Estudios,
    /// Objetivos personales.
    Objetivos,
    /// Planes de entrenamiento por objetivo.
    Planes,
    /// Imagen de cada menú (entrenamiento, alimentación, salud).
    Imagenes,
}

impl Seccion {
    /// Orden de las pestañas.
    pub const ALL: [Self; 13] = [
        Self::Habitos,
        Self::Entrenamientos,
        Self::Ejercicios,
        Self::Series,
        Self::Comidas,
        Self::Nutricion,
        Self::Mental,
        Self::Salud,
        Self::Controles,
        Self::Estudios,
        Self::Objetivos,
        Self::Planes,
        Self::Imagenes,
    ];

    /// Etiqueta de la pestaña.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Habitos => "Hábitos",
            Self::Entrenamientos => "Entrenamientos",
            Self::Ejercicios => "Ejercicios",
            Self::Series => "Series",
            Self::Comidas => "Comidas",
            Self::Nutricion => "Nutrición",
            Self::Mental => "Mental",
            Self::Salud => "Salud",
            Self::Controles => "Controles",
            Self::Estudios => "Estudios",
            Self::Objetivos => "Objetivos",
            Self::Planes => "Planes",
            Self::Imagenes => "Imágenes",
        }
    }

    /// Tabla SQL.
    #[must_use]
    pub const fn tabla(self) -> &'static str {
        match self {
            Self::Habitos => "bienestar_habito",
            Self::Entrenamientos => "bienestar_entrenamiento",
            Self::Ejercicios => "bienestar_ejercicio",
            Self::Series => "bienestar_entrenamiento_ejercicio",
            Self::Comidas => "bienestar_comida",
            Self::Nutricion => "bienestar_dia_nutricional",
            Self::Mental => "bienestar_sesion_mental",
            Self::Salud => "bienestar_salud",
            Self::Controles => "bienestar_control_medico",
            Self::Estudios => "bienestar_estudio_archivo",
            Self::Objetivos => "bienestar_objetivo",
            Self::Planes => "bienestar_plan_entrenamiento",
            Self::Imagenes => "bienestar_menu_imagen",
        }
    }

    /// Columna que identifica la fila (`bienestar_menu_imagen` no tiene `id`).
    #[must_use]
    pub const fn clave(self) -> &'static str {
        match self {
            Self::Imagenes => "rowid",
            _ => "id",
        }
    }

    /// Orden del listado.
    #[must_use]
    pub const fn orden(self) -> &'static str {
        match self {
            Self::Salud => "id",
            Self::Ejercicios | Self::Series | Self::Objetivos | Self::Planes => "id DESC",
            Self::Estudios => "fecha_subida DESC, id DESC",
            Self::Imagenes => "menu_key",
            _ => "fecha DESC, id DESC",
        }
    }

    /// Columna única: guardar un valor repetido actualiza esa fila (upsert de MiFutbolC).
    #[must_use]
    pub const fn unica(self) -> Option<&'static str> {
        match self {
            Self::Nutricion => Some("fecha"),
            Self::Imagenes => Some("menu_key"),
            _ => None,
        }
    }

    /// Columnas editables, en orden de formulario y tabla.
    #[must_use]
    pub const fn columnas(self) -> &'static [Columna] {
        match self {
            Self::Habitos => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("dormi_bien", "Dormí bien", TipoDato::Booleano),
                    col("hidratacion", "Hidratación (0-10)", ESCALA),
                    col("alcohol", "Alcohol", TipoDato::Booleano),
                    col("estado_animico", "Estado anímico", TipoDato::Texto(40)),
                    col("nervios", "Nervios (0-10)", ESCALA),
                    col("confianza", "Confianza (0-10)", ESCALA),
                    col("motivacion", "Motivación (0-10)", ESCALA),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Entrenamientos => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("tipo", "Tipo", TipoDato::Opcion(TIPOS_ENTRENAMIENTO)),
                    col("duracion_min", "Duración (min)", TipoDato::Entero(1, 600)),
                    col("intensidad", "Intensidad (1-10)", TipoDato::Entero(1, 10)),
                    col("omitido", "Omitido", TipoDato::Booleano),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Comidas => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("tipo", "Tipo", TipoDato::Opcion(TIPOS_COMIDA)),
                    col("calidad", "Calidad", TipoDato::Opcion(CALIDADES)),
                    col("descripcion", "Descripción", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Mental => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("tipo", "Tipo", TipoDato::Opcion(TIPOS_MENTAL)),
                    col("momento", "Momento", TipoDato::Opcion(MOMENTOS)),
                    col("confianza", "Confianza (0-10)", ESCALA),
                    col("estres", "Estrés (0-10)", ESCALA),
                    col("motivacion", "Motivación (0-10)", ESCALA),
                    col("presion", "Presión (0-10)", ESCALA),
                    col("concentracion", "Concentración (0-10)", ESCALA),
                    col(
                        "pensamientos_clave",
                        "Pensamientos clave",
                        TipoDato::Texto(240),
                    ),
                    col("texto_libre", "Texto libre", TipoDato::Texto(400)),
                ];
                C
            }
            Self::Salud => {
                const C: &[Columna] = &[
                    col("altura_cm", "Altura (cm)", TipoDato::Decimal(0.0, 300.0)),
                    col("peso_kg", "Peso (kg)", TipoDato::Decimal(0.0, 500.0)),
                    col("tipo_sangre", "Tipo de sangre", TipoDato::Texto(5)),
                    col("ultima_revision", "Última revisión", TipoDato::Fecha),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Ejercicios => {
                const C: &[Columna] = &[
                    col("nombre", "Ejercicio", TipoDato::Texto(120)),
                    col("grupo_muscular", "Grupo muscular", TipoDato::Texto(120)),
                ];
                C
            }
            Self::Series => {
                const C: &[Columna] = &[
                    col(
                        "entrenamiento_id",
                        "Entrenamiento",
                        TipoDato::Referencia(Referencia::Entrenamiento),
                    ),
                    col(
                        "ejercicio_id",
                        "Ejercicio",
                        TipoDato::Referencia(Referencia::Ejercicio),
                    ),
                    col("series", "Series (0-100)", TipoDato::Entero(0, 100)),
                    col(
                        "repeticiones",
                        "Repeticiones (0-500)",
                        TipoDato::Entero(0, 500),
                    ),
                    col("tiempo_min", "Tiempo (min)", TipoDato::Entero(0, 600)),
                ];
                C
            }
            Self::Nutricion => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col(
                        "hidratacion",
                        "Hidratación",
                        TipoDato::Opcion(HIDRATACIONES),
                    ),
                    col("alcohol", "Alcohol", TipoDato::Booleano),
                    col(
                        "peso_corporal",
                        "Peso corporal (kg)",
                        TipoDato::Decimal(0.0, 500.0),
                    ),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Controles => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("tipo", "Tipo", TipoDato::Texto(60)),
                    col("profesional", "Profesional", TipoDato::Texto(60)),
                    col("resultado", "Resultado", TipoDato::Texto(120)),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Estudios => {
                const C: &[Columna] = &[
                    col(
                        "control_id",
                        "Control médico",
                        TipoDato::Referencia(Referencia::Control),
                    ),
                    col("nombre_original", "Nombre", TipoDato::Texto(255)),
                    col("tipo_archivo", "Tipo", TipoDato::Texto(20)),
                    col("fecha_subida", "Fecha", TipoDato::Fecha),
                    col("ruta_archivo", "Archivo", TipoDato::Archivo),
                ];
                C
            }
            Self::Objetivos => {
                const C: &[Columna] = &[
                    col("nombre", "Objetivo", TipoDato::Texto(120)),
                    col("fecha_inicio", "Inicio", TipoDato::Fecha),
                    col("fecha_fin", "Fin", TipoDato::Fecha),
                    col("estado", "Estado", TipoDato::Opcion(ESTADOS_OBJETIVO)),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Planes => {
                const C: &[Columna] = &[
                    col(
                        "objetivo_id",
                        "Objetivo",
                        TipoDato::Referencia(Referencia::Objetivo),
                    ),
                    col(
                        "frecuencia_semanal",
                        "Frecuencia semanal (1-14)",
                        TipoDato::Entero(1, 14),
                    ),
                    col("rutina_semanal", "Rutina semanal", TipoDato::Texto(250)),
                    col("notas", "Notas", TipoDato::Texto(250)),
                ];
                C
            }
            Self::Imagenes => {
                const C: &[Columna] = &[
                    col("menu_key", "Menú", TipoDato::Opcion(MENUS_IMAGEN)),
                    col("imagen_ruta", "Imagen", TipoDato::Archivo),
                ];
                C
            }
        }
    }

    /// Cantidad de columnas que entran en la tabla del listado.
    #[must_use]
    pub const fn columnas_listado(self) -> usize {
        match self {
            Self::Habitos | Self::Mental => 8,
            Self::Imagenes | Self::Ejercicios => 2,
            Self::Comidas | Self::Planes => 4,
            _ => 5,
        }
    }

    /// Índice de la columna de archivo, si la sección adjunta archivos.
    #[must_use]
    pub fn columna_archivo(self) -> Option<usize> {
        self.columnas()
            .iter()
            .position(|columna| columna.tipo == TipoDato::Archivo)
    }
}

/// Fila de una sección: valores ya formateados para mostrar/editar, en el
/// orden de [`Seccion::columnas`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registro {
    /// Id.
    pub id: i64,
    /// Valores como texto (fechas `DD/MM/AAAA`, booleanos `Sí`/`No`,
    /// referencias como id) para editar.
    pub valores: Vec<String>,
    /// Lo que muestra el listado: igual a `valores`, salvo las referencias,
    /// que muestran la etiqueta del registro apuntado.
    pub vista: Vec<String>,
}

/// Entradas del asistente (promedios de los registros recientes).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EntradasAsistente {
    /// Rendimiento promedio de los últimos 10 partidos (0-10).
    pub rendimiento: f64,
    /// Lesiones activas.
    pub lesiones: i64,
    /// Fracción de días con buen sueño (últimos 14 hábitos, 0-1).
    pub dormi: f64,
    /// Hidratación promedio (0-10).
    pub hidratacion: f64,
    /// Fracción de días con alcohol (0-1).
    pub alcohol: f64,
    /// Nervios promedio (0-10).
    pub nervios: f64,
    /// Confianza promedio en hábitos (0-10).
    pub confianza_habito: f64,
    /// Motivación promedio en hábitos (0-10).
    pub motivacion_habito: f64,
    /// Calidad de comida promedio de las últimas 14 (Buena=3, Regular=2, Mala=1; 0 sin datos).
    pub comida: f64,
    /// Hidratación promedio de los últimos 14 días nutricionales (Alta=3, Media=2, Baja=1; 0 sin datos).
    pub dia_hidratacion: f64,
    /// Fracción de días nutricionales con alcohol (0-1).
    pub dia_alcohol: f64,
    /// Entrenamientos no omitidos entre los últimos 14.
    pub entrenamientos: i64,
    /// Intensidad promedio de esos entrenamientos.
    pub intensidad: f64,
    /// Duración promedio (min).
    pub duracion: f64,
    /// Entrenamientos intensos (≥8, no omitidos) en los últimos 7 días.
    pub intensos_semana: i64,
    /// Promedios de las últimas 10 sesiones mentales (0-10).
    pub confianza: f64,
    /// Motivación.
    pub motivacion: f64,
    /// Concentración.
    pub concentracion: f64,
    /// Estrés.
    pub estres: f64,
    /// Presión.
    pub presion: f64,
}

/// Resultado del asistente de entrenamientos personalizados.
#[derive(Debug, Clone, PartialEq)]
pub struct Asistente {
    /// Puntaje de preparación 0-100.
    pub preparacion: i64,
    /// Riesgo de lesión 0-100.
    pub riesgo: i64,
    /// Nivel de carga sugerido: 0 baja, 1 moderada, 2 alta.
    pub nivel: u8,
    /// Puntajes parciales: rendimiento, hábitos, nutrición, mental, entrenamiento.
    pub parciales: [i64; 5],
    /// Resumen de una línea (formato MiFutbolC).
    pub resumen: String,
    /// Rutina semanal sugerida.
    pub rutina: String,
    /// Alerta de alta intensidad, si corresponde.
    pub alerta_intensidad: Option<String>,
}

/// Recomendación guardada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recomendacion {
    /// Fecha ISO.
    pub fecha: String,
    /// Resumen.
    pub resumen: String,
}

/// Datos de los análisis de `bienestar.c`: entrenamiento vs rendimiento y
/// estadísticas de alimentación. Los promedios son `None` sin partidos que cumplan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnalisisBienestar {
    /// Días distintos con entrenamiento no omitido.
    pub dias_entrenamiento: i64,
    /// Partidos registrados.
    pub partidos: i64,
    /// Rendimiento promedio de los partidos jugados un día con entrenamiento.
    pub rend_con_entrenamiento: Option<f64>,
    /// Rendimiento promedio de los partidos jugados un día sin entrenamiento.
    pub rend_sin_entrenamiento: Option<f64>,
    /// Días distintos con comidas registradas.
    pub dias_comida: i64,
    /// Días con al menos una comida "Buena".
    pub dias_comida_buena: i64,
    /// Rendimiento promedio los días con comida "Buena".
    pub rend_comida_buena: Option<f64>,
    /// Rendimiento promedio los días con comida "Mala".
    pub rend_comida_mala: Option<f64>,
    /// Últimos 10 partidos (`fecha_hora`, rendimiento) con comida "Mala" ese día.
    pub partidos_comida_mala: Vec<(String, i64)>,
}

impl AnalisisBienestar {
    /// Porcentaje de días con buena alimentación (0 sin días).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn pct_comida_buena(&self) -> f64 {
        if self.dias_comida > 0 {
            self.dias_comida_buena as f64 / self.dias_comida as f64 * 100.0
        } else {
            0.0
        }
    }
}
