# Recomendaciones de rendimiento, paridad con MiFutbolC y flujo de trabajo

## Objetivo
Implementar las 6 recomendaciones dadas tras el login multiusuario, usando como referencia
MiFutbolC local (`D:\ANIME\Libros\Lenguaje C\Proyectos\MiFutbolC`).

## Alcance autorizado
1. Importar perfiles de MiFutbolC (`users.db` + `mifutbol_<usuario>.db`) al registro `cuentas.db`
   con DB propia por cuenta, conservando contraseñas (hash FNV-1a 2 rondas de C, migrado a SHA-256
   en el primer login correcto).
2. Bandera `undo_activo` para que los triggers de deshacer no corran en importaciones/restauraciones.
3. `PRAGMA optimize` al cerrar cada conexión.
4. Resumen de Inicio recalculado solo al volver al menú (marcado sucio al guardar).
5. Paridad con MiFutbolC: clima Open-Meteo en segundo plano, módulos Bienestar, Colecciones,
   Entrenador IA, exportación PDF y ODS. Se porta el núcleo funcional de cada módulo (la versión C
   suma ~14k líneas); lo no portado queda listado aquí.
6. CI: `cargo deny check`, workflow de release por tag con instalador Inno Setup, benchmarks.

## Restricciones
- Nuevas dependencias solo donde no hay alternativa razonable (HTTP para Open-Meteo).
- Importaciones nunca modifican la BD de origen de MiFutbolC (solo lectura).
- Tests y docs junto al comportamiento.

## Configuración
- TDD: desactivado (sin configuración). Runner: `cargo test`.
- RDD: no disponible (gentle-ai ausente) → disabled/unmanaged.
- Engram: herramientas `mem_*` no disponibles → espejo `odd/recomendaciones/tasks` PENDIENTE.
- Entrega: ask-on-risk; previsión > 400 líneas (R5) → rama única `feature/recomendaciones`,
  commits por tarea; la división en PRs la decide el usuario.

## Tareas
- [x] R2 Suspender deshacer en la importación (la bandera `undo_control.suspendido` ya existía). Ruta: inline.
- [x] R3 PRAGMA optimize al cerrar. Ruta: inline.
- [x] R4 Inicio perezoso. Ruta: inline.
- [x] R6 CI deny + release + bench. Ruta: inline.
- [x] R1 Importador de perfiles MiFutbolC. Ruta: delegated (writer trigger: servicio + usuario + CLI + portada + tests).
- [x] R5a Clima Open-Meteo en segundo plano. Ruta: delegated.
- [x] R5b Bienestar. Ruta: delegated.
- [x] R5c Colecciones. Ruta: delegated.
- [x] R5d Entrenador IA. Ruta: delegated.
- [x] R5e Exportación PDF y ODS. Ruta: delegated.
- [x] R5f Paridad completa (reabierto: R5 quedó parcial). Bienestar restante (objetivos, planes y
  ejercicios, nutrición diaria, controles médicos, archivos de estudios, imágenes de menú),
  exportación de Bienestar/Colecciones/IA (como `export_bienestar.c`) en ODS y PDF, alerta del
  Entrenador IA antes de cargar un partido, deshacer para las tablas nuevas, importación de las
  tablas nuevas. Ruta: delegated (writer trigger: migración + servicios + UI + tests).

## Criterios de aceptación
- Cada tarea con test propio; `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` OK.
- R2: inserción masiva con undo desactivado ≈ sin triggers (medido).

## Progreso / evidencia
- R2 (`54eec1c`): la bandera ya existía (`undo_control.suspendido`) pero la importación no la usaba;
  ahora se activa dentro de la transacción. Medido antes: 10k inserts 1,20 s con triggers vs 0,24 s
  sin ellos. Test verificado en rojo sin el cambio (9 filas de historial) y verde con él.
- R3 (`54eec1c`): `impl Drop for Database` → `PRAGMA optimize`. Trivial, sin test.
- R4 (`54eec1c`): bandera `inicio_sucio`; resumen (33 ms con 20k partidos) se recalcula tras la tecla
  solo con el menú visible. Test `home_summary_refreshes_only_when_menu_is_visible`.
