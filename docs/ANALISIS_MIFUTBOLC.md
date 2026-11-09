# Análisis de MiFutbolC y plan de implementación para MiFutbolR

> **Qué es este documento.** Análisis completo del repositorio
> [thomashamer3/MiFutbolC](https://github.com/thomashamer3/MiFutbolC) (v4.5.1) y catálogo
> priorizado de todo lo que MiFutbolR puede incorporar, con el **cómo** técnico concreto
> (esquema, capas, pantallas, teclas, pruebas) y el **flujo de uso** actual y propuesto.
>
> **Fecha:** 2026-11-08 · **Alcance:** MiFutbolC completo + MiFutbolR al día de hoy.
> **Método:** clonado del repo, conteo de código, extracción del árbol de menús, volcado del
> esquema real de la base (75 tablas), lectura de los módulos de lógica y comparación con el
> código Rust local (75 archivos, 22.640 líneas).

---

## Índice

1. [Resumen ejecutivo](#1-resumen-ejecutivo)
2. [Metodología y evidencia](#2-metodología-y-evidencia)
3. [MiFutbolC en profundidad](#3-mifutbolc-en-profundidad)
4. [Lógica y fórmulas destacadas](#4-lógica-y-fórmulas-destacadas)
5. [Fortalezas y debilidades de MiFutbolC](#5-fortalezas-y-debilidades-de-mifutbolc)
6. [Flujo de uso de MiFutbolC](#6-flujo-de-uso-de-mifutbolc)
7. [MiFutbolR hoy: estado y cobertura](#7-mifutbolr-hoy-estado-y-cobertura)
8. [Catálogo de implementación](#8-catálogo-de-implementación)
9. [Flujo de uso propuesto para MiFutbolR](#9-flujo-de-uso-propuesto-para-mifutbolr)
10. [Roadmap por olas](#10-roadmap-por-olas)
11. [Decisiones técnicas y dependencias](#11-decisiones-técnicas-y-dependencias)
12. [Riesgos y mitigaciones](#12-riesgos-y-mitigaciones)
13. [Apéndices](#13-apéndices)

---

## 1. Resumen ejecutivo

MiFutbolC es una aplicación de consola en C (C99/C11) para llevar el historial futbolístico
personal: partidos, equipamiento, canchas, equipos, lesiones, finanzas, torneos, temporadas,
bienestar, carrera, logros y estadísticas avanzadas. Es un proyecto maduro y muy amplio:
**25 módulos de primer nivel**, **~150 logros**, **8 temas**, **2 idiomas**, **8 formatos de
exportación** y una base SQLite que en la práctica ronda las **75 tablas**.

MiFutbolR ya lo supera en lo que hace: TUI ratatui moderna, arquitectura en capas, tipado,
pruebas, animaciones, imágenes, ratón y una base de 25 tablas. Pero cubre **11 módulos** frente
a los 25 del original y todavía no tiene las capas "de hábito" (dashboard, calendario, logros,
notificaciones, recordatorios, búsqueda global, undo) ni las "de gestión deportiva" (torneos,
temporadas) ni las "de persona" (bienestar, colecciones, tiendas, amigos).

### Números clave

| Métrica | MiFutbolC | MiFutbolR |
|---|---:|---:|
| Versión analizada | 4.5.1 | 0.1.0 |
| Archivos de código propios | 162 (.c/.h) | 75 (.rs) |
| Líneas propias (sin SQLite ni cJSON) | ~89.977 | ~22.640 |
| Módulos de primer nivel | 25 | 11 |
| Tablas (declaradas / reales) | 59 / 75 | 25 |
| Logros definidos | ~150 | 0 (solo badges de carrera) |
| Formatos de exportación | TXT, CSV, JSON, HTML, PDF, ODS, XLSX, Markdown | TXT, CSV, JSON, HTML, Markdown (resumen) |
| Pruebas automatizadas | 10 suites Unity (humo, opcionales) | 157 (100 unitarias + 57 de integración) |

### Diagnóstico en una frase

MiFutbolC tiene **más superficie funcional**; MiFutbolR tiene **mejor base técnica**. La jugada
no es portar C a Rust línea por línea, sino **reimplementar los módulos que generan hábito y
valor** sobre la arquitectura existente, empezando por los que se apoyan en datos que ya están
en la base (partidos, camisetas, canchas) y dejando para el final los que exigen infraestructura
nueva (HTTP, PDF/XLSX, multiusuario).

### Las 10 mejoras de mayor retorno (Ola 0)

1. **Dashboard "Hoy"** como pantalla de inicio: próximo partido, racha, alertas y acciones rápidas.
2. **Calendario mensual** con partidos, finanzas y recordatorios en una grilla navegable.
3. **Búsqueda global + paleta de comandos** (Ctrl+K) que salte a cualquier entidad o acción.
4. **Logros** (~150) calculados por SQL sobre partidos, con progresión y notificación al desbloquear.
5. **Records & Rankings**: 16 reportes históricos que ya se pueden calcular con los datos actuales.
6. **Centro de notificaciones**: lesiones activas, presupuesto excedido, rachas, récords y avisos.
7. **Undo (Ctrl+Z)** con pila de snapshots y confirmación, para crear/editar/borrar sin miedo.
8. **Exportación filtrada** de cualquier listado a CSV/JSON/HTML/Markdown a la carpeta Exportaciones.
9. **Tutorial de primera ejecución** en 3 pasos que crea cancha, camiseta y primer partido.
10. **Atajos globales** (n, c, b, /, ?, Ctrl+Z) y ayuda actualizada.

Después vienen las olas de mayor esfuerzo pero también mayor valor: **torneos y temporadas**
(Ola 1), **bienestar, recordatorios, colecciones y amigos** (Ola 2) y **entrenador IA, clima,
sincronización, PDF/XLSX/ODS y multiusuario** (Ola 3).

### Estimación global (un desarrollador)

| Ola | Contenido | Esfuerzo |
|---|---|---:|
| 0 | Hábito y fundaciones (10 ítems) | ~2 semanas |
| 1 | Gestión deportiva (6 ítems) | ~3 semanas |
| 2 | Persona y contexto (8 ítems) | ~4 semanas |
| 3 | Inteligencia y ecosistema (8 ítems) | ~6 semanas |
| — | **Total** | **~15 semanas (3,5–4 meses)** |

Las estimaciones son orientativas y asumen dedicación parcial, reutilización de los patrones
existentes (listado → detalle → formulario) y pruebas por módulo.

---

## 2. Metodología y evidencia

### 2.1 Qué se hizo

1. **Clonado completo** del repositorio remoto en un directorio temporal y verificación de que
   el árbol de trabajo está limpio (git clone --depth 1).
2. **Conteo de código** por archivo, excluyendo las dependencias embebidas (sqlite3.c/h,
   cJSON.c/h): 162 archivos, ~89.977 líneas.
3. **Extracción mecánica del árbol de menús**: se recorrieron todos los .c buscando las
   definiciones de MenuItem para reconstruir los 25 menús de primer nivel y sus submenús.
4. **Volcado del esquema real**: se abrió una base MiFutbolC migrada (mifutbol.db, 75 tablas)
   y se listaron tablas, columnas y conteos de filas; se contrastó con las definiciones
   CREATE TABLE de db.c (59 declaradas; el resto las crean módulos bajo demanda).
5. **Lectura de lógica**: se extrajeron las funciones clave de entrenador_ia.c, logros.c,
   bienestar.c, torneo.c, temporada.c, ranking_amigos.c, sync.c, openmeteo_client.c,
   backup.c, undo.c, recordatorios.c, calendario.c, estadisticas_meta.c,
   resumen_compartible.c, ascii_charts.c, export_pdf.c, settings.c y utils.c.
6. **Lectura del flujo**: main.c, menu.c, utils.c (login multiusuario), settings.c
   (modos de menú) y el manual de usuario (1.906 líneas) para reconstruir los recorridos reales.
7. **Revisión de ingeniería**: CMake preset/Ninja, Makefile, proyecto CodeBlocks, scripts de
   instalación, análisis estático (clang-format, clang-tidy, Cppcheck, Sonar) y las 10 suites
   Unity de tests/.
8. **Auditoría del estado local**: Cargo.toml, src/ completo, migrations/, docs/,
   README.md y ACEPTACION.md para establecer qué existe hoy y con qué patrones.

### 2.2 Límites del análisis

- **No se compiló ni ejecutó MiFutbolC.** El análisis es estático, sobre código y datos.
- **No se probó la sincronización** contra un PocketBase real ni la descarga de clima (requieren
  red y credenciales). Se describe el contrato que implementa el código.
- **No se midió rendimiento** del binario C; solo se infiere de su diseño (consultas SQL por
  pantalla, sin caché, LIKE sin índices en búsquedas).
- Las cifras de tablas corresponden a una base real de ejemplo; un usuario sin usar ciertos
  módulos tendrá menos tablas creadas.
- Los conteos de logros son aproximados (±5) porque se expanden macros del array LOGROS.

### 2.3 Fuentes principales

| Fuente | Uso |
|---|---|
| README.md (1.608 líneas) | Funcionalidades, instalación, arquitectura declarada |
| Manual_Usuario_MiFutbolC.md (1.906 líneas) | Flujo de uso, semántica de cada módulo |
| menu.c + menús de cada módulo | Árbol real de navegación |
| db.c + base mifutbol.db | Modelo de datos efectivo |
| models.h, *.h | Contratos y estructuras |
| entrenador_ia.c, logros.c, bienestar.c, etc. | Algoritmos y reglas |
| src/ de MiFutbolR, migrations/, docs/ | Punto de partida y patrones de implementación |

---

## 3. MiFutbolC en profundidad

### 3.1 Arquitectura

MiFutbolC es una aplicación **monolítica de consola** organizada por capas lógicas, sin
framework: un main corto, un motor de menús numéricos y un módulo .c por dominio.

~~~mermaid
flowchart TD
    A[main.c] --> B[Login multiusuario local]
    B --> C[initialize_application]
    C --> D[Carga settings + idioma + BD]
    D --> E[Menú principal 25 opciones]
    E --> F[Módulo de dominio]
    F --> G[db.c / SQL directo]
    G --> H[(SQLite)]
    F --> I[utils.c: UI de consola]
    F --> J[export_*.c / pdfgen.c]
    F --> K[sync.c / openmeteo_client.c]
~~~

**Capas reales**

| Capa | Archivos | Responsabilidad |
|---|---|---|
| Entrada | main.c | Lock de instancia única (Linux), login, init, sync inicial, menú |
| Presentación | menu.c, utils.c, ascii_art.h, ascii_charts.c | Menús, cabeceras, tablas, pausas, entrada validada, gráficos ASCII |
| Dominio | camiseta.c, partido.c, cancha.c, equipo.c, torneo.c, temporada.c, bienestar.c, carrera.c, logros.c, entrenador_ia.c, analisis.c, records_rankings.c, resumen_compartible.c, colecciones.c, tienda.c, media.c, recordatorios.c, notificaciones.c, calendario.c, progresion.c | Reglas, cálculos y pantallas del dominio |
| Datos | db.c, db_integridad.c, backup.c, undo.c | Conexión, esquema, migraciones, integridad, backups, snapshots |
| Salidas | export*.c, export_pdf.c, pdfgen.c, import.c | TXT/CSV/JSON/HTML/PDF/ODS/XLSX/Markdown |
| Integraciones | sync*.c, openmeteo_client.c, settings.c (updates) | PocketBase REST, clima, GitHub releases |
| Soporte | settings.c, lang.c, atajos.c, tutorial.c, utils.c | Preferencias, i18n, atajos, onboarding |

**Patrones**

- **Menú como estructura de datos**: cada pantalla declara MenuItem items[] con
  {número, texto, función} y llama a ejecutar_menu(título, items, n). El motor limpia la
  pantalla, imprime, lee un entero y despacha; 0 vuelve.
- **Una función por acción**: crear_camiseta, listar_camisetas, modificar_camiseta, etc.
  La UI y la lógica viven en el mismo archivo (no hay separación estricta).
- **SQL directo** desde los módulos con sqlite3_prepare_v2 + preparar_stmt, sin capa de
  repositorio. Hay helpers de paginación (listado_paginado_config).
- **Tablas creadas bajo demanda**: varios módulos hacen ALTER TABLE ... ADD COLUMN o
  CREATE TABLE IF NOT EXISTS al entrar (por eso hay más tablas que las declaradas).
- **Estado global**: una única conexión sqlite3 *db y variables estáticas de sesión.
- **Persistencia heterogénea**: casi todo en SQLite, pero los recordatorios viven en un JSON
  (Importaciones/recordatorios.json), el undo en un JSON y las exportaciones en archivos.

**Arranque real** (main.c)

1. Lock de instancia única en Linux (/tmp/mifutbolc.lock).
2. iniciar_sesion_multiusuario_local(): registro users.db, elegir/crear perfil, contraseña
   opcional; cada perfil usa su propia base mifutbol_<usuario>.db.
3. initialize_application(): db_init, usuario activo, lang_init, settings_init, log.
4. Bienvenida con nombre, verificación silenciosa de actualización en GitHub.
5. Aviso de "Modo Retro" si hay recuerdos históricos.
6. Inicializa atajos y undo; sincroniza partidos pendientes y, si está configurado, ejecuta
   auto-sync con PocketBase.
7. Construye el menú filtrado según el modo (simple/avanzado/personalizado) y entra al bucle.

### 3.2 Modelo de datos

El esquema declarado en db.c tiene **59 tablas**; una base real de ejemplo ronda las **75**
porque los módulos crean tablas auxiliares. Agrupadas por dominio:

| Dominio | Tablas principales |
|---|---|
| Núcleo | camiseta, cancha, botin, equipo, jugador, partido, lesion, usuario, settings |
| Canchas | cancha (más de 30 columnas: dirección, localidad, tipo, superficie, techada, iluminación, horarios, precios, servicios, coordenadas, imagen, activa) |
| Partidos | partido (más de 45 columnas: contexto, métricas, clima, ratings, tags, detalles, uuid) + partido_meta, partido_tag |
| Competición | torneo, equipo_torneo, equipo_torneo_estadisticas, partido_torneo, torneo_fases, equipo_fase, equipo_historial, jugador_estadisticas |
| Temporadas | temporada, temporada_fase, torneo_temporada, equipo_temporada_fatiga, jugador_temporada_fatiga, equipo_temporada_evolucion, temporada_resumen, mensual_resumen |
| Bienestar | bienestar_objetivo, bienestar_plan_entrenamiento, bienestar_entrenamiento, bienestar_ejercicio, bienestar_entrenamiento_ejercicio, bienestar_habito, bienestar_comida, bienestar_dia_nutricional, bienestar_sesion_mental, bienestar_salud, bienestar_control_medico, bienestar_estudio_archivo, bienestar_recomendacion, bienestar_menu_imagen |
| Carrera | carrera_identidad, carrera_partido_hito, carrera_resumen_narrativo, entrenamiento_plan, progresion_jugador |
| Finanzas | financiamiento, presupuesto_mensual |
| Gamificación | Logros calculados por SQL (sin tabla propia), consejos_historial, ia_config, perfil_usuario_ia |
| Colecciones | coleccion, inventario_item, coleccion_inventario, equipamiento_historial |
| Social | amigo |
| Comercio y media | tienda, media |
| Sistema | notificacion, reporte_config, reporte_generado, backup_config, atajo_config, custom_menus, comparacion_historial |
| Sync | sync_config, sync_dirty, sync_conflicto |

**Convenciones del original**

- Dinero en **REAL** (no centavos), fechas en **texto** con formatos mixtos (YYYY-MM-DD,
  YYYY-MM-DD HH:MM:SS, a veces DD/MM/YYYY), importes y temperaturas como double.
- Booleanos como INTEGER 0/1; enums como códigos enteros o texto (estado).
- Imágenes como **ruta de archivo** (imagen_ruta) y visor propio en consola.
- Normalización de clima en clima_json (payload completo de Open-Meteo) además de columnas
  derivadas (temperatura_c, precip_mm, wind_kmh, weather_code).
- uuid + actualizado_en en partido para sincronización.

**Contraste con MiFutbolR**

MiFutbolR ya normalizó lo importante: **dinero en centavos** (monto_centavos,
precio_centavos, costo_centavos), fechas ISO, CHECK en casi todas las columnas, claves
foráneas activas, tablas hijas para tipos/superficies de cancha y migraciones versionadas.
Ese es el terreno correcto para crecer: **no conviene copiar el modelo del C**, sino traducir
sus conceptos a las convenciones locales.

### 3.3 Inventario funcional (25 módulos)

Resumen de lo que hace cada módulo y sus submenús principales. La columna "Submenú" resume las
acciones visibles; los detalles de cálculo están en la sección 4.

| # | Módulo | Submenú y capacidades |
|---:|---|---|
| 1 | Calendario | Grilla mensual, navegación por meses, eventos por día (partidos y finanzas), volver a hoy |
| 2 | Camisetas | Crear, listar (filtro/orden/estadísticas), modificar, eliminar, sortear con reinicio, cargar/ver imagen, ajustes de visor, ver/cargar información, reactivar/desactivar, gráfico ASCII de uso |
| 3 | Canchas | Crear, listar con V/E/D y filtros, modificar, eliminar, imagen, ver info, reactivar/desactivar, exportar información |
| 4 | Botines | Crear, listar, modificar, eliminar, sortear/reiniciar, imagen, ver info, cargar info, reactivar, fijar predeterminado |
| 5 | Equipos | Crear fijo, crear momentáneo (uno o dos), listar, modificar, eliminar, imagen, reactivar; plantilla de hasta 11 jugadores con dorsal, posición y capitán |
| 6 | Partidos | Crear (amistoso/torneo/entrenamiento), listar con filtros avanzados, modificar todos los campos, eliminar, simular con equipos guardados, favoritos, tags, clima histórico, submenú "Último" (victoria/derrota/empate/gol/asistencia con tiempo transcurrido) |
| 7 | Lesiones | Crear, listar, modificar (campos o todo), eliminar, estadísticas, diferencias entre lesiones, actualizar estados |
| 8 | Estadísticas | Generales, partidos, goles, asistencias, rendimiento, canchas (10 reportes), reportes automáticos, filtros avanzados |
| 9 | Logros | Ver todos, completados, en progreso y no completados (~150 logros, 5 niveles) |
| 10 | Financiamiento | Ingresos/gastos, listar, modificar, eliminar, resumen, balance, exportar, presupuestos mensuales con alertas |
| 11 | Torneos | Crear (4 tipos, 11 formatos según cantidad de equipos), listar, modificar, eliminar, administrar (fixture, resultados, tabla, goleadores, dashboard, export, finalizar) |
| 12 | Temporada | Crear, listar, modificar, eliminar, administrar (fases, torneos, fatiga, evolución, resumen, dashboard), comparar temporadas, resúmenes mensuales |
| 13 | Análisis | Análisis básico (últimos 5, rachas, mensajes), comparador avanzado (camisetas/torneos/periodos/condiciones), evolución temporal, química entre jugadores, entrenador IA |
| 14 | Bienestar | Planificación (objetivos/planes), hábitos y mentalidad, entrenamiento (sesiones, ejercicios, alertas), alimentación (comidas, días, estadísticas), salud (perfil, controles, archivos), asistente de rutinas, informe mensual PDF |
| 15 | Carrera | Identidad, perfil dinámico, partidos que marcaron, timeline, Hall of Fame, resumen narrativo, modo retro "hoy en tu historia", estadísticas por temporada, logros de carrera, entrenamiento y progresión |
| 16 | Recordatorios | Listar, agregar, editar, eliminar, filtrar por temática, exportar/importar JSON, agenda (recordatorios + partidos), recurrencia diaria/semanal/mensual |
| 17 | Colecciones | Inventario (crear/listar/filtrar), sincronizar camisetas, colecciones, asociar/quitar ítems, backup JSON, historial de equipamiento |
| 18 | Records & Rankings | 16 reportes: récords de goles/asistencias, mejores/peores combinaciones cancha+camiseta, temporadas, partidos, rachas, top 5, ranking de camisetas |
| 19 | Tiendas | CRUD de tiendas con contacto, redes, productos, rango de precio, envío, rating y favorito |
| 20 | Referencias Multimedia | CRUD de video/foto/artículo/audio asociado a partidos, con filtro por tipo |
| 21 | Tutorial | 8 pasos guiados, ver paso específico, reiniciar; persiste tutorial_completed |
| 22 | Resumen Compartible | Genera HTML (tarjeta visual con Open Graph) o Markdown con estadísticas, racha, camiseta/cancha/rival favoritos, lesiones, mejor temporada |
| 23 | Ranking Amigos | CRUD de amigos, ranking con el usuario local incluido, comparación directa |
| 24 | Ajustes | Tema (8), idioma (ES/EN), accesibilidad (tamaño de texto, alto contraste), usuario y contraseña, modo de menú (simple/avanzado/personalizado), exportar, importar, ODS, búsqueda global, actualización, backup/restore, integridad, undo, notificaciones |
| 25 | Sincronización | Configurar servidor PocketBase, login, sincronizar ahora, subir todo, estado/pendientes, conflictos, auto-sync, cerrar sesión |

Además, fuera del menú: atajos globales (D dashboard, B búsqueda, C calendario, N nuevo partido,
S estadísticas, H ayuda, Q salir) y un dashboard que el tutorial describe aunque no aparece como
opción numerada.

### 3.4 Ingeniería, build y calidad

Más allá de las features, el repositorio muestra una ingeniería de proyecto bastante completa:

| Área | Implementación |
|---|---|
| Build | CMake ≥ 3.20 con presets Ninja (mingw-debug/release, linux-debug/release), biblioteca estática mifutbolc_core + ejecutable; Makefile simple y proyecto CodeBlocks (.cbp) |
| Estándar | C11 estricto (CMAKE_C_EXTENSIONS OFF); -Wall -Wextra -Wpedantic en GCC y /W4 en MSVC |
| SQLite | Compilado con SQLITE_THREADSAFE=0, sin extensiones ni JSON1, y flags de tamaño |
| Tests | Unity minimal en tests/; 10 suites (utils, camiseta, cancha, botín, equipo, lesión, partido, temporada, torneo, estadísticas de canchas); se activan con -DMIFUTBOLC_BUILD_TESTS=ON y corren con ctest; validan símbolos y construcción de menús, no flujos con base de datos |
| Análisis estático | .clang-format, .clang-tidy, Cppcheck (script y archivo de entrada), SonarCloud y CodeFactor |
| Instalación | Inno Setup (MiFutbolC.iss), install.ps1 (Windows, instala en LOCALAPPDATA y crea launcher), Instalador-Linux.sh (dependencias + build + launcher), Actualizar.sh (git pull/tag + rebuild) |
| Escritorio | mifutbolc.desktop e icono .ico para el instalador |
| Documentación | README.md/html/pdf, Manual_Usuario en md/pdf/html, index.html, DeepWiki y videos de presentación/tutorial |
| Dependencias | SQLite 3.53.2, cJSON y pdfgen embebidos; sin DLLs externas |
| Licencia | GPL-3.0 |

**Lectura**: el proyecto cuida el build reproducible, los warnings y el análisis estático, pero
las pruebas son de humo y quedan fuera los módulos más complejos (bienestar, carrera, sync,
exportación). Para MiFutbolR la lección es inversa: mantener la disciplina de build (ya existe)
y sumar pruebas de comportamiento donde el C no las tiene.

**Nota de licencia**: MiFutbolC es GPL-3.0 y MiFutbolR es MIT. Este documento analiza ideas,
fórmulas y contratos; **no se debe copiar código del proyecto C** a MiFutbolR sin revisar la
compatibilidad. Reimplementar la lógica en Rust es la vía correcta.

---

## 4. Lógica y fórmulas destacadas

Esta sección documenta los cálculos que conviene **replicar con fidelidad** (o mejorar a
propósito) al implementar cada módulo. Todos se pueden expresar en SQL sobre las tablas que
MiFutbolR ya tiene o sobre las nuevas que se proponen en la sección 8.

### 4.1 Entrenador IA

Evalúa el estado reciente del jugador y emite consejos. No usa modelos externos: son reglas
sobre los últimos partidos.

**Estado calculado** (últimos 10 partidos, orden descendente por fecha):

- rendimiento_promedio, cansancio_promedio, estado_animo_promedio (medias).
- partidos_consecutivos: partidos jugados en los últimos 7 días (tope 7).
- derrotas_consecutivas: racha negativa actual.
- dias_descanso: días desde el último partido.
- **riesgo_lesion** = (cansancio_efectivo / 10) + (partidos_consecutivos / 3) + (derrotas_consecutivas / 2).

El cansancio se atenúa por el descanso:

| Días sin jugar | Factor |
|---|---:|
| 0 | 1,00 |
| 1 a 6 | 1 − (días / 14) |
| 7 a 13 | 0,50 |
| 14 o más | 0,00 |

**Reglas de consejo**

| Condición | Nivel | Categoría |
|---|---|---|
| Cansancio > 8 y ≥ 3 partidos en 7 días | Advertencia | Físico |
| Rendimiento promedio < 3 | Advertencia | Deportivo |
| Ánimo promedio < 3 y ≥ 2 derrotas seguidas | Advertencia | Mental |
| Riesgo de lesión > 3,0 | Crítico | Salud |
| Más de 14 días sin jugar | Info | Deportivo |
| Sin ninguna condición | Info | Físico ("estado general bueno") |

**Niveles de intervención** (persistidos en ia_config): 0 silencioso, 1 conservador (solo
crítico), 2 moderado (alto, por defecto), 3 agresivo (cualquier indicio). Hay un enfriamiento de
24 h para no repetir el aviso antes de cada partido.

**Otras funciones**: predicción de resultado (a partir de promedios y contexto), recomendación
de formación (jugadores disponibles por posición), alertas de rendimiento, sugerencia de
descanso y análisis de puntos débiles. El historial guarda consejo + si se siguió, y calcula un
**índice de prudencia** = aceptados / (aceptados + ignorados).

### 4.2 Logros

No hay tabla de logros: cada logro es una **consulta SQL derivada** sobre partido, filtrada por
la camiseta activa. Ejemplo:

~~~sql
SELECT COUNT(*) FROM partido WHERE camiseta_id = ? AND goles >= 3;  -- hat-tricks
SELECT IFNULL(SUM(goles), 0) FROM partido WHERE camiseta_id = ?;    -- goles totales
SELECT COUNT(DISTINCT cancha_id) FROM partido WHERE camiseta_id = ?; -- canchas distintas
~~~

Cada logro declara nombre, descripción, objetivo y tipo; el progreso es valor/objetivo y el
estado es 0 (no iniciado), 1 (en progreso) o 2 (completado). Hay **~150 logros** agrupados en
series de 5 niveles (Novato, Promedio, Experto, Maestro, Leyenda), por ejemplo:

- Goles (1, 5, 10, 25, 50, 100), asistencias, partidos (1, 5, 10, 25, 50, 100).
- Victorias, empates, derrotas; contribuciones (goles + asistencias).
- Rendimiento y ánimo acumulados; rendimiento/ánimo perfecto.
- Canchas distintas; hat-tricks; póker de asistencias; rachas (victorias, derrotas, empates).
- Logros contextuales: goles en victorias/derrotas/empates, rendimiento en cada resultado.
- Promedios por partido (goles, asistencias, rendimiento, ánimo) escalados ×10.
- Último partido: goles, asistencias, rendimiento y ánimo del partido más reciente.

### 4.3 Bienestar

El módulo más grande del original (3.738 líneas, 14 tablas). Su pieza central es el **asistente
de entrenamiento**, que calcula dos puntajes de 0 a 100.

**Puntaje de preparación** (prep_score), ponderado:

| Componente | Peso | Cálculo |
|---|---:|---|
| Rendimiento | 30 % | promedio de rendimiento × 10 |
| Hábitos | 20 % | media de sueño, hidratación, nervios invertidos, confianza y motivación − alcohol × 15 |
| Nutrición | 20 % | media de calidad de comidas e hidratación diaria − alcohol × 15 |
| Mental | 20 % | (confianza + motivación + concentración)/3 × 10 − (estrés + presión)/2 × 3 |
| Entrenamiento | 10 % | función de cantidad, intensidad media y duración media |

**Riesgo de lesión**:

- Si hay lesiones activas: 70 + 5 × lesiones (tope 100).
- Si no: (100 − prep_score) × 0,6 + estrés × 3 + entrenamientos intensos × 5 (tope 100).

**Nivel de rutina**: 2 si prep ≥ 75 y sin lesiones; 1 si prep ≥ 50; 0 en caso contrario. La
rutina sugerida cambia por nivel y por objetivo activo; si el riesgo ≥ 70 agrega una alerta de
recuperación. Cada recomendación se guarda con fecha, score, riesgo, resumen y rutina.

**Submódulos**: objetivos y planes, hábitos diarios (sueño, hidratación, alcohol, ánimo,
nervios, confianza, motivación), sesiones mentales (tipo, momento, partido, confianza, estrés,
miedo, presión, concentración), entrenamientos y ejercicios (series, repeticiones, tiempo),
alimentación (comidas y día nutricional), salud (perfil, controles médicos con archivos
adjuntos) e informe personal mensual en PDF.

### 4.4 Temporadas

- **Fases**: pretemporada, temporada regular y postemporada.
- **Fatiga de equipo**: 10 puntos por partido jugado en la ventana reciente, tope 100.
- **Fatiga de jugador**: acumula minutos, partidos y lesiones; se guarda una foto por fecha.
- **Evolución**: puntuación 0–100, tendencia (Mejorando / Estable / Decayendo), partidos ganados
  sobre totales.
- **Resumen de temporada**: partidos, goles, promedio, campeón, goleador, lesiones.
- **Resumen mensual**: partidos, goles, V/E/D, lesiones, ingresos, gastos, mejor y peor equipo
  del mes.
- **Comparación**: dos temporadas lado a lado por las mismas métricas.

### 4.5 Torneos

**Tipos**: ida y vuelta, solo ida, eliminación directa y grupos + eliminación.

**Formatos** (11): round robin, mini grupo con final, liga simple, liga doble, grupos con final,
copa simple, grupos + eliminación, copa con repechaje, liga grande, múltiples grupos y
eliminación por fases. La cantidad de equipos decide qué opciones se ofrecen:

| Equipos | Opciones |
|---:|---|
| 4–6 | Round robin ida y vuelta; mini grupo con final |
| 7–12 | Liga simple; liga doble; grupos con final; copa simple |
| 13–20 | Grupos + eliminación (por defecto); copa con repechaje; liga grande |
| 21+ | Múltiples grupos (por defecto); eliminación por fases |

**Fixture**: método del círculo. Se fija un equipo y se rotan los demás; si la cantidad es impar
se agrega un bye (−1) que no genera partido. Con ida y vuelta se inserta también el partido
invertido. Todos los partidos nacen en fase "Fase de Grupos".

**Tabla de posiciones**: victoria 3, empate 1, derrota 0; se actualizan partidos jugados,
ganados, empatados, perdidos, goles a favor, en contra y puntos de ambos equipos en cada
resultado. El dashboard muestra próximos partidos, estado por fase, estadísticas por equipo y
goleadores.

### 4.6 Ranking de amigos

Cada amigo guarda posición, goles, asistencias, partidos y mejor rendimiento. El ranking arma
una lista con todos los amigos **más el usuario local** (calculado desde partido) y la ordena
por goles con un comparador propio; muestra puesto, nombre, goles, asistencias, partidos y mejor
rendimiento. La comparación directa enfrenta las métricas del amigo contra las del usuario.

### 4.7 Meta-análisis

Reportes calculados con SQL agregado, sin tabla intermedia:

- **Consistencia**: promedio, desviación estándar con SQRT(AVG(x²) − AVG(x)²), coeficiente de
  variación porcentual, mínimo y máximo.
- **Partidos atípicos**: rendimiento fuera de media ± 2 desviaciones estándar (alto/bajo).
- **Dependencia del contexto**: rendimiento según clima, día, franja, cancha, rival y formato.
- **Impacto real del cansancio y del ánimo**: comparación de promedios entre grupos alto/bajo.
- **Eficiencia**: goles vs rendimiento, asistencias vs cansancio.
- **Rendimiento por intensidad, dolor físico, arbitraje y temperatura**.
- **Tendencia**: sparkline de los últimos partidos.

### 4.8 Calendario

Dos máscaras de bits por mes: una marca los días con partidos y otra los días con movimientos
financieros. La grilla se dibuja con el primer día de la semana y los días del mes; al elegir un
día se listan los eventos (partido, finanzas, recordatorio). Navegación por mes anterior,
siguiente y "hoy".

### 4.9 Recordatorios

Viven en un archivo JSON (Importaciones/recordatorios.json), no en SQLite. Campos: id, fecha,
hora, nota, temática y periodicidad (una vez, diario, semanal, mensual). Al verificar
recurrentes, calcula la próxima fecha y agrega una copia para el ciclo siguiente; el original
permanece y la agenda muestra la ocurrencia vigente. La agenda mezcla recordatorios y partidos en
una sola lista ordenada por fecha.

### 4.10 Sincronización con PocketBase

Cliente REST propio (sin libcurl en Windows: usa curl.exe; en Linux, popen con curl). Contrato:

- Autenticación: POST /api/collections/users/auth-refresh con el token guardado.
- Búsqueda remota: GET /api/collections/<coleccion>/records?filter=uuid='<uuid>'&perPage=1.
- Subida: creación o actualización del registro remoto; borrado = tombstone con marca de tiempo.
- Entidades sincronizadas: partidos (sync.c) y catálogos de camisetas y botines (sync_entidades.c).
- Cola local: sync_dirty (partido_id, uuid, operación, estado, último error, creado_en).
- Conflictos: si ambos lados cambiaron, guarda local_json/remoto_json en sync_conflicto y ofrece
  conservar local o remoto.
- Configuración: URL local y remota, email, token, user_id, cursor, última sync, auto_sync.

### 4.11 Clima con Open-Meteo

Se dispara al registrar un partido si la cancha tiene latitud y longitud. Pide a la API de
archivo de Open-Meteo:

- latitude, longitude, start_date, end_date (mismo día), timezone=auto.
- daily: sunrise, sunset.
- hourly: temperature_2m, apparent_temperature, precipitation, weather_code, wind_speed_10m.

Elige la hora del partido (o el mediodía), guarda temperatura, sensación térmica,
precipitación, viento, código WMO, hora de salida/puesta del sol y el JSON completo. Los códigos
WMO se traducen a texto (despejado, nublado, llovizna, lluvia, tormenta, nieve, etc.). Rechaza
fechas futuras y valida los parámetros antes de construir el comando. También existe un proceso
por lotes para completar el clima de partidos históricos.

### 4.12 Undo y backups

- **Undo**: pila de hasta 50 operaciones en Exportaciones/undo_history.json. Cada entrada guarda
  tabla, id, tipo (create/update/delete), descripción, timestamp y snapshots JSON antes/después
  (4 KB). Deshacer restaura el snapshot y elimina la entrada.
- **Backup**: crea copias con fecha; el auto-backup guarda intervalo en horas y próxima fecha, y
  al arrancar crea uno si venció. Restaurar y eliminar se hacen desde el menú.
- **Integridad**: PRAGMA integrity_check, VACUUM, ANALYZE, reparación y ficha con tamaño/tablas.

### 4.13 Exportación e informes

| Formato | Implementación |
|---|---|
| TXT / CSV | Volcado de tablas y reportes, un archivo por tema |
| JSON | Estructura completa o por módulo; también formato MiFutbolMobile |
| HTML | Documento autocontenido con CSS embebido, cabecera de marca, filas cebra, insignias de resultado, modo oscuro y reglas de impresión; datos escapados |
| Markdown | Exportación total y resumen compartible |
| PDF | Motor propio (pdfgen): páginas, fuentes, texto con wrap, líneas, rectángulos e imágenes; el "Informe Total" concatena ~20 reportes TXT |
| ODS / XLSX | Hojas de cálculo con una hoja por módulo |

Además del export completo, hay **exportación filtrada**: el listado de camisetas o canchas
exporta solo el conjunto visible (estado + búsqueda) con los mismos formatos.

### 4.14 Simulador de partidos

Genera un partido entre equipos guardados con eventos aleatorios (goles, asistencias, tarjetas,
lesiones), marcador y estadísticas, con una cancha ASCII y animación por consola. Guarda el
resultado si el usuario lo confirma.

---

## 5. Fortalezas y debilidades de MiFutbolC

### 5.1 Fortalezas

1. **Cobertura funcional enorme**: 25 módulos que cubren el ciclo completo de un futbolista
   amateur, desde el partido hasta la salud, las finanzas y la carrera.
2. **Datos ricos**: más de 45 columnas en partido (clima, ratings, contexto, tags, detalles) y
   más de 30 en cancha; permiten análisis que MiFutbolR todavía no explota.
3. **Estadística seria**: meta-análisis con desviación estándar, outliers, consistencia,
   eficiencia y comparaciones; no se queda en promedios.
4. **Gamificación y hábito**: logros por niveles, rachas, records, carrera, modo retro, tutorial,
   notificaciones y ranking de amigos.
5. **Portabilidad**: SQLite y cJSON embebidos, sin DLLs; compila en Windows/Linux/macOS.
6. **Exportación amplia** y reportes HTML/PDF con acabado cuidado.
7. **Documentación**: README extenso y manual de usuario de 1.906 líneas.

### 5.2 Debilidades

1. **UI de consola rígida**: menús numéricos, "Presione ENTER para continuar", sin navegación
   por flechas ni búsqueda global real (la búsqueda global existe, pero enterrada en Ajustes).
2. **Sin separación de capas**: UI, SQL y reglas en el mismo archivo; difícil de testear.
3. **Esquema improvisado**: tablas creadas bajo demanda, ALTER TABLE al vuelo, formatos de fecha
   y dinero heterogéneos, sin claves foráneas en varias tablas.
4. **Duplicación**: múltiples export_*.c y variantes "mejorado"; lógica repetida entre módulos.
5. **Persistencia dispersa**: recordatorios y undo en JSON fuera de la base; backups y exports en
   carpetas.
6. **Riesgos de integridad**: borrados físicos, restauraciones manuales y sync con conflictos
   textuales.
7. **Pruebas de humo limitadas**: 10 suites Unity que verifican símbolos y menús, sin base de
   datos ni flujos; quedan fuera sync, bienestar, carrera y exportación.
8. **Accesibilidad limitada**: tamaño de texto y alto contraste son básicos; la consola depende
   de la terminal y de chcp 65001.
9. **Sincronización frágil**: depende de curl externo, un servidor PocketBase y un modelo de
   conflictos simple.
10. **Mantenibilidad**: 90k líneas de C con funciones largas y estado global.
11. **Atajos muertos**: la infraestructura de atajos existe, pero no se invoca desde el bucle
    principal ni desde los menús.

### 5.3 Qué conviene copiar y qué no

| Copiar | No copiar |
|---|---|
| El catálogo de features y sus reglas de negocio | La UI de menús numerados y pausas |
| Las fórmulas de IA, bienestar, fatiga y meta-análisis | El esquema con tablas bajo demanda |
| La idea de logros derivados por SQL | El dinero en REAL y las fechas mixtas |
| La agenda que mezcla recordatorios y partidos | Guardar recordatorios/undo en JSON |
| El resumen compartible con Open Graph | El PDF armado concatenando TXT |
| La exportación filtrada por listado | La duplicación de exportadores |

---

## 6. Flujo de uso de MiFutbolC

### 6.1 Arranque y sesión

~~~mermaid
flowchart TD
    A[Ejecutar MiFutbolC] --> B{¿Otra instancia?}
    B -- sí --> Z[Error y salir]
    B -- no --> C[Login multiusuario local]
    C --> D[Elegir o crear perfil]
    D --> E[Contraseña opcional]
    E --> F[Abrir mifutbol_usuario.db]
    F --> G[Idioma, tema, modo de menú]
    G --> H{¿Actualización?}
    H -- sí --> I[Aviso]
    H -- no --> J[Menú principal]
    I --> J
    J --> K[Elegir 1-25 o 0 para salir]
    K --> L[Submenú del módulo]
    L --> M[Acción: crear/listar/modificar/eliminar]
    M --> N[Pausa y volver]
    N --> L
    L -- 0 --> J
~~~

Puntos observados:

- El **login siempre aparece**: con varios perfiles se elige usuario; con uno solo se pide su
  contraseña (si tiene). Es una barrera de entrada para un uso personal.
- El menú principal se **filtra por modo**: simple muestra pocos módulos, avanzado todos y
  personalizado permite habilitar/deshabilitar cada uno.
- Los módulos nuevos (sync) no figuran en el manual, señal de que la documentación va por detrás
  del código.

### 6.2 Motor de navegación

- Cada pantalla es un MenuItem items[] con opciones numéricas; 0 vuelve.
- input_int(">") valida; una opción inexistente se ignora y se vuelve a mostrar.
- Cada acción termina en pause_console() ("Presione ENTER"), lo que añade un paso por pantalla.
- Las pantallas se redibujan completas (clear_screen) y usan arte ASCII para cabeceras.
- Los atajos globales (D/B/C/N/S/H/Q) están definidos y tienen pantalla de ayuda, pero
  verificar_atajo no se llama desde el flujo principal: en la práctica son código muerto.

### 6.3 Flujo típico de un usuario nuevo

~~~mermaid
flowchart LR
    A[Primer inicio] --> B[Crear perfil]
    B --> C[Ajustes: tema/idioma/modo]
    C --> D[Crear camiseta]
    D --> E[Crear cancha + coordenadas]
    E --> F[Crear botín predeterminado]
    F --> G[Crear equipo y jugadores]
    G --> H[Registrar primer partido]
    H --> I[Ver estadísticas]
    I --> J[Revisar logros]
    J --> K[Exportar resumen]
~~~

**Registro de un partido** (según el manual y el código):

1. Menú 6 → Crear partido.
2. Modalidad: amistoso, torneo o entrenamiento.
3. Fecha y hora; cancha (si tiene coordenadas, se consulta el clima); camiseta y botín.
4. Marcador, goles, asistencias, rendimiento, cansancio, ánimo.
5. Contexto ampliado: rival, posición, formato, tarjeta, dolor, arbitraje, temperatura, tags,
   lo mejor y qué mejorar.
6. Al guardar se inserta el partido y se crea un movimiento financiero de alquiler de cancha
   asociado; el listado y las estadísticas se calculan al consultar.
7. Desde el listado: filtros, favoritos, tags, editar, eliminar, "Último" con tiempo transcurrido.

### 6.4 Flujos secundarios

| Flujo | Recorrido | Observación |
|---|---|---|
| Torneo | Torneos → crear → asociar equipos → administrar → generar fixture → cargar resultados → tabla → finalizar | Muchos pasos, pero potente |
| Temporada | Temporada → crear → administrar → fases/torneos → fatiga/evolución → resumen → comparar | Concepto denso para un usuario casual |
| Bienestar | Bienestar → planificación/hábitos/entrenamiento/alimentación/salud/asistente | 6 submenús y 14 tablas |
| Carrera | Carrera → identidad → perfil → hitos → timeline → Hall of Fame → retro → logros | Muy narrativo, poca edición |
| Recordatorios | Recordatorios → agenda (recordatorios + partidos) | Datos fuera de SQLite |
| Exportar | Ajustes → Exportar → módulo → formato | También hay export por módulo |
| Sincronizar | Ajustes → Sync (o menú 25) → configurar → login → sincronizar | Requiere servidor externo |
| Undo | Ajustes → Undo → confirmar | No hay Ctrl+Z real |
| Notificaciones | Ajustes → Notificaciones | No hay bandeja global |

### 6.5 Fricciones detectadas

1. **Profundidad de menús**: para registrar un partido hay que atravesar 2–3 niveles; no hay
   acceso directo desde el inicio.
2. **Pausa obligatoria** después de cada acción: duplica las pulsaciones y corta el ritmo.
3. **Sin dashboard real**: el tutorial lo menciona, pero no hay pantalla de inicio con contexto
   ("qué pasó esta semana", "qué toca ahora").
4. **Búsqueda global escondida** en Ajustes, cuando debería ser una tecla global.
5. **Undo y notificaciones** también viven en Ajustes, fuera del flujo natural.
6. **Recordatorios fuera de la base**: no se pueden consultar con SQL ni sincronizar.
7. **Clima silencioso**: si falla la red o faltan coordenadas, el partido se guarda sin clima y
   sin aviso claro.
8. **Formato de fechas mixto**: algunas consultas asumen ISO y otras DD/MM/YYYY; fuente de bugs.
9. **Listados largos sin búsqueda global**: cada módulo tiene su propio filtro, pero no hay una
   búsqueda transversal.
10. **Sync con curl externo**: en Windows depende de curl.exe; si falta, falla sin explicación.

### 6.6 Lecciones para MiFutbolR

- Mantener el **modelo de datos rico**, pero con las convenciones locales (centavos, ISO, CHECK,
  FK, migraciones).
- Reemplazar la navegación numérica por **teclado + ratón + búsqueda + acciones rápidas**.
- Convertir los "menús escondidos" (logros, notificaciones, undo) en **superficies de primer
  nivel**.
- Llevar recordatorios, notificaciones y undo a **SQLite** o a un almacén único.
- Hacer del **dashboard** la pantalla de inicio y del **calendario** la vista de planificación.
- No portar el PDF concatenando TXT: usar un generador real o exportar HTML imprimible.

---

## 7. MiFutbolR hoy: estado y cobertura

### 7.1 Arquitectura actual

MiFutbolR es una TUI en Rust 2024 con ratatui 0.30 y SQLite (rusqlite con bundled). La
separación es estricta:

~~~mermaid
flowchart LR
    A[main.rs] --> B[App: estado + pantallas + teclado]
    B --> C[services: reglas + transacciones]
    C --> D[db: repositorios SQL]
    D --> E[(SQLite)]
    B --> F[ui: ratatui]
    F --> B
~~~

- **ui** solo dibuja: recibe el App y no ejecuta SQL ni reglas.
- **app** mantiene la pila de pantallas, el foco, los formularios, la búsqueda y las animaciones.
- **services** valida, decide (eliminar vs desactivar), ejecuta transacciones y devuelve errores
  tipados con thiserror.
- **db** expone funciones por repositorio sobre una Connection; Database centraliza la conexión,
  las migraciones (PRAGMA user_version) y los backups (VACUUM INTO).
- **domain** contiene modelos, dinero en centavos, fechas, validaciones y estadísticas puras.
- **util** contiene logging, texto seguro y helpers.

**Estado**: 75 archivos, 22.640 líneas, 25 tablas, 11 módulos, 157 pruebas, clippy y fmt limpios,
release de 4,81 MB.

### 7.2 Módulos y pantallas actuales

| Módulo | Pantallas | Funciones clave |
|---|---|---|
| Camisetas | Lista, detalle, formulario | CRUD, búsqueda, filtro, orden, sorteo, activa/inactiva, imagen, miniatura |
| Canchas | Lista, detalle, formulario | CRUD, tipos/superficies múltiples, servicios, precios, coordenadas, imagen, activa |
| Botines | Lista, detalle, formulario | CRUD, predeterminado, sorteo, activa, imagen |
| Equipos | Lista, detalle, formulario, jugador | CRUD, plantel, dorsales únicos, capitán único, imagen |
| Partidos | Lista, detalle, formulario, wizard | CRUD, wizard 4 pasos, filtros, paginación, favoritos, tags, transacción financiera, imágenes |
| Lesiones | Lista, detalle, formulario | CRUD, estados, filtros, estadísticas, aviso de fecha futura |
| Estadísticas | Hub, detalle | Generales, por mes, por año, rendimiento (20 métricas), canchas, goles/asistencias, récords, sparkline, BarChart, LineGauge |
| Financiamiento | Lista, formulario de transacción, presupuesto | CRUD, filtros, resumen, categorías, presupuesto mensual con alertas |
| Carrera | Menú, resumen, identidad, hitos, timeline, Hall of Fame, perfil, retro, logros | Identidad, badges, rachas, hitos, narrativa, modo "hoy en tu historia" |
| Resumen | Vista previa | Markdown/HTML/texto, exportación a carpeta Exportaciones |
| Ajustes | Formulario | Tema (8), idioma, modo, tamaño, visor, botín, backup, exportar BD, integridad, restaurar, importar legacy |

Además: menú principal con 11 módulos, tabs (≥70 columnas) o sidebar (≥140), breadcrumb, footer
con atajos, modales, visor de imagen a pantalla completa, ratón (rueda y clic), animaciones.

### 7.3 Flujo actual

1. main.rs parsea --version, --help y --importar-legacy; abre Database; crea App; captura el
   ratón; corre el bucle.
2. El bucle dibuja, espera 120 ms (16 ms si hay animaciones), lee teclado o ratón.
3. App.handle_key resuelve modales, búsqueda, formularios y atajos; luego delega en la pantalla.
4. Menú: flechas o j/k, Enter, 1-9 directo, ? ayuda, q salir.
5. Módulo: n nuevo, e editar, d borrar, Enter detalle, s sortear, f filtro, o orden, / buscar,
   r refrescar; formularios con Tab y Ctrl+S; Esc vuelve.
6. Detalle: e, d, v imagen, Esc.
7. Cambio de módulo reinicia la pila a MainMenu y abre el listado del módulo.
8. Ajustes: Ctrl+B backup, Ctrl+I integridad, Ctrl+E exportar BD, Ctrl+R restaurar, Ctrl+L
   importar legacy.

**Fortalezas**: consistencia de patrones, feedback por footer, navegación rápida, búsqueda
incremental, mouse, imágenes, animaciones sutiles, buenos tiempos (listados ~215 µs, stats ~71 ms).

**Fricciones propias**:

- No hay pantalla de inicio con contexto: se entra directo al menú de módulos.
- La búsqueda es por listado, no transversal.
- No hay logros, notificaciones, recordatorios, calendario ni undo.
- El estado "hoy" (próximo partido, racha, alertas) no existe como superficie.
- La exportación está limitada al resumen y a la base; no hay export por listado.
- El idioma, el modo y el tamaño de texto se guardan y se muestran en Ajustes, pero no
  modifican la interfaz; solo el tema se aplica.
- No hay login ni perfiles; la tabla usuario existe pero no se usa.
- La ayuda es una pantalla estática; no hay onboarding.

### 7.4 Matriz de cobertura frente a MiFutbolC

Leyenda: ✅ implementado · 🟡 parcial · ❌ ausente.

| Capacidad | MiFutbolC | MiFutbolR | Comentario |
|---|:--:|:--:|---|
| CRUD camisetas / canchas / botines / equipos / partidos / lesiones | ✅ | ✅ | Base sólida |
| Sorteos y predeterminados | ✅ | ✅ | Camiseta, botín |
| Imágenes y visor | ✅ | ✅ | Mejor en MiFutbolR (ratatui-image) |
| Wizard de partido | ❌ | ✅ | MiFutbolR agrega wizard |
| Favoritos y tags de partido | ✅ | ✅ | — |
| Submenú "Último" con tiempo transcurrido | ✅ | ❌ | Fácil de agregar |
| Simulador de partido | ✅ | ❌ | Ola 3 / opcional |
| Clima automático (Open-Meteo) | ✅ | ❌ | Columnas ya existen |
| Lesiones: CRUD y estados | ✅ | ✅ | — |
| Lesiones: diferencias y actualización masiva | ✅ | 🟡 | Falta "diferencias" |
| Estadísticas generales y series | ✅ | ✅ | — |
| 20 métricas de rendimiento | ✅ | 🟡 | Parcial |
| Meta-análisis (consistencia, outliers, impacto) | ✅ | 🟡 | Falta outliers/impacto |
| Estadísticas de canchas (10 reportes) | ✅ | 🟡 | Menos reportes |
| Records & Rankings (16) | ✅ | 🟡 | Récords básicos |
| Logros (~150) | ✅ | ❌ | Gran quick win |
| Torneos completos | ✅ | ❌ | Tablas existen sin UI |
| Temporadas completas | ✅ | ❌ | Tabla existe sin UI |
| Comparador A/B | ✅ | ❌ | — |
| Química entre jugadores | ✅ | ❌ | — |
| Bienestar (14 tablas) | ✅ | ❌ | Ola 2 |
| Recordatorios y agenda | ✅ | ❌ | Ola 2 |
| Colecciones e inventario | ✅ | ❌ | Ola 2 |
| Tiendas | ✅ | ❌ | Ola 2 |
| Referencias multimedia | ✅ | ❌ | Ola 2 |
| Ranking de amigos | ✅ | ❌ | Ola 2 |
| Carrera (identidad, hitos, timeline, retro) | ✅ | ✅ | Buen nivel |
| Resumen compartible | ✅ | ✅ | Markdown/HTML/texto |
| Calendario mensual | ✅ | ❌ | Ola 0 |
| Dashboard de inicio | 🟡 | ❌ | Ola 0 |
| Notificaciones | ✅ | ❌ | Ola 0 |
| Búsqueda global | ✅ | 🟡 | Solo por listado |
| Undo / historial | ✅ | ❌ | Ola 0 |
| Backup manual / integridad | ✅ | ✅ | — |
| Auto-backup programado | ✅ | ❌ | Fácil |
| Importar JSON/CSV/HTML | ✅ | ❌ | Solo legacy |
| Exportar PDF / XLSX / ODS | ✅ | ❌ | Ola 3 |
| Exportación filtrada por listado | ✅ | 🟡 | Solo resumen |
| Multiperfil + contraseña | ✅ | ❌ | Tabla sin flujo |
| i18n ES/EN | ✅ | 🟡 | Ajuste sin efecto |
| Modo simple/avanzado/personalizado | ✅ | 🟡 | Ajuste sin efecto |
| Tamaño de texto / alto contraste | 🟡 | 🟡 | Básico |
| Atajos configurables | 🟡 | ❌ | Definidos en C pero sin cablear; en Rust, teclas fijas |
| Auto-actualización | ✅ | ❌ | Ola 3 |
| Sync PocketBase | ✅ | ❌ | Ola 3 |
| Tutorial / onboarding | ✅ | 🟡 | Solo pantalla de ayuda |
| Gráficos ASCII | ✅ | 🟡 | Sparkline y barras |
| Tests automatizados | 🟡 | ✅ | 10 suites de humo vs 157 pruebas reales |
| Animaciones y mouse | ❌ | ✅ | Ventaja de MiFutbolR |

**Conclusión de la matriz**: de 49 capacidades comparadas, MiFutbolR tiene 12 completas, 12 parciales y 25 ausentes. Las ausentes se concentran en tres bloques: hábito (dashboard,
calendario, logros, notificaciones, undo), gestión deportiva (torneos, temporadas, comparador) y
persona (bienestar, recordatorios, colecciones, tiendas, amigos).

---

## 8. Catálogo de implementación

Esta es la parte accionable: **32 ítems** agrupados en cuatro olas, cada uno con qué es, qué
datos toca, cómo se implementa en la arquitectura actual y cómo se verifica. Los identificadores
(IMP-01, IMP-02, …) sirven para abrir issues o tareas y para trazar el roadmap.

### 8.0 Convenciones y receta general

**Prioridad**: P0 = sin esto el resto no se disfruta; P1 = alto valor; P2 = deseable; P3 = nicho.

**Esfuerzo**: S ≤ 3 días; M = 1–2 semanas; L = 3–6 semanas (un desarrollador, dedicación parcial).

**Estados de cada ítem**: propuesto → en curso → hecho. Ningún ítem se considera hecho sin:
migración (si aplica), repositorio, servicio, pantalla, teclas, pruebas y documentación.

**Receta para agregar un módulo** (aplicable a casi todos los ítems):

1. **Migración** nueva en migrations/ (004, 005, …) y agregarla al array MIGRATIONS de
   src/db/mod.rs. Nunca editar una migración ya publicada.
2. **Modelos** en src/domain/models.rs (entidad, fila de listado, filtro, DTO de detalle).
3. **Repositorio** en src/db/<modulo>_repo.rs con funciones sobre &Connection: list, get, insert,
   update, delete, count y las consultas agregadas.
4. **Servicio** en src/services/<modulo>_service.rs con errores thiserror, validaciones de
   dominio y transacciones; el servicio decide eliminar vs desactivar.
5. **Estado** en src/app_<modulo>.rs: campos en App, refresh_<modulo>, open/close de formularios,
   guardar y borrar.
6. **Pantallas** en src/ui/screens/<modulo>.rs: draw_list, draw_detail, draw_form con los widgets
   de src/ui/widgets (form, detail, hit, scroll).
7. **Enumerados**: agregar la variante a Module (label, short_label, key, description, ALL) y las
   pantallas a Screen; sumar el dispatch en ui/mod.rs y el match de teclas en app.rs.
8. **Teclas y footer**: n/e/d/Enter/f/o///r como el resto; agregar la pista al footer y a Ayuda.
9. **Pruebas**: unitarias de validación y cálculo; integración de flujo de teclado con base en
   memoria (ver tests/ui_smoke.rs y tests/tui_widgets.rs).
10. **Capturas y docs**: ejemplo en examples/capturar_tui.rs, captura en docs/img/, sección en
    docs/TUI.md y entrada en README.

**Reglas de oro**

- La UI no ejecuta SQL: todo pasa por App → service → repo.
- Dinero en centavos (i64), fechas ISO (chrono), enums con CHECK, FK activas.
- Migraciones idempotentes y con backup previo (ya lo hace Database::open).
- Ningún panic por entrada inválida: validar en dominio y mostrar Modal::Error.
- Pruebas con base en memoria para repos y servicios; pruebas de teclado para pantallas.
- Rendimiento: listados < 100 ms con 10.000 partidos; el render no consulta la base.

---

### 8.1 Ola 0 — Hábito y fundaciones

#### IMP-01 · Dashboard "Hoy" · P0 · M

**Qué**: pantalla de inicio con el contexto del día: próximo partido (o el último), racha actual,
totales del mes, lesiones activas, alerta de presupuesto, recordatorios próximos, logro más
cercano y acciones rápidas.

**Datos**: solo lectura sobre partido, lesion, financiamiento, presupuesto_mensual y (cuando
existan) recordatorio y logro. No necesita tabla nueva en la primera versión.

**Cómo**:

- Dominio: InicioResumen { proximo: Option<PartidoRow>, ultimo, racha: RachaTipo, mes: MesResumen,
  lesiones_activas, alerta_presupuesto: Option<AlertaPresupuesto>, logro_cercano, acciones }.
- Repo: src/db/inicio_repo.rs con 6 consultas independientes (una por tarjeta) y límites.
- Servicio: inicio_service::resumen(conn) que compone el DTO y tolera tablas vacías.
- App: Screen::Inicio como pantalla inicial; Module::Inicio (label "Inicio", key "inicio").
- UI: grilla 2x3 de tarjetas con Gauge/Sparkline/Paragraph y una columna de acciones rápidas;
  en pantallas angostas se apila.
- Teclas: Enter abre la tarjeta; n nuevo partido; c calendario; g vuelve a Inicio; Ctrl+K paleta.

**Aceptación**: abre con base vacía sin errores; carga < 50 ms; sin SQL en ui; prueba de teclado
que recorre las tarjetas y abre Partidos.

#### IMP-02 · Calendario mensual · P0 · M

**Qué**: grilla de 7 columnas × 6 filas con el mes, marcas en los días con partidos, movimientos
financieros y (luego) recordatorios; panel de detalle del día seleccionado.

**Datos**: eventos derivados de partido (fecha_hora) y financiamiento (fecha); no requiere tabla.
Opcional: vista de "semana" y "agenda".

**Cómo**:

- Dominio: EventoCalendario { fecha: NaiveDate, tipo: EventoTipo, titulo, referencia_id }.
- Repo: calendario_repo::eventos_mes(conn, year, month) -> Vec<EventoCalendario>, con dos
  consultas y agrupación en memoria.
- App: CalendarioState { year, month, selected_day, eventos }; Screen::Calendario.
- UI: Layout con grilla (Constraint::Ratio) + panel lateral; cabecera con mes y año; días con
  color de acento si tienen eventos; hoy con borde.
- Teclas: h/l día, j/k semana, n/p mes, t hoy, Enter abre el evento (detalle de partido o
  transacción), n crea partido en el día, Esc vuelve.

**Aceptación**: meses de 28/29/30/31 días correctos; navegación entre años; con datos reales
marca los días; prueba de límites (enero/diciembre, bisiesto).

#### IMP-03 · Búsqueda global y paleta de comandos · P0 · M

**Qué**: una tecla abre una búsqueda transversal (partidos, camisetas, canchas, equipos,
lesiones, movimientos, torneos, notas) y otra abre una paleta de acciones (crear partido, ir a
calendario, exportar, abrir ajustes) con filtro difuso.

**Datos**: consultas LIKE por entidad; a futuro, tabla virtual FTS5 sobre textos.

**Cómo**:

- Dominio: ResultadoBusqueda { tipo, id, titulo, subtitulo, score }.
- Repo: busqueda_repo::buscar(conn, q, limite) con UNION ALL de consultas por tabla y orden por
  coincidencia exacta > prefijo > contiene.
- Servicio: busqueda_service::buscar y paleta_service::acciones() (lista estática de comandos).
- App: Overlay::Busqueda { query, resultados, seleccion } y Overlay::Paleta.
- UI: modal centrado con input, lista de resultados agrupada por tipo y contador; Enter navega.
- Teclas: Ctrl+K paleta (global), / búsqueda (global), flechas y Enter, Esc cierra.

**Aceptación**: desde cualquier pantalla, Ctrl+K abre y filtra; elegir "Nuevo partido" abre el
wizard; la búsqueda encuentra un partido por rival, una camiseta por nombre y una cancha por
localidad; sin coincidencias muestra estado vacío.

#### IMP-04 · Logros · P0 · M

**Qué**: catálogo de logros con progreso, estados y desbloqueo; notificación al completar.

**Datos**: logros definidos en código (tabla estática); opcional tabla logro_desbloqueado para
guardar la fecha del primer desbloqueo.

~~~sql
CREATE TABLE IF NOT EXISTS logro_desbloqueado (
    logro_id TEXT PRIMARY KEY,
    fecha    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
~~~

**Cómo**:

- Dominio: LogroDef { id, nombre, descripcion, tipo, objetivo, categoria, nivel } y
  LogroProgreso { def, valor, completado }.
- Repo: logros_repo::valores(conn, camiseta_id) con un match por tipo y consultas preparadas.
- Servicio: logros_service::evaluar(conn, camiseta_id) -> Vec<LogroProgreso>; al guardar un
  partido, comparar con logro_desbloqueado e insertar los nuevos.
- UI: Screen::Logros con tabs (Todos / Completados / En progreso / Bloqueados), barra de progreso
  por fila y detalle; filtro por categoría.
- Teclas: h/l cambia tab, / busca, Enter detalle, x exporta.

**Aceptación**: con 3 partidos de prueba, "Primer Gol" y "Debutante" se completan; el progreso
coincide con SQL; al desbloquear aparece una notificación (IMP-06) y se guarda la fecha.

#### IMP-05 · Records & Rankings · P0 · M

**Qué**: 16 reportes históricos: récord de goles, de asistencias, mejor/peor combinación
cancha+camiseta, mejor/peor temporada, mejor/peor rendimiento, partidos sin goles/asistencias,
mejor racha goleadora, peor racha, partidos consecutivos anotando, top 5 y ranking de camisetas.

**Datos**: consultas agregadas sobre partido (y torneo/temporada cuando existan).

**Cómo**:

- Repo: records_repo con una función por reporte; devolver filas tipadas, no strings.
- Servicio: records_service::todos(conn) para la vista de tarjetas.
- UI: Screen::Records con lista de reportes a la izquierda y detalle a la derecha; Enter abre el
  partido implicado.
- Teclas: j/k, Enter, x exporta el reporte.

**Aceptación**: con datos sintéticos, cada récord devuelve la fila esperada; empates resueltos de
forma determinista; reporte vacío muestra "sin datos".

#### IMP-06 · Centro de notificaciones · P0 · M

**Qué**: bandeja de avisos generados por reglas (lesión activa, presupuesto excedido, próximo
partido, racha, logro desbloqueado, recordatorio vencido) con severidad y estado leído.

**Datos**: tabla notificacion (o derivación en vivo + tabla de descartes). Recomendado table:

~~~sql
CREATE TABLE IF NOT EXISTS notificacion (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo       TEXT NOT NULL,
    severidad  INTEGER NOT NULL DEFAULT 0,
    titulo     TEXT NOT NULL,
    mensaje    TEXT NOT NULL,
    leida      INTEGER NOT NULL DEFAULT 0,
    entidad    TEXT NOT NULL DEFAULT '',
    entidad_id INTEGER,
    fecha      TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
~~~

**Cómo**:

- Servicio: notificacion_service::generar(conn) idempotente (clave tipo + entidad + id) y
  marcar_leida / marcar_todas.
- Reglas iniciales: lesiones ACTIVA de los últimos 7 días; presupuesto del mes por encima del
  límite; partido en las próximas 24 h; logro desbloqueado; racha de 3+ victorias.
- UI: Screen::Notificaciones (lista con icono de severidad) y badge en el header (! 3).
- Teclas: ! abre, m marca leída, M todas, Enter navega a la entidad.

**Aceptación**: no duplica avisos al reabrir; el badge refleja no leídas; un partido próximo
genera aviso; marcar todas funciona.

#### IMP-07 · Undo e historial de acciones · P0 · M

**Qué**: deshacer la última operación de escritura (crear/editar/borrar) con confirmación y
historial visible; alternativa a "preguntar antes de borrar".

**Datos**: no requiere tabla: pila en memoria + archivo opcional data/undo.json (máx. 50).

**Cómo**:

- Dominio: UndoEntry { tabla, registro_id, operacion, descripcion, antes: String, despues:
  String, ts }.
- Servicio: undo_service con snapshot(conn, tabla, id) genérico (SELECT * → JSON) y
  aplicar(conn, entry) con whitelist de tablas y orden de restauración.
- Integración: en cada service de escritura, registrar antes/después (helper común en
  service_utils).
- UI: Ctrl+Z abre modal de confirmación con descripción; Screen::UndoHistorial lista y permite
  deshacer una entrada concreta.
- Teclas: Ctrl+Z deshacer, Ctrl+Shift+Z historial, Esc cancelar.

**Aceptación**: crear un partido y deshacerlo lo elimina con su transacción financiera; editar
una camiseta y deshacer restaura los campos; borrar y deshacer reinserta con el mismo id; 50
entradas máximo; prueba con FK.

#### IMP-08 · Exportación filtrada multiformato · P0 · S

**Qué**: exportar el listado visible (respetando filtro, orden y búsqueda) a CSV, JSON, HTML o
Markdown en la carpeta Exportaciones, con nombre fechado.

**Datos**: los DTO de cada listado; no toca la base.

**Cómo**:

- Servicio: export_service::exportar<T: Serialize>(filas, columnas, formato, nombre) y
  export_service::html(titulo, tabla) reutilizando la plantilla del resumen.
- App: en cada listado, tecla x abre modal de formato; usa el filtro activo.
- UI: modal con 4 opciones y vista previa del conteo; toast con la ruta generada.

**Aceptación**: exportar camisetas filtradas por "activas" produce solo esas filas; HTML escapa
nombres con & y <; archivo se crea en la carpeta correcta; CSV abre en Excel con UTF-8.

#### IMP-09 · Tutorial y primera ejecución · P0 · S

**Qué**: onboarding de 3–5 pasos que crea lo mínimo para registrar el primer partido y explica
las teclas; se puede repetir desde Ayuda.

**Datos**: columna settings.tutorial_completado (migración) o tabla custom_menus reutilizada.

**Cómo**:

- UI: Screen::Tutorial con pasos ilustrados (texto + tecla), barra de progreso y botones
  "Crear cancha", "Crear camiseta", "Crear equipo", "Registrar partido".
- App: primera ejecución (settings.tutorial_completado = 0) abre el tutorial; Esc lo saltea;
  al terminar marca 1.
- Ayuda: entrada "Repetir tutorial".

**Aceptación**: base nueva abre tutorial; completar los 4 accesos deja datos mínimos; se puede
saltear y repetir; no bloquea el resto de la app.

#### IMP-10 · Atajos globales y ayuda · P0 · S

**Qué**: un mapa de teclas coherente y global, ayuda actualizada y descubrible.

**Cómo**:

- App: KeyAction { Nuevo, Calendario, Buscar, Paleta, Inicio, Undo, Ayuda, Salir } resuelto antes
  del handler de pantalla, con prioridad: modal > búsqueda > formulario > global > pantalla.
- Teclas propuestas: g inicio, n nuevo (contextual), c calendario, / buscar, Ctrl+K paleta,
  Ctrl+Z undo, ? ayuda, q salir.
- UI: pantalla de Ayuda con tabla de teclas por contexto y enlace al tutorial.

**Aceptación**: las teclas funcionan desde cualquier pantalla (salvo conflicto con input); la
ayuda lista todas; prueba de precedencia.

---

### 8.2 Ola 1 — Gestión deportiva

#### IMP-11 · Torneos · P1 · L

**Qué**: CRUD de torneos, asociación de equipos, fixture automático, carga de resultados, tabla
de posiciones, goleadores, dashboard y exportación. Es el módulo más grande de esta ola.

**Datos**: las tablas torneo, partido_torneo y equipo_historial ya existen. Faltan:

~~~sql
CREATE TABLE IF NOT EXISTS torneo_fase (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    torneo_id  INTEGER NOT NULL REFERENCES torneo(id) ON DELETE CASCADE,
    nombre     TEXT NOT NULL,
    orden      INTEGER NOT NULL DEFAULT 1,
    tipo       TEXT NOT NULL DEFAULT 'grupos'
);

CREATE TABLE IF NOT EXISTS equipo_torneo (
    torneo_id INTEGER NOT NULL REFERENCES torneo(id) ON DELETE CASCADE,
    equipo_id INTEGER NOT NULL REFERENCES equipo(id) ON DELETE CASCADE,
    PRIMARY KEY (torneo_id, equipo_id)
);

CREATE TABLE IF NOT EXISTS equipo_torneo_estadisticas (
    torneo_id        INTEGER NOT NULL,
    equipo_id        INTEGER NOT NULL,
    partidos_jugados INTEGER NOT NULL DEFAULT 0,
    ganados          INTEGER NOT NULL DEFAULT 0,
    empatados        INTEGER NOT NULL DEFAULT 0,
    perdidos         INTEGER NOT NULL DEFAULT 0,
    goles_favor      INTEGER NOT NULL DEFAULT 0,
    goles_contra     INTEGER NOT NULL DEFAULT 0,
    puntos           INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (torneo_id, equipo_id)
);
~~~

**Cómo**:

- Dominio: TipoTorneo, FormatoTorneo, Torneo, FaseTorneo, FixturePartido, StandingRow.
- Repo: torneo_repo (CRUD + equipos) y fixture_repo (insertar partidos, resultados, tabla,
  goleadores).
- Servicio: crear_torneo valida 4–32 equipos; generar_fixture usa el método del círculo en una
  transacción; registrar_resultado actualiza partido_torneo y **recalcula** la tabla desde cero
  (idempotente y simple).
- UI: Screen::TorneosList, TorneoDetail con tabs Resumen/Fixture/Tabla/Goleadores, formularios de
  alta y de resultado.
- Teclas: n nuevo, Enter detalle, g genera fixture (con confirmación), r resultado, t tabla,
  x exporta.

**Aceptación**: 6 equipos → 15 partidos (ida) o 30 (ida y vuelta); 7 equipos → 21; tabla
coherente tras 3 resultados; recalcular no duplica; borrar torneo limpia partidos y estadísticas;
pruebas de fixture par/impar y de posiciones con empate.

#### IMP-12 · Temporadas · P1 · L

**Qué**: ciclos deportivos con fases, torneos asociados, fatiga, evolución, resumen y
comparación.

**Datos**: temporada ya existe; agregar:

~~~sql
CREATE TABLE IF NOT EXISTS temporada_fase (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    temporada_id  INTEGER NOT NULL REFERENCES temporada(id) ON DELETE CASCADE,
    nombre        TEXT NOT NULL,
    tipo          TEXT NOT NULL DEFAULT 'regular',
    fecha_inicio  TEXT NOT NULL DEFAULT '',
    fecha_fin     TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS torneo_temporada (
    torneo_id    INTEGER NOT NULL REFERENCES torneo(id) ON DELETE CASCADE,
    temporada_id INTEGER NOT NULL REFERENCES temporada(id) ON DELETE CASCADE,
    PRIMARY KEY (torneo_id, temporada_id)
);

CREATE TABLE IF NOT EXISTS temporada_resumen (
    temporada_id  INTEGER PRIMARY KEY REFERENCES temporada(id) ON DELETE CASCADE,
    partidos      INTEGER NOT NULL DEFAULT 0,
    goles         INTEGER NOT NULL DEFAULT 0,
    promedio      REAL NOT NULL DEFAULT 0,
    lesiones      INTEGER NOT NULL DEFAULT 0,
    campeon_id    INTEGER,
    goleador      TEXT NOT NULL DEFAULT '',
    generado_en   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
~~~

**Cómo**:

- Servicio: crear_temporada + fases por defecto (Pretemporada, Regular, Postemporada);
  asociar_torneo; calcular_fatiga (partidos en los últimos 14 días × 10, tope 100);
  generar_resumen (agrega partidos y lesiones del rango); comparar(a, b) -> Vec<Comparacion>.
- UI: Screen::TemporadasList y TemporadaDetail con tabs Resumen/Fases/Torneos/Fatiga/Comparar;
  LineChart para evolución y Gauge para fatiga.
- Teclas: n, Enter, a administrar, c comparar, x exportar.

**Aceptación**: una temporada con 10 partidos genera resumen correcto; fatiga de 3 partidos en
14 días = 30; comparación muestra diferencias; fases por defecto creadas una sola vez.

#### IMP-13 · Estadísticas de canchas (10 reportes) · P1 · M

**Qué**: resumen general, más/menos usadas, rendimiento, goles/asistencias, por tipo, por
superficie, techadas vs descubiertas, por localidad y por franja horaria.

**Cómo**:

- Extender estadisticas_repo con 10 funciones que agrupan partido por cancha_id y cruzan con
  cancha (tipo, superficie, techada, localidad); franja horaria con substr(fecha_hora, 12, 2).
- Agregar variantes a EstadisticaKind (o un submenú Canchas dentro de Estadísticas).
- UI: tabla + BarChart horizontal; Enter abre la cancha.

**Aceptación**: los totales cierran con el conteo de partidos; canchas sin uso aparecen en "menos
usadas"; techada/descubierta respeta el código 0/1/2.

#### IMP-14 · Comparador A/B · P1 · M

**Qué**: comparar dos entidades o dos periodos con las mismas métricas (partidos, goles,
asistencias, rendimiento, winrate, rachas).

**Cómo**:

- Servicio: comparador_service::comparar(conn, Entidad, a, b) -> Comparacion { filas: Vec<Fila> }
  donde Fila { metrica, a, b, mejor }.
- Entidades: camiseta, cancha, equipo, torneo, periodo (desde/hasta) y condición (clima).
- UI: dos selectores + tabla lado a lado con barras y resaltado del ganador.

**Aceptación**: comparar una camiseta con otra da métricas verificables; periodos sin datos
muestran 0; se puede exportar.

#### IMP-15 · Meta-análisis extendido · P1 · M

**Qué**: consistencia, outliers, dependencia del contexto, impacto del cansancio/ánimo,
eficiencia, rendimiento por intensidad/dolor/arbitraje/temperatura y tendencia.

**Cómo**:

- Repo: consultas con AVG, SQRT(AVG(x*x) − AVG(x)*AVG(x)), MIN/MAX y GROUP BY.
- Servicio: meta_service::reporte(kind) -> ReporteMeta { filas, interpretacion }.
- UI: dentro de Estadísticas → Rendimiento, sublista "Meta-análisis" con 10 reportes; cada uno
  muestra tabla y una frase de lectura ("tu rendimiento cae 1,2 puntos con cansancio alto").

**Aceptación**: con datos sintéticos, desviación y CV coinciden con el cálculo manual; outliers
= fuera de media ± 2σ; interpretación no divide por cero.

#### IMP-16 · Submenú "Último" y tiempo transcurrido · P1 · S

**Qué**: última victoria, derrota, empate, gol y asistencia, con tiempo transcurrido en años,
meses, días, horas y minutos; acceso rápido desde Partidos.

**Cómo**:

- Repo: una consulta por caso (ORDER BY fecha_hora DESC LIMIT 1 con filtro).
- Dominio: formatear_transcurrido(desde, ahora) -> String (reutilizable en otras pantallas).
- UI: Screen::PartidosUltimo con 5 tarjetas; Enter abre el partido.
- Teclas: u desde el listado de partidos.

**Aceptación**: fechas de prueba producen textos correctos ("hace 2 años, 3 meses…"); sin datos
muestra "todavía no hay".

---

### 8.3 Ola 2 — Persona y contexto

#### IMP-17 · Bienestar · P2 · L

**Qué**: objetivos y planes, hábitos diarios, sesiones mentales, entrenamientos y ejercicios,
alimentación, salud y asistente de rutinas con score/riesgo.

**Datos**: 10–14 tablas nuevas; se puede entregar en dos fases (hábitos + entrenamiento + salud,
y luego nutrición + mental + asistente).

**Cómo**:

- Migración 005 con las tablas del apéndice B (versión simplificada: objetivo, plan, habito,
  entrenamiento, ejercicio, comida, dia_nutricional, sesion_mental, salud, control_medico,
  recomendacion).
- Dominio y repo por subdominio; servicio con las fórmulas de §4.3 (prep_score ponderado y
  riesgo).
- UI: Screen::Bienestar con hub de 6 tarjetas; cada una abre lista/formulario; el asistente
  muestra dos Gauge y la rutina.
- Teclas: n nuevo registro, Enter detalle, a asistente.

**Aceptación**: el score coincide con la fórmula para un fixture; con lesión activa el riesgo es
≥ 70; el asistente guarda la recomendación; las tablas vacías no rompen el hub.

#### IMP-18 · Recordatorios y agenda · P2 · M

**Qué**: recordatorios con fecha, hora, temática y recurrencia; agenda que mezcla recordatorios y
partidos; avisos en el dashboard y el calendario.

**Datos**:

~~~sql
CREATE TABLE IF NOT EXISTS recordatorio (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    titulo        TEXT NOT NULL,
    nota          TEXT NOT NULL DEFAULT '',
    fecha         TEXT NOT NULL,
    hora          TEXT NOT NULL DEFAULT '',
    periodicidad  INTEGER NOT NULL DEFAULT 0 CHECK (periodicidad BETWEEN 0 AND 3),
    tematica      TEXT NOT NULL DEFAULT '',
    completado    INTEGER NOT NULL DEFAULT 0,
    ultima_gen    TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
~~~

**Cómo**:

- Servicio: crear/editar/listar/filtrar; al listar, generar las ocurrencias vencidas según
  periodicidad (0 una vez, 1 diaria, 2 semanal, 3 mensual) y actualizar ultima_gen.
- UI: Screen::Recordatorios (lista + formulario) y Screen::Agenda (recordatorios + partidos
  ordenados por fecha, con secciones Hoy / Próximos / Pasados).
- Integración: calendario marca días con recordatorios; dashboard muestra los de hoy.

**Aceptación**: un recordatorio diario genera la ocurrencia de hoy una sola vez; mensual respeta
fin de mes; la agenda ordena bien; borrar no deja huérfanos.

#### IMP-19 · Colecciones e inventario · P2 · M

**Qué**: inventario de equipamiento (camisetas, botines, pelotas, accesorios), colecciones y
asociaciones; historial de equipamiento; backup JSON.

**Datos**: coleccion, inventario_item, coleccion_inventario, equipamiento_historial (modelo del
original, adaptado a centavos e ISO).

**Cómo**:

- Servicio: CRUD de ítems; sincronizar camisetas (upsert por camiseta_id, sin duplicar); crear
  colección y asociar/quitar; exportar/importar JSON.
- UI: Screen::Colecciones con tabs Inventario / Colecciones / Historial; detalle con ítems.

**Aceptación**: sincronizar 14 camisetas crea 14 ítems y no duplica al repetir; quitar un ítem no
borra el inventario; el backup reimporta sin perder relaciones.

#### IMP-20 · Tiendas · P2 · S

**Qué**: catálogo de tiendas con contacto, redes, productos, rango de precio, envío, rating y
favorito.

**Cómo**: tabla tienda (20 columnas del original), CRUD, filtros por tipo/favorito y búsqueda;
detalle con enlaces. UI: lista + detalle + formulario.

**Aceptación**: alta con campos opcionales; filtro por "vende botines"; favorito persistente.

#### IMP-21 · Referencias multimedia · P2 · S

**Qué**: video, foto, artículo o audio asociado a un partido, con título, URL y descripción.

**Cómo**: tabla media (partido_id, tipo, titulo, url, descripcion, fecha); CRUD; filtro por tipo;
Enter abre la URL en el navegador (o muestra la ruta). Enlace desde el detalle de partido.

**Aceptación**: alta de un video y apertura; filtro por tipo; borrar partido deja media con
partido_id NULL (o cascada, decidir).

#### IMP-22 · Ranking de amigos · P2 · S

**Qué**: amigos con totales manuales, ranking con el usuario local y comparación directa.

**Cómo**: tabla amigo; servicio que calcula las stats locales desde partido y ordena; UI con
tabla y comparación lado a lado.

**Aceptación**: el usuario local aparece; orden por goles; comparación muestra diferencias.

#### IMP-23 · Progresión de jugadores · P2 · M

**Qué**: planes de entrenamiento y mediciones semanales por skill (ataque, defensa, resistencia,
velocidad, técnica).

**Cómo**: tablas entrenamiento_plan y progresion_jugador (ya existen); CRUD de planes y
mediciones; LineChart por skill; comparación entre semanas.

**Aceptación**: 4 semanas generan una línea; valores 0–100 validados; no se duplica la semana.

#### IMP-24 · Química entre jugadores · P2 · M

**Qué**: mejores duplas por winrate, con datos automáticos (equipos/partidos) o manuales
(goles, asistencias, rating, comentario).

**Cómo**: tabla quimica_jugador_estadistica; servicio que calcula la mejor combinación y permite
CRUD manual; UI de ranking y detalle.

**Aceptación**: una dupla con 3 partidos y 2 victorias da 66 %; el modo híbrido no duplica;
editar/eliminar actualiza el ranking.

---

### 8.4 Ola 3 — Inteligencia y ecosistema

#### IMP-25 · Entrenador IA · P2 · L

**Qué**: motor de reglas que evalúa el estado, genera consejos por severidad, predice resultado,
recomienda formación, alerta rendimiento, sugiere descanso y detecta puntos débiles.

**Datos**: ia_config (nivel, última alerta, último riesgo), consejos_historial (fecha, consejo,
seguido), perfil_usuario_ia (aceptados, ignorados, índice).

**Cómo**:

- Servicio: ia_service::estado(conn) con las fórmulas de §4.1; generar_consejos(estado, nivel) con
  las 5 reglas; cooldown de 24 h; predecir con promedios de local/visitante y contexto;
  formacion con jugadores disponibles por posición.
- UI: Screen::EntrenadorIA con tarjetas por severidad (info/advertencia/crítico), historial y
  configuración de nivel; modal antes de guardar un partido si el nivel lo permite.
- Teclas: 1–4 nivel, h historial, Enter detalle.

**Aceptación**: los umbrales se respetan; el nivel 0 no muestra nada; el aviso no se repite antes
de 24 h; el índice de prudencia se actualiza al responder "seguí/no seguí".

#### IMP-26 · Clima automático (Open-Meteo) · P2 · M

**Qué**: completar temperatura, sensación, precipitación, viento y código WMO al registrar un
partido, usando las coordenadas de la cancha; proceso por lotes para históricos.

**Datos**: columnas de partido ya existentes (temperatura_c, precip_mm, wind_kmh, weather_code,
apparent_temp_c, clima_json) y cancha.latitud/longitud.

**Cómo**:

- Dependencia: ureq (bloqueante, rustls) + serde_json.
- Servicio: clima_service::obtener(lat, lon, fecha_hora) -> Result<Clima, ClimaError>; valida
  fecha no futura, arma la URL, parsea hourly y elige la hora; mapea WMO a texto.
- Integración: en el formulario de partido, tecla w "obtener clima" y aviso si falta coordenada;
  opción por lotes en Ajustes o Partidos.
- UI: spinner/estado en el formulario; columnas en el detalle.

**Aceptación**: con coordenadas de Buenos Aires y una fecha pasada devuelve datos; fecha futura
rechazada; sin red muestra error claro y no bloquea el guardado; prueba con JSON fixture.

#### IMP-27 · Sincronización con PocketBase · P3 · L

**Qué**: subir/bajar partidos y catálogos contra un servidor PocketBase, con cola, token,
conflictos y auto-sync.

**Datos**: sync_config, sync_dirty, sync_conflicto; uuid y actualizado_en en partido.

**Cómo**:

- Servicio: sync_service con ureq; login/auth-refresh; búsqueda por uuid; push/pull; cola
  transaccional; resolución de conflictos (local/remoto) en UI.
- Seguridad: token en tabla local; no loguear; TLS obligatorio.
- UI: Screen::Sync con estado, pendientes, conflictos y configuración.

**Aceptación**: dos bases sincronizadas convergen; un conflicto se resuelve y no se pierde; sin
token muestra "iniciá sesión"; auto-sync respeta el flag.

#### IMP-28 · PDF, XLSX y ODS · P2 · L

**Qué**: informes PDF por módulo y total, planillas XLSX/ODS con una hoja por módulo.

**Cómo**:

- PDF: printpdf con Helvetica; helper pdf::documento(titulo, secciones) que maneja saltos de
  página, tablas y UTF-8; portada con fecha.
- XLSX: rust_xlsxwriter; una hoja por entidad, encabezados con estilo y autofiltro.
- ODS: generar XML plano + ZIP (crate zip) o spreadsheet-ods; empezar por XLSX.
- UI: modal Exportar con formatos y progreso; archivos en Exportaciones.

**Aceptación**: PDF abre y respeta acentos; XLSX tiene hojas y filas correctas; nombres de
archivo únicos; error claro si la carpeta no se puede escribir.

#### IMP-29 · Multiperfil y contraseña · P3 · M

**Qué**: varios perfiles locales, cada uno con su base; contraseña opcional; pantalla de login.

**Cómo**:

- Tabla usuario en una base compartida (users.db) o en la base principal; hash argon2 + salt.
- main: si hay perfiles, mostrar Screen::Login antes de abrir la base del perfil; recordar último.
- UI: lista de perfiles, crear, borrar con confirmación, cambiar contraseña.

**Aceptación**: dos perfiles tienen datos separados; contraseña incorrecta no entra; sin
perfiles se crea el primero; no se puede borrar el último.

#### IMP-30 · i18n y accesibilidad · P2 · L

**Qué**: español/inglés real (menús, etiquetas, footer, mensajes) y accesibilidad (tamaño de
texto, alto contraste, foco visible).

**Cómo**:

- Módulo util/i18n.rs con enum Language y función t(key); claves en un mapa estático; tests que
  verifican que ambas lenguas tienen las mismas claves.
- Aplicar en menú, tabs, footer, formularios y mensajes; los datos del usuario no se traducen.
- Tamaño: escalar padding/altura de fila y tipografía del menú; alto contraste como tema.

**Aceptación**: cambiar a inglés traduce el chrome; ninguna clave falta; el layout no se rompe
con texto más largo; el tema de alto contraste cumple contraste mínimo.

#### IMP-31 · Auto-actualización · P3 · S

**Qué**: comprobar la última release en GitHub, comparar versión y ofrecer descargar/instalar.

**Cómo**: ureq a la API de releases; comparar semver; descargar el instalador a temp y lanzarlo
con confirmación; opción "solo avisar".

**Aceptación**: versión igual no avisa; versión mayor avisa; sin red no molesta; nunca ejecuta
sin confirmación.

#### IMP-32 · Simulador de partido · P3 · M

**Qué**: partido simulado entre equipos guardados, con eventos aleatorios, marcador y animación
ASCII en la cancha; opción de guardarlo.

**Cómo**: dominio puro con RNG inyectable (rand); ui/screens/simulador.rs con tachyonfx para la
animación; al final, ofrece guardar como partido.

**Aceptación**: 100 simulaciones no panican; el marcador es coherente; se puede cancelar; guardar
crea el partido con los datos generados.

---

### 8.5 Resumen del catálogo

| ID | Mejora | Ola | Prioridad | Esfuerzo | Depende de |
|---|---|---:|---:|---:|---|
| IMP-01 | Dashboard "Hoy" | 0 | P0 | M | — |
| IMP-02 | Calendario mensual | 0 | P0 | M | — |
| IMP-03 | Búsqueda global + paleta | 0 | P0 | M | — |
| IMP-04 | Logros | 0 | P0 | M | — |
| IMP-05 | Records & Rankings | 0 | P0 | M | — |
| IMP-06 | Notificaciones | 0 | P0 | M | — |
| IMP-07 | Undo | 0 | P0 | M | — |
| IMP-08 | Export filtrado | 0 | P0 | S | — |
| IMP-09 | Tutorial | 0 | P0 | S | — |
| IMP-10 | Atajos globales | 0 | P0 | S | — |
| IMP-11 | Torneos | 1 | P1 | L | IMP-02 |
| IMP-12 | Temporadas | 1 | P1 | L | IMP-11 |
| IMP-13 | Stats de canchas | 1 | P1 | M | — |
| IMP-14 | Comparador A/B | 1 | P1 | M | — |
| IMP-15 | Meta-análisis | 1 | P1 | M | — |
| IMP-16 | Submenú "Último" | 1 | P1 | S | — |
| IMP-17 | Bienestar | 2 | P2 | L | IMP-01 |
| IMP-18 | Recordatorios | 2 | P2 | M | IMP-02 |
| IMP-19 | Colecciones | 2 | P2 | M | — |
| IMP-20 | Tiendas | 2 | P2 | S | — |
| IMP-21 | Multimedia | 2 | P2 | S | — |
| IMP-22 | Ranking amigos | 2 | P2 | S | — |
| IMP-23 | Progresión | 2 | P2 | M | — |
| IMP-24 | Química | 2 | P2 | M | — |
| IMP-25 | Entrenador IA | 3 | P2 | L | IMP-06 |
| IMP-26 | Clima Open-Meteo | 3 | P2 | M | — |
| IMP-27 | Sync PocketBase | 3 | P3 | L | IMP-07 |
| IMP-28 | PDF/XLSX/ODS | 3 | P2 | L | IMP-08 |
| IMP-29 | Multiperfil | 3 | P3 | M | IMP-30 |
| IMP-30 | i18n y accesibilidad | 3 | P2 | L | — |
| IMP-31 | Auto-actualización | 3 | P3 | S | — |
| IMP-32 | Simulador | 3 | P3 | M | IMP-11 |

---

## 9. Flujo de uso propuesto para MiFutbolR

La meta no es sumar 25 menús, sino **cambiar el centro de gravedad**: que la app abra en el
contexto del usuario ("qué pasa hoy") y que cualquier acción importante esté a una o dos teclas.
La navegación por módulos sigue existiendo, pero deja de ser la única puerta.

### 9.1 Principios de diseño

1. **Contexto primero**: el inicio responde "¿qué jugué?, ¿qué viene?, ¿cómo vengo?".
2. **Dos teclas al destino**: cualquier módulo o entidad frecuente se alcanza con un atajo o con
   la paleta (Ctrl+K), sin recorrer menús.
3. **Una pantalla, un intento**: registrar un partido, revisar la semana y planificar no comparten
   pantalla; cada flujo tiene su entrada clara.
4. **Acciones reversibles**: toda escritura ofrece undo; borrar deja de dar miedo.
5. **Progresión visible**: logros, rachas y récords se muestran cuando importan, no escondidos.
6. **Offline primero**: todo funciona sin red; clima, sync y actualización son mejoras opcionales.
7. **Densidad con aire**: más datos sin perder legibilidad; tarjetas, tabs y paneles.
8. **Consistencia**: los patrones de listado/detalle/formulario se mantienen en todos los módulos.

### 9.2 Nueva estructura de navegación

Con 24–26 módulos, una lista plana deja de funcionar. Propuesta: **sidebar agrupado** (≥120
columnas), **tabs** (70–119) y **menú agrupado** (<70).

| Grupo | Módulos |
|---|---|
| Inicio | Hoy (dashboard) |
| Deporte | Partidos · Torneos · Temporadas · Estadísticas · Records · Logros |
| Recursos | Camisetas · Canchas · Botines · Equipos · Colecciones · Tiendas · Multimedia |
| Persona | Calendario · Recordatorios · Bienestar · Carrera · Entrenador IA · Amigos |
| Datos | Resumen · Exportar/Importar · Sincronización |
| Sistema | Ajustes · Ayuda · Tutorial |

El sidebar muestra el grupo como encabezado y los módulos como filas; el módulo activo se
resalta. Los grupos se pueden colapsar (Enter sobre el encabezado) y el estado se recuerda en
settings. En pantallas angostas se usa el menú de Inicio con la misma agrupación.

~~~mermaid
flowchart TD
    A[Inicio / Hoy] --> B[Deporte]
    A --> C[Recursos]
    A --> D[Persona]
    A --> E[Datos]
    A --> F[Sistema]
    B --> B1[Partidos]
    B --> B2[Torneos]
    B --> B3[Temporadas]
    B --> B4[Estadísticas]
    B --> B5[Records]
    B --> B6[Logros]
    C --> C1[Camisetas]
    C --> C2[Canchas]
    C --> C3[Botines]
    C --> C4[Equipos]
    C --> C5[Colecciones]
    C --> C6[Tiendas]
    C --> C7[Multimedia]
    D --> D1[Calendario]
    D --> D2[Recordatorios]
    D --> D3[Bienestar]
    D --> D4[Carrera]
    D --> D5[Entrenador IA]
    D --> D6[Amigos]
    E --> E1[Resumen]
    E --> E2[Exportar]
    E --> E3[Sync]
    F --> F1[Ajustes]
    F --> F2[Ayuda]
~~~

**Compatibilidad**: los atajos actuales (1-9 en el menú, n/e/d, /, ?) se conservan. Los nuevos
se suman sin robar contexto: si el foco está en un input, las teclas van al input; si no, se
resuelven como acciones globales.

### 9.3 Mapa de teclas propuesto

| Contexto | Tecla | Acción |
|---|---|---|
| Global | Ctrl+K | Paleta de comandos (acciones + entidades) |
| Global | / | Búsqueda global |
| Global | g | Ir a Inicio (Hoy) |
| Global | c | Calendario |
| Global | n | Nueva entidad del contexto (partido en Partidos, camiseta en Camisetas…) |
| Global | Ctrl+Z | Deshacer última acción |
| Global | ? / F1 | Ayuda contextual |
| Global | q | Salir (con confirmación si hay formulario abierto) |
| Listado | j/k o ↑↓ | Mover selección |
| Listado | Enter | Abrir detalle |
| Listado | e / d | Editar / borrar |
| Listado | f / o | Filtro / orden |
| Listado | x | Exportar el listado filtrado |
| Listado | r | Refrescar |
| Formulario | Tab / Shift+Tab | Campo siguiente / anterior |
| Formulario | Ctrl+S | Guardar |
| Formulario | Esc | Cancelar (con confirmación si hay cambios) |
| Detalle | v | Ver imagen (si existe) |
| Detalle | Esc | Volver |
| Calendario | h/l, j/k | Día anterior/siguiente, semana anterior/siguiente |
| Calendario | n/p, t | Mes siguiente/anterior, hoy |
| Notificaciones | ! | Abrir bandeja |
| Notificaciones | m / M | Marcar leída / todas |
| Torneos | g / r / t | Generar fixture / resultado / tabla |
| Bienestar | a | Asistente de rutinas |

La ayuda muestra esta tabla filtrada por la pantalla actual y un enlace al tutorial.

### 9.4 Recorridos clave

#### A. Registrar un partido en menos de 30 segundos

~~~mermaid
sequenceDiagram
    participant U as Usuario
    participant A as App
    participant S as PartidoService
    participant C as ClimaService
    participant D as SQLite
    U->>A: n (desde cualquier pantalla)
    A->>A: abre wizard en paso 1
    U->>A: fecha, cancha, camiseta, botín
    A->>C: obtener clima (lat/lon + fecha)
    C-->>A: temperatura, viento, código WMO
    U->>A: goles, asistencias, rendimiento
    U->>A: Ctrl+S
    A->>S: guardar (validar + transacción)
    S->>D: insert partido + financiamiento
    S->>D: actualizar estadística de camiseta
    A-->>U: toast "Partido guardado" + Deshacer
    A->>A: evaluar logros y notificaciones
~~~

Claves: el wizard ya existe; hay que sumar la obtención de clima, el toast y la evaluación de
logros al guardar. Si el clima falla, el partido se guarda igual y el aviso es discreto.

#### B. Revisión semanal

1. g abre Hoy: racha, totales del mes, próximo partido, lesiones activas.
2. Enter en "Estadísticas" abre el hub; elegir Rendimiento → Tendencia.
3. La tecla ! abre notificaciones (récord cerca, logro desbloqueado, presupuesto).
4. x exporta la vista si se quiere compartir.

#### C. Planificar la semana

1. c abre Calendario; el día de hoy aparece seleccionado con sus eventos.
2. h/l/j/k recorren días; Enter en un día muestra partidos, finanzas y recordatorios.
3. n crea un recordatorio en ese día (o un partido si el contexto es deportivo).
4. Esc vuelve; el dashboard muestra el próximo recordatorio.

#### D. Cerrar una temporada

1. Ir a Temporadas → detalle de la temporada activa.
2. Pestaña Resumen: totales, campeón, goleador, lesiones.
3. Pestaña Fatiga: evolución y alertas.
4. Comparar con la temporada anterior.
5. Exportar PDF o XLSX (IMP-28) y generar el resumen compartible.

### 9.5 Bocetos de pantalla

**Dashboard "Hoy" (160×42)**

~~~text
┌ MiFutbolRS › Inicio ──────────────────────────────────────────────── BD: … ─┐
│                                                                            │
│  ┌ Próximo partido ──────────────┐  ┌ Racha ────────────────────────────┐  │
│  │ Sáb 14 · 21:00 · Cancha 5     │  │ 3 victorias · 12 goles en 5       │  │
│  │ vs Los Pibes · F7             │  │ ▁▂▃▅▆█  (sparkline)               │  │
│  └───────────────────────────────┘  └───────────────────────────────────┘  │
│  ┌ Este mes ─────────────────────┐  ┌ Alertas ──────────────────────────┐  │
│  │ 6 partidos · 9 goles · 4 asist│  │ ⚠ Lesión activa (rodilla)         │  │
│  │ Rendimiento 7,2 · 4V 1E 1D    │  │ ⚠ Presupuesto 92 %                │  │
│  └───────────────────────────────┘  └───────────────────────────────────┘  │
│  ┌ Acciones rápidas ────────────────────────────────────────────────────┐  │
│  │  [n] Nuevo partido   [c] Calendario   [t] Torneos   [Ctrl+K] Buscar  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│  ┌ Logro más cercano ───────────────────────────────────────────────────┐  │
│  │ Goleador Experto ████████░░ 18/25 goles                              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│ ↑↓: mover   Enter: abrir   Ctrl+K: paleta   ?: ayuda                       │
└────────────────────────────────────────────────────────────────────────────┘
~~~

**Paleta de comandos (Ctrl+K)**

~~~text
┌ Paleta ─────────────────────────────────────────────┐
│ > parti                                             │
│                                                     │
│  Acciones                                           │
│  ▸ Nuevo partido                            n       │
│  ▸ Ir a Partidos                            p       │
│  ▸ Buscar "partido"                         /       │
│                                                     │
│  Entidades                                          │
│  ▸ Partido #142 · vs Los Pibes · 14/09              │
│  ▸ Partido #138 · vs Deportivo · 07/09              │
└─────────────────────────────────────────────────────┘
~~~

**Listado con panel de vista previa** (patrón actual que se conserva)

~~~text
┌ Camisetas ──────────────────────────────┬ Ficha ──────────────────────────┐
│ > Azul 2024     14 PJ  8V 3E 3D  1,9 GP │ Azul 2024                        │
│   Roja 2023      6 PJ  2V 1E 3D  0,8 GP │ Marca · Modelo · Temporada       │
│   Blanca         2 PJ  1V 0E 1D  0,5 GP │ Costo · Estado · Imagen          │
└─────────────────────────────────────────┴──────────────────────────────────┘
~~~

### 9.6 Feedback y estados

- **Toasts**: éxito/error con duración 2,5 s y acción "Deshacer" cuando aplica.
- **Badges**: contador de notificaciones no leídas en el header; punto en el módulo con novedades.
- **Empty states**: cada listado explica qué falta y ofrece la acción ("Todavía no hay torneos.
  Creá el primero con n.").
- **Loading**: operaciones de red (clima, sync, update) muestran spinner y permiten cancelar;
  nunca bloquean el render.
- **Confirmaciones**: solo para borrados irreversibles, restauración y salida con cambios.
- **Undo**: tras crear/editar/borrar, toast con Ctrl+Z; el historial queda en Ajustes.

### 9.7 Responsive y accesibilidad

| Ancho | Layout |
|---:|---|
| < 60 | Una columna; tabs colapsan a texto; footer de 1 línea con teclas esenciales |
| 60–79 | Menú y detalle apilados; tarjetas en 1 columna |
| 80–119 | Sidebar colapsado a iconos o tabs; tarjetas en 2 columnas |
| 120–159 | Sidebar agrupado de 20 columnas; dashboard 2×3 |
| ≥ 160 | Sidebar + panel de vista previa + dashboard 3 columnas |

Accesibilidad: tamaño de texto (3 niveles) aplicado a listas y formularios, tema de alto
contraste, foco visible con borde de acento, navegación completa sin ratón, mensajes sin
depender solo del color (iconos + texto).

### 9.8 Qué se conserva del flujo actual

- Listado → detalle → formulario, con n/e/d/Enter.
- Búsqueda incremental con / y filtros con f/o.
- Footer contextual y pantalla de ayuda.
- Tabs y sidebar (con umbrales ajustados).
- Wizard de partido y transacción financiera automática.
- Visor de imágenes y animaciones sutiles.
- Ajustes con backup, integridad y exportación de la base.

El cambio es **aditivo**: nada de lo actual desaparece; se agregan superficies y se mudan tres
cosas que hoy están mal ubicadas (búsqueda, undo y notificaciones).

---

## 10. Roadmap por olas

### 10.1 Plan de olas

| Ola | Duración | Ítems | Hito de salida |
|---|---:|---|---|
| 0 | 2 semanas | IMP-01 a IMP-10 | La app abre en Hoy, se navega con paleta, hay logros/records/notificaciones/undo y se exporta cualquier listado |
| 1 | 3 semanas | IMP-11 a IMP-16 | Torneos y temporadas completos; estadísticas de canchas, comparador y meta-análisis; "Último" |
| 2 | 4 semanas | IMP-17 a IMP-24 | Bienestar, recordatorios/agenda, colecciones, tiendas, multimedia, amigos y progresión |
| 3 | 6 semanas | IMP-25 a IMP-32 | Entrenador IA, clima, sync, PDF/XLSX/ODS, multiusuario, i18n, auto-update y simulador |

Cada ola termina con: **cargo test**, **cargo clippy --all-targets -- -D warnings**,
**cargo fmt --check**, build release, capturas actualizadas y una entrada en docs/ACEPTACION.md.

### 10.2 Sprints sugeridos (2 semanas)

| Sprint | Entrega | Criterio de aceptación |
|---|---|---|
| S1 | IMP-01, IMP-02, IMP-03 | Dashboard, calendario y paleta funcionando con datos reales |
| S2 | IMP-04, IMP-05, IMP-06 | Logros, records y notificaciones integrados al guardar partido |
| S3 | IMP-07, IMP-08, IMP-09, IMP-10 | Undo, export filtrado, tutorial y atajos; cierre de Ola 0 |
| S4 | IMP-11 (parte 1) | CRUD de torneos + equipos + fixture |
| S5 | IMP-11 (parte 2) | Resultados, tabla, goleadores, dashboard y export |
| S6 | IMP-12, IMP-16 | Temporadas con resumen y comparación; "Último" |
| S7 | IMP-13, IMP-14, IMP-15 | Reportes de canchas, comparador y meta-análisis |
| S8 | IMP-17 (fase 1) | Hábitos, entrenamiento y salud con score/riesgo |
| S9 | IMP-17 (fase 2), IMP-18 | Nutrición/mental/asistente; recordatorios y agenda |
| S10 | IMP-19, IMP-20, IMP-21 | Colecciones, tiendas y multimedia |
| S11 | IMP-22, IMP-23, IMP-24 | Amigos, progresión y química |
| S12 | IMP-25 (parte 1) | Estado y consejos del entrenador IA |
| S13 | IMP-25 (parte 2), IMP-26 | Predicción/formación/alertas; clima |
| S14 | IMP-28 | PDF, XLSX y ODS |
| S15 | IMP-27, IMP-29, IMP-30, IMP-31, IMP-32 | Sync, multiusuario, i18n, update y simulador |

La numeración es orientativa; el orden dentro de cada ola se puede ajustar según prioridad del
usuario. Lo importante es no abrir la Ola 1 sin cerrar la 0: el hábito es lo que hace que las
demás features se usen.

### 10.3 Definición de terminado (por ítem)

- Migración + modelos + repo + servicio + pantalla + teclas.
- Pruebas unitarias del cálculo y de validación; al menos una de integración de flujo.
- Sin SQL en UI; sin unwrap/expect fuera de tests; errores mostrados en modal/toast.
- Documentación en docs/TUI.md (pantalla y teclas) y captura si aplica.
- cargo test, clippy y fmt limpios; sin regresión de rendimiento.
- Entrada en docs/ACEPTACION.md con evidencia.

### 10.4 Métricas de éxito

| Métrica | Objetivo |
|---|---|
| Tiempo para registrar un partido | < 30 s con datos precargados |
| Teclas hasta cualquier módulo | ≤ 2 (atajo o paleta) |
| Apertura del dashboard | < 50 ms con 10.000 partidos |
| Cobertura de pruebas | ≥ 180 pruebas al cerrar Ola 1 |
| Crash-free en uso normal | 100 % (sin panics por entrada) |
| Adopción de logros/records | Visibles en el dashboard y notificados |

---

## 11. Decisiones técnicas y dependencias

### 11.1 Esquema y migraciones

- Una migración por ola: 004 (Ola 0), 005 (Ola 1), 006 (Ola 2), 007 (Ola 3). Nunca editar una
  migración aplicada; los cambios van en la siguiente.
- Todas las tablas nuevas con FK, CHECK y created_at/updated_at cuando corresponda.
- Índices para las consultas nuevas: por fecha, por entidad y por estado.
- El backup previo a migrar ya está resuelto en Database::open; mantenerlo.

### 11.2 Convenciones de datos

| Tema | Convención |
|---|---|
| Dinero | INTEGER en centavos (i64); helper domain::money |
| Fechas | TEXT ISO 8601 (chrono); fechas solas YYYY-MM-DD; datetime YYYY-MM-DD HH:MM:SS |
| Booleanos | INTEGER 0/1 con CHECK |
| Enums | TEXT o INTEGER con CHECK; conversión en domain |
| Textos | NOT NULL DEFAULT ''; sin NULL para campos de formulario |
| Imágenes | Ruta relativa a la carpeta de imágenes; validar extensión |
| JSON | Solo para payloads externos (clima) o snapshots (undo); no para datos consultables |

### 11.3 Dependencias nuevas

| Necesidad | Crate recomendado | Alternativa | Impacto |
|---|---|---|---|
| HTTP (clima, sync, update) | ureq 3 con rustls | reqwest + tokio (descartado: async) | +~1 MB, sin runtime async |
| PDF | printpdf | genpdf (más opinado) | +~1–2 MB |
| XLSX | rust_xlsxwriter | zip + XML manual | +~1 MB |
| ODS | spreadsheet-ods o zip + XML | solo XLSX al inicio | +~0,5 MB |
| Hash de contraseña | argon2 | bcrypt | +~0,2 MB |
| Filtro difuso de paleta | fuzzy-matcher | implementación propia | +~0,1 MB |
| Serialización | serde/serde_json (ya está) | — | — |

Antes de sumar cada una: revisar licencia (MIT/Apache), MSRV ≤ 1.88, tamaño en release y
mantenimiento. cargo deny ya está configurado.

### 11.4 HTTP sin async

La app es síncrona. Para no congelar la UI:

- Operaciones puntuales (clima de un partido, check de update): llamada bloqueante con timeout
  de 5–15 s dentro de un modal de progreso; el usuario puede cancelar.
- Operaciones largas (sync, descarga de PDF): hilo con std::thread + canal mpsc; el bucle de la
  UI consulta el canal en cada tick y muestra progreso. Sin Tokio.
- Errores de red nunca bloquean el guardado local: se guarda y se avisa.

### 11.5 Texto y UTF-8

- Todo el código y los datos en UTF-8; unicode-width para anchos de celda.
- PDF: elegir fuente con soporte Latin-1/UTF-8; probar acentos y eñes; si el motor no cubre,
  incrustar una TTF (DejaVu) o generar HTML imprimible como fallback.
- Nombres largos: truncar con util::text::truncate_width (ya existe).

### 11.6 Rendimiento

- Presupuesto: render < 16 ms por frame; listados < 100 ms con 10.000 partidos; dashboard < 50 ms.
- Índices nuevos: partido(fecha_hora), lesion(estado, fecha), notificacion(leida), torneo(estado),
  recordatorio(fecha), logro_desbloqueado(fecha).
- Evitar N+1: el dashboard hace 6 consultas fijas, no una por tarjeta.
- Medir con Criterion los reportes nuevos (records, meta, torneos).

### 11.7 Pruebas

- Unitarias: fórmulas (IA, bienestar, fatiga, fixture, standings, comparador), validaciones,
  formateo de tiempo.
- Repos: base en memoria + fixtures SQL; verificar list/get/insert/update/delete y agregados.
- Integración: teclado de punta a punta (tests/ui_smoke.rs), incluyendo nuevos atajos y modales.
- Red: inyectar un cliente HTTP de prueba (trait ClimaClient) para no depender de internet.
- Migraciones: abrir una base con la versión anterior y migrar; verificar datos.

### 11.8 Logging y privacidad

- Mantener util/logging; no loguear tokens, contraseñas ni coordenadas exactas.
- Sync: token en tabla local; errores con código HTTP, no con el body completo.
- Clima: guardar el JSON solo si se necesita; si no, columnas derivadas.

### 11.9 Compatibilidad con la importación legacy

- El importador actual cubre 22 tablas; al agregar módulos, extenderlo solo si el usuario tiene
  datos legacy de ese módulo (torneos, temporadas, bienestar, amigos, tiendas, media,
  recordatorios en JSON).
- Los logros no se importan: se recalculan.
- Documentar en docs/MIGRACION_BD.md qué se importa y qué no.

### 11.10 Estructura de archivos después del crecimiento

~~~text
src/
  app.rs                 # App, Module, Screen, teclado global
  app_<modulo>.rs        # estado y acciones por módulo
  domain/
    models.rs            # entidades y DTO (crecerá; evaluar dividir por dominio)
    stats.rs             # cálculos puros
    ...
  db/
    <modulo>_repo.rs
  services/
    <modulo>_service.rs
  ui/
    screens/<modulo>.rs
    widgets/
  util/
    i18n.rs
    http.rs
~~~

Cuando models.rs supere ~3.000 líneas, dividir en domain/deporte.rs, domain/persona.rs y
domain/datos.rs sin cambiar los tipos públicos.

---

## 12. Riesgos y mitigaciones

| Riesgo | Impacto | Probabilidad | Mitigación |
|---|---|---|---|
| Scope creep (25 módulos) | Alto | Alta | Olas cerradas; no abrir la siguiente sin terminar la anterior |
| Deriva de esquema | Alto | Media | Migraciones numeradas, revisión, backups, tests de migración |
| UI congelada por red | Medio | Media | Timeouts, hilos + canal, guardar local primero |
| PDF con acentos rotos | Medio | Media | Probar temprano; fuente TTF embebida o HTML imprimible |
| Dependencia de PocketBase | Medio | Baja | Sync opcional; el núcleo funciona sin servidor |
| i18n incompleta | Bajo | Alta | Empezar por chrome; tests de claves; no traducir datos |
| Pérdida de datos por undo | Alto | Baja | Snapshots validados, confirmación, whitelist de tablas |
| Regresión de rendimiento | Medio | Media | Benchmarks por ola; índice y consultas revisadas |
| Solapamiento Carrera/Logros/Records | Bajo | Media | Definir alcance: carrera narrativa, logros gamificación, records datos |
| Sobrecarga visual (24 módulos) | Medio | Alta | Sidebar agrupado, paleta, dashboard, búsqueda |
| Deuda de pruebas | Medio | Media | DoD por ítem; no merge sin tests |
| Ratón/terminales dispares | Bajo | Media | Todo accesible por teclado; mouse como extra |
| Licencia GPL-3.0 del original | Alto | Media | Reimplementar ideas/fórmulas, no copiar código; MiFutbolR es MIT |
| Windows: rutas/UTF-8 | Medio | Media | Probar release en Windows; usar PathBuf |

---

## 13. Apéndices

### A. Mapa de tablas de MiFutbolC a MiFutbolR

| Tabla C | Propósito | En MiFutbolR |
|---|---|---|
| camiseta, cancha, botin, equipo, jugador, partido, lesion | Núcleo | ✅ existen |
| financiamiento, presupuesto_mensual | Finanzas | ✅ existen |
| settings, usuario, custom_menus, atajo_config | Sistema | ✅ settings/usuario/custom_menus; atajo_config nueva |
| carrera_identidad, carrera_partido_hito, carrera_resumen_narrativo | Carrera | ✅ existen |
| entrenamiento_plan, progresion_jugador | Progresión | ✅ existen (sin UI) |
| torneo, partido_torneo, equipo_historial | Torneos | ✅ existen; faltan equipo_torneo, torneo_fase y estadísticas |
| temporada | Temporadas | ✅ existe; faltan fases, resumen, fatiga, evolución y torneo_temporada |
| estadistica | Agregado por camiseta | ✅ existe |
| partido_meta, partido_tag | Favoritos/tags | ✅ existen |
| notificacion | Avisos | ❌ nueva (IMP-06) |
| logro_desbloqueado | Fecha de logro | ❌ nueva (IMP-04) |
| recordatorio | Recordatorios | ❌ nueva (IMP-18) |
| amigo | Ranking | ❌ nueva (IMP-22) |
| tienda | Tiendas | ❌ nueva (IMP-20) |
| media | Multimedia | ❌ nueva (IMP-21) |
| coleccion, inventario_item, coleccion_inventario | Colecciones | ❌ nuevas (IMP-19) |
| equipamiento_historial | Historial | ❌ nueva (IMP-19) |
| bienestar_* (14) | Bienestar | ❌ nuevas (IMP-17) |
| quimica_jugador_estadistica | Química | ❌ nueva (IMP-24) |
| ia_config, consejos_historial, perfil_usuario_ia | Entrenador IA | ❌ nuevas (IMP-25) |
| sync_config, sync_dirty, sync_conflicto | Sync | ❌ nuevas (IMP-27) |
| reporte_config, reporte_generado | Reportes | 🟡 opcional |
| comparacion_historial | Comparaciones | 🟡 opcional |
| backup_config | Auto-backup | 🟡 opcional (Ajustes) |

### B. Árbol de menús de MiFutbolC (resumen)

1. Calendario: mes, eventos del día, hoy.
2. Camisetas: crear, listar, modificar, eliminar, sortear, imagen, ver, ajustes de imagen,
   información, cargar información, reactivar, gráfico de uso.
3. Canchas: crear, listar, modificar, eliminar, imagen, ver, información, reactivar, exportar.
4. Botines: crear, listar, modificar, eliminar, sortear, imagen, ver, ajustes, información,
   reactivar, predeterminado.
5. Equipos: fijo, momentáneo (1 o 2), listar, modificar, eliminar, imagen, ver, reactivar.
6. Partidos: crear, listar, modificar, eliminar, simular, favoritos, tags, clima histórico,
   último (victoria/derrota/empate/gol/asistencia).
7. Lesiones: crear, listar, modificar, eliminar, estadísticas, diferencias, actualizar estados.
8. Estadísticas: partidos, goles, asistencias, rendimiento, canchas, reportes, filtros.
9. Logros: todos, completados, en progreso, no completados.
10. Financiamiento: ingreso, gasto, listar, modificar, eliminar, resumen, balance, exportar,
    presupuestos.
11. Torneos: crear, listar, modificar, eliminar, administrar.
12. Temporada: crear, listar, modificar, eliminar, administrar, comparar.
13. Análisis: básico, comparador, evolución temporal, química, entrenador IA.
14. Bienestar: planificación, mentalidad, entrenamiento, alimentación, asistente, mental, salud.
15. Carrera: identidad, perfil, hitos, timeline, Hall of Fame, resumen, retro, estadísticas,
    logros, progresión.
16. Recordatorios: listar, agregar, editar, eliminar, filtrar, exportar, importar, agenda,
    recurrentes.
17. Colecciones: inventario, sincronizar, colecciones, asociar, quitar, ver, filtrar, backup,
    historial.
18. Records & Rankings: 16 reportes.
19. Tiendas: crear, listar, modificar, eliminar.
20. Multimedia: crear, listar, filtrar, editar, eliminar.
21. Tutorial: iniciar, paso, reiniciar.
22. Resumen compartible: HTML, Markdown, estadísticas, mejor temporada.
23. Ranking amigos: agregar, listar, modificar, eliminar, ranking, comparar.
24. Ajustes: tema, idioma, accesibilidad, usuario, actual, reset, modo, exportar, importar, ODS,
    búsqueda, update, backup, integridad, undo, notificaciones.
25. Sync: configurar, login, sincronizar, subir todo, estado, conflictos, auto, logout.

### C. Glosario

- **Camiseta**: equipación con historial de uso; se puede sortear y desactivar.
- **Cancha**: instalación con servicios, precios y coordenadas para el clima.
- **Botín**: calzado; puede ser predeterminado o sorteado.
- **Torneo**: competencia con formato, equipos, fixture y tabla.
- **Temporada**: ciclo con fases que agrupa torneos.
- **Fatiga**: carga acumulada por partidos recientes (0–100).
- **Logro**: objetivo derivado de los datos (goles, partidos, rachas).
- **Récord**: mejor/peor valor histórico de una métrica.
- **Química**: rendimiento conjunto de dos jugadores.
- **Prep score**: puntaje de preparación física/mental (0–100).
- **Riesgo de lesión**: probabilidad estimada por cansancio, racha y lesiones activas.
- **Undo**: deshacer una escritura restaurando el snapshot anterior.
- **Sync**: sincronización bidireccional con PocketBase.
- **Modo retro**: "hoy en tu historia", recuerdos de partidos en la misma fecha.

### D. Cómo reproducir el análisis

~~~text
git clone --depth 1 https://github.com/thomashamer3/MiFutbolC
# conteo: 162 archivos .c/.h propios, ~89.977 líneas
# menús: buscar MenuItem items[] en cada .c
# esquema: abrir mifutbol.db y consultar sqlite_master
# lógica: leer entrenador_ia.c, logros.c, bienestar.c, torneo.c, temporada.c
# local: 75 archivos .rs, 22.640 líneas, 25 tablas, 157 pruebas
~~~

### E. Checklist del primer sprint

- [ ] Migración 004 con logro_desbloqueado, notificacion y settings.tutorial_completado.
- [ ] Módulo Inicio con InicioResumen y 6 consultas.
- [ ] Pantalla Calendario con grilla y navegación.
- [ ] Overlay de búsqueda global + paleta de comandos.
- [ ] Logros MVP (30–40) con barras y desbloqueo.
- [ ] Records MVP (8 reportes) con detalle.
- [ ] Notificaciones con badge y marcar leídas.
- [ ] Undo con snapshots y Ctrl+Z.
- [ ] Export filtrado CSV/JSON/HTML/MD.
- [ ] Tutorial de 4 pasos y ayuda actualizada.
- [ ] Pruebas: fórmulas, repos, teclado; capturas y docs.

### F. Correspondencia con los documentos existentes

- docs/PLAN_IMPLEMENTACION.md: este catálogo continúa el roadmap M0–M12 y lo extiende.
- docs/ACEPTACION.md: cada ola agrega su evidencia al final.
- docs/TUI.md: cada pantalla nueva documenta layout y teclas.
- docs/MIGRACION_BD.md: se amplía si se importan tablas nuevas.
- README.md: actualizar la lista de módulos y capturas al cerrar cada ola.

### G. Mapa de módulos de MiFutbolC

| Módulo | Archivos principales |
|---|---|
| Calendario | calendario.c/h |
| Camisetas | camiseta.c/h, export_camisetas*.c |
| Canchas | cancha.c/h, estadisticas_canchas.c/h |
| Botines | botin.c/h |
| Equipos | equipo.c/h (incluye jugadores) |
| Partidos | partido.c/h, filtros.c/h, busqueda.c/h |
| Lesiones | lesion.c/h, estadisticas_lesiones.c/h |
| Estadísticas | estadisticas*.c/h (generales, año, mes, meta, canchas, lesiones) |
| Logros | logros.c/h |
| Financiamiento | financiamiento.c/h |
| Torneos | torneo.c/h, posiciones.c/h |
| Temporada | temporada.c/h |
| Análisis | analisis.c/h, entrenador_ia.c/h |
| Bienestar | bienestar.c/h, progresion.c/h, reportes.c/h |
| Carrera | carrera.c/h |
| Recordatorios | recordatorios.c/h |
| Colecciones | colecciones.c/h, equipamiento_historial.c/h |
| Records & Rankings | records_rankings.c/h |
| Tiendas | tienda.c/h |
| Multimedia | media.c/h |
| Tutorial | tutorial.c/h |
| Resumen Compartible | resumen_compartible.c/h |
| Ranking Amigos | ranking_amigos.c/h |
| Ajustes | settings.c/h, lang.c/h, atajos.c/h, backup.c/h, db_integridad.c/h, undo.c/h |
| Sincronización | sync.c/h, sync_entidades.c/h, sync_http.c/h, sync_queue.c/h |

Se leyeron en profundidad los módulos de lógica y datos (partido, cancha, camiseta, equipo,
torneo, temporada, bienestar, carrera, logros, entrenador IA, analisis, records, resumen,
recordatorios, calendario, sync, openmeteo, backup, undo, settings, utils). El resto se relevó a
través de sus menús, cabeceras, consultas SQL y documentación.

---

**Fin del documento.** Si se implementa una sola cosa de todo esto, que sea el dashboard "Hoy"
con acciones rápidas: es lo que convierte una app de consulta en una app de uso diario.

