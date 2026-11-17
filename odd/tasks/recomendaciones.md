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
- [ ] R2 Bandera undo_activo (migración 012 + import/restore). Ruta: inline.
- [ ] R3 PRAGMA optimize al cerrar. Ruta: inline.
- [ ] R4 Inicio perezoso. Ruta: inline.
- [ ] R6 CI deny + release + bench. Ruta: inline.
- [ ] R1 Importador de perfiles MiFutbolC. Ruta: delegated (writer trigger: servicio + usuario + CLI + portada + tests).
- [ ] R5a Clima Open-Meteo en segundo plano. Ruta: delegated.
- [ ] R5b Bienestar. Ruta: delegated.
- [ ] R5c Colecciones. Ruta: delegated.
- [ ] R5d Entrenador IA. Ruta: delegated.
- [ ] R5e Exportación PDF y ODS. Ruta: delegated.

## Criterios de aceptación
- Cada tarea con test propio; `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` OK.
- R2: inserción masiva con undo desactivado ≈ sin triggers (medido).

## Progreso / evidencia
(pendiente)
