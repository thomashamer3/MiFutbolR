# Implementación de las mejoras de ANALISIS_MIFUTBOLC.md

> Seguimiento del objetivo **"Implementa todas las Mejoras Propuestas"**.
> Cada ítem se marca cuando tiene migración, dominio, repositorio, servicio,
> pantalla, teclas, pruebas y documentación.

## Estado general

| Ola | Ítems | Hechos | En curso | Pendientes |
|---|---:|---:|---:|---:|
| 0 | 10 | 10 | 0 | 0 |
| 1 | 6 | 5 | 1 | 0 |
| 2 | 8 | 5 | 0 | 3 |
| 3 | 8 | 0 | 0 | 8 |
| **Total** | **32** | **20** | **1** | **11** |

## Detalle por ítem

| ID | Mejora | Estado | Notas |
|---|---|---|---|
| IMP-01 | Dashboard "Hoy" | ✅ | Integrado en el menú principal (tarjetas Próximo/Racha/Mes/Alertas) y refrescado al volver. |
| IMP-02 | Calendario mensual | ✅ | `Screen::Calendario`; grilla 7×6, eventos de partidos y finanzas, panel del día, navegación h/l/j/k/n/p/t. |
| IMP-03 | Búsqueda global + paleta | ✅ | Overlays `Ctrl+F` y `Ctrl+K`; búsqueda en 7 entidades; paleta ejecuta acciones. |
| IMP-04 | Logros | ✅ | 65 logros en 5 niveles, 4 pestañas, desbloqueo persistido y notificación. |
| IMP-05 | Records & Rankings | ✅ | 16 reportes, pantalla con detalle y Enter al partido. |
| IMP-06 | Notificaciones | ✅ | Generación de lesiones, partido próximo, presupuesto, racha y logros; bandeja y badge. |
| IMP-07 | Undo | ✅ | Triggers SQLite en 8 tablas, Ctrl+Z con confirmación, historial de 50. |
| IMP-08 | Exportación filtrada | ✅ | Tecla `x` en 7 listados; genera CSV, JSON, HTML y Markdown en Exportaciones. |
| IMP-09 | Tutorial | ✅ | 4 pasos, banner en el menú, persiste `settings.tutorial_completado`. |
| IMP-10 | Atajos globales y ayuda | ✅ | g/c/!/l/t/x/R, Ctrl+K/F, Ctrl+Z y ayuda actualizada. |
| IMP-11 | Torneos | 🟡 | MVP: CRUD, equipos automáticos, round robin simple/doble, resultados, tabla y export. Faltan copa/grupos/eliminación. |
| IMP-12 | Temporadas | ✅ | CRUD, fases por defecto, torneos asociados, resumen, fatiga y comparación con la anterior. |
| IMP-13 | Stats de canchas | ✅ | 10 reportes: resumen, más/menos usadas, rendimiento, goles/asist., tipo, superficie, techada, localidad y franja. |
| IMP-14 | Comparador A/B | ✅ | Camisetas y canchas, 9 métricas lado a lado con ganador por métrica. |
| IMP-15 | Meta-análisis | ✅ | Consistencia (desvío/CV), outliers, contexto, impacto cansancio/ánimo, eficiencia y tendencia. |
| IMP-16 | Submenú "Último" | ✅ | 5 tarjetas (victoria/derrota/empate/gol/asistencia) con tiempo transcurrido y Enter al partido. |
| IMP-17 | Bienestar | ⏳ | Ola 2. |
| IMP-18 | Recordatorios | ✅ | CRUD, recurrencia diaria/semanal/mensual, agenda fusionada y calendario. |
| IMP-19 | Colecciones | ⏳ | Ola 2. |
| IMP-20 | Tiendas | ✅ | CRUD, contacto, productos, envío, favoritos y export. |
| IMP-21 | Multimedia | ✅ | CRUD de video/foto/artículo/audio, filtro por tipo y partido asociado. |
| IMP-22 | Ranking amigos | ✅ | CRUD de amigos y ranking con el usuario local incluido. |
| IMP-23 | Progresión | ✅ | Planes de entrenamiento y mediciones semanales de 5 habilidades. |
| IMP-24 | Química | ⏳ | Ola 2. |
| IMP-25 | Entrenador IA | ⏳ | Ola 3. |
| IMP-26 | Clima Open-Meteo | ⏳ | Ola 3. |
| IMP-27 | Sync PocketBase | ⏳ | Ola 3. |
| IMP-28 | PDF/XLSX/ODS | ⏳ | Ola 3. |
| IMP-29 | Multiperfil | ⏳ | Ola 3. |
| IMP-30 | i18n y accesibilidad | ⏳ | Ola 3. |
| IMP-31 | Auto-actualización | ⏳ | Ola 3. |
| IMP-32 | Simulador | ⏳ | Ola 3. |

## Ola 1 — torneos

- **IMP-11**: migración `006_ola1.sql` (`equipo_torneo`), dominio/repo/servicio de torneos,
  fixture por método del círculo, tabla 3/1/0, pantalla de listado/detalle y formularios de alta
  y resultado. Acceso con `T` o desde el menú.

## Cierre de la Ola 0

- **IMP-05**: `records_repo` + `records_service` + `Screen::Records` (16 reportes).
- **IMP-07**: migración `005_undo.sql` con triggers para camiseta, cancha, botín, equipo,
  jugador, partido, lesión y financiamiento; `undo_service` aplica la operación inversa con los
  triggers suspendidos; `Ctrl+Z` pide confirmación y refresca la pantalla.
- **IMP-10**: se suman `R` (récords), `L` (logros) y `Ctrl+Z` a los atajos y a la ayuda.

## Qué se agregó en esta ronda

- Migración `004_ola0.sql`: `logro_desbloqueado`, `notificacion` y `settings.tutorial_completado`.
- Dominio: `inicio`, `calendario`, `logros`, `busqueda`, `notificacion`.
- Repositorios: `inicio_repo`, `calendario_repo`, `logros_repo`, `busqueda_repo`, `notificacion_repo`.
- Servicios: `inicio_service`, `logros_service`, `busqueda_service`, `notificacion_service`, `export_service`.
- App: `app_inicio.rs` (panel, calendario, logros, avisos, overlays, tutorial, exportación).
- UI: `calendario`, `logros`, `notificaciones`, `busqueda`, `tutorial`; tarjetas "Hoy" en el menú; badge de avisos.
- Pruebas: **198** (123 unitarias + 75 de integración).
- `cargo clippy --all-targets -- -D warnings` y `cargo fmt --check` limpios.

## Decisiones y desvíos

- El dashboard vive en el menú principal en lugar de una pantalla `Inicio` separada. Evita romper los atajos 1-9 existentes y deja el contexto y los módulos en una sola vista.
- `/` sigue siendo la búsqueda **del listado**; la búsqueda global es `Ctrl+F`. Así no se pierde el filtro incremental ya probado.
- `x` exporta **los cuatro formatos** de una vez (CSV, JSON, HTML, MD) en lugar de abrir un selector; menos fricción y cubre IMP-08.
- Las notificaciones se generan al abrir la bandeja y al guardar un partido; se deduplican por tipo/entidad/día.

## Próximos pasos

1. Ola 2: colecciones, química y bienestar.
2. Completar torneos con copa/grupos/eliminación.
3. Actualizar `docs/TUI.md` y `docs/ACEPTACION.md` al cerrar cada ola.
