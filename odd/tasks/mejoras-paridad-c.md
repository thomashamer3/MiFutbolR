# Mejoras tras el análisis: errores, flujo MiFutbolC, rendimiento, funciones y limpieza

## Objetivo
Implementar los 5 bloques propuestos en el análisis del 2026-11-23 ("Implementa Todo"), con
MiFutbolC (`D:\ANIME\Libros\Lenguaje C\Proyectos\MiFutbolC`) como referencia de flujo.

## Problema / por qué
El análisis encontró un bypass de contraseñas, un panic con terminales angostas, escrituras sin
transacción, búsquedas rotas con `%`/`_`, y un flujo de menú que se aleja del de C (opciones ocultas
tras teclas, Análisis ausente, modo Simple/Avanzado sin efecto, sin restaurar backup al inicio).

## Alcance autorizado
- B1 Errores 1–7 del análisis.
- B2 Flujo C: menú numerado completo (Calendario, Logros, Records, Análisis, Tutorial), números de
  dos dígitos que abren, modo Simple/Avanzado filtra, oferta de restaurar backup al arrancar,
  Esc/`q` coherentes, teclas globales que no se cuelen en módulos.
- B3 Rendimiento: búsqueda solo al cambiar el texto, `go_back` con `inicio_sucio`, filtros por
  `mes_anio`/rango, migración 015 con índices de FKs, caché de imágenes acotada.
- B4 Funciones de C: Análisis (básico, evolución mensual, química), reactivar camisetas,
  administrar backups (listar/restaurar/eliminar), VACUUM/ANALYZE/integridad, exportar todo a
  JSON y Markdown.
- B5 Limpieza: un helper de selección, un flujo de borrado común, textos de ayuda compartidos,
  quitar `Screen::Module`/`module_placeholder`.

## Fuera de alcance (no pedido en la propuesta)
Sincronización PocketBase, XLSX (ODS ya cubre hojas de cálculo), MiFutbolMobile, importar
JSON/TXT/CSV/HTML, migración chrono → jiff.

## Restricciones
- Sin dependencias nuevas.
- Tests y docs junto al comportamiento; commit Conventional por tarea.

## Configuración
- TDD: desactivado (fuente: feature anterior `recomendaciones.md`, sin configuración). Runner: `cargo test`.
- RDD: `gentle-ai` no disponible → disabled/unmanaged.
- Entrega: ask-on-risk; previsión > 400 líneas → rama única `feature/mejoras-paridad-c`, commit por
  bloque; división en PRs a decisión del usuario.
- Engram: espejo `odd/mejoras-paridad-c/tasks`.

## Tareas
- [x] B1 Errores. Ruta: delegated (writer trigger: ~8 archivos).
- [ ] B2 Flujo C. Ruta: delegated (writer trigger: app.rs, ui, ajustes, main).
- [ ] B3 Rendimiento. Ruta: delegated (writer trigger: repos + migración + app).
- [ ] B4 Funciones de C. Ruta: delegated (writer trigger: servicios + pantallas nuevas).
- [ ] B5 Limpieza. Ruta: delegated (writer trigger: app_*.rs).

## Criterios de aceptación
- Cada bloque: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --check` OK; tests nuevos para cada error y cada lógica nueva.

## Progreso / evidencia
- Línea base: 307 tests OK, clippy y fmt limpios (master `d589bde`).
  (Recuento real de `#[test]` en `d589bde`: 257; el 307 anterior no coincide con `cargo test`.)
- B1 (commit en `feature/mejoras-paridad-c`):
  - 1 Seguridad: `usuario_service::cambiar_password(conn, id, actual, nueva)` y
    `eliminar(conn, id, actual)` verifican la contraseña actual (acepta hash `fnv1$` de C);
    el formulario de cambio pide "Contraseña actual" si la cuenta tiene, y borrar abre la
    pantalla `Screen::UsuarioEliminar` tras confirmar (como `eliminar_mi_cuenta_local` de C).
  - 2 `ui::draw` muestra "Terminal demasiado pequeña (mín. 40x12)" bajo 40x12; modal y
    overlay de búsqueda acotados al área (sin `clamp` con máximo < mínimo).
  - 3 `like_escape` (escapa `\`, `%`, `_`) + `ESCAPE '\'` en los 27 `LIKE` de `src/db`
    (búsqueda global y filtros de listados).
  - 4 `torneo_repo::reemplazar_fixture`: borrado + inserción en la misma transacción.
  - 5 Transacciones en `undo_service::deshacer`, cada migración + `user_version`, y
    `temporadas_service::crear`. Migraciones revisadas: los `BEGIN` son cuerpos de triggers.
  - 6 Evento de recordatorio en Inicio abre `Module::Recordatorios`.
  - 7 `calendario_repo::fecha_de` usa `texto.get(..10)`; el resto de slices en código no
    test opera sobre ASCII verificado (`validar_mes`) o son tests.
  - Tests nuevos (6): contraseña actual (servicio + UI), terminal 30x10 con modal, LIKE
    literal (búsqueda y filtro), fixture revertido, undo fallido sin `suspendido=1`,
    `fecha_de` multibyte. Total 263 OK; `cargo clippy --all-targets -D warnings` y
    `cargo fmt --check` limpios.