- 213 tests OK, clippy limpio tras R2–R4.
- R6 (`c290d95`): job cargo-deny (local: advisories/bans/licenses/sources ok tras ignorar
  RUSTSEC-2026-0543 chrono sin mantenimiento — migración a jiff/time pendiente), release.yml por tag
  (instalador probado localmente: `ISCC /DMyAppVersion=0.0.1` → MiFutbolR-Setup-0.0.1.exe, 5,4 MB;
  la versión debe ser numérica), bench.yml base vs PR. actionlint NO ejecutado (descarga remota
  denegada por permisos) → validación de workflows pendiente en el primer push.
- `.claude/` agregado a .gitignore (`fba1b40`).
- R1 (`8f3f621`, delegated): cuentas desde users.db, hash FNV-1a de C verificado contra el `utils.c`
  original compilado con gcc y migrado a SHA-256 en el primer login; origen adjunto en solo lectura.
  `e0e9457`: también importa las tablas de Bienestar/Colecciones/IA. Prueba con COPIA de un perfil
  real ("Hamer") a destino temporal: 166 filas, conteos idénticos al origen, origen sin cambios
  (SHA-1); la copia se borró después.
- R5b–R5d (`0bcb00c` + merge `4e5bdf2`, delegated en worktree): migración 012, módulos Bienestar
  (5 pestañas + asistente con las fórmulas de C), Colecciones (inventario, colecciones, sync de
  camisetas), Entrenador IA (reglas R1–R5, predicción, formación, descanso, puntos débiles).
  Corregidos bugs de C: `partido.fecha` inexistente → `date(fecha_hora)`; derrota = resultado 3.
  No portado: objetivos/planes, ejercicios, archivos y fotos, nutrición diaria, alerta
  interactiva antes del partido, deshacer y exportación de estas tablas.
- R5a (`3786010`, delegated): migración 013, cola con 8 intentos, hilo + mpsc, ureq 3.5.1 (rustls,
  MIT/Apache-2.0; deny permite CDLA-Permissive-2.0 de webpki-roots). Verificado con respuesta
  REAL de Open-Meteo (Buenos Aires 2025-06-01: 9,7 °C, WMO 0).
- R5e (`3786010` + fix `5f89779`): ODS y PDF propios. La validación independiente encontró un bug
  del ZIP (encabezado central de 48 bytes en vez de 46): Python zipfile lo rechazaba. Corregido
  con un test que recorre todas las entradas (rojo con el bug). Ahora zipfile: CRC OK, mimetype
  STORED primero, XML bien formado; poppler (pdftotext UTF-8): acentos, ñ, ¿ ¡ correctos.
  No probado con LibreOffice (no instalado).
- Integración: 248 tests OK, clippy `-D warnings` y fmt limpios, cargo deny ok. Binario release
  en consola real: 0 ms de CPU en 8 s de reposo, proceso vivo, 4 hilos.

## Pendiente (fuera de alcance)
- Migrar chrono → jiff/time (RUSTSEC-2026-0543 ignorado con motivo).
- actionlint no ejecutado (descarga denegada); sustituido por la validación offline descrita arriba.
- Pantallas de análisis de Bienestar de C (entrenamiento vs rendimiento, estadísticas de alimentación),
  abrir documentos con la app del sistema y optimizar imágenes al copiarlas.
- El importador no copia los archivos de `MiFutbolC\images` (en el perfil real esa carpeta está vacía).
- R5f (`09bb2d6`, delegated): migración 014 con las 8 tablas restantes de Bienestar (mismas columnas
  que db.c, + ON DELETE CASCADE en hijas), 8 pestañas en el CRUD existente, adjuntos copiados a
  Imagenes/ como C, nutrición del asistente con la fórmula de C; hojas ODS y secciones PDF de
  Bienestar/Colecciones/IA (export_bienestar.c / export_pdf.c; bug `partido.fecha` corregido);
  alerta del Entrenador IA al abrir el asistente de partido (umbrales por nivel, 24 h, crítico);
  deshacer para 16 tablas nuevas (excluidas ia_config, cola de clima y las 2 sin `id`); importación
  de las 8 tablas. Verificado por mí: 257 tests OK, fmt, clippy `-D warnings`, cargo deny ok.
- Workflows (sustituto de actionlint): parseo YAML con serde_yaml offline + chequeo de estructura
  (on, jobs, runs-on, needs existentes, uses XOR run, claves conocidas; verificado que detecta 4
  errores en un workflow roto); 15 scripts `run` bash con `bash -n` y 1 PowerShell con el parser
  de PowerShell: 0 errores.

