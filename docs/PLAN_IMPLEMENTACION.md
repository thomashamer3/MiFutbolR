# Plan de implementación — MiFutbolR

> Estado final: M0–M12 completados. Ver `docs/ACEPTACION.md` para la evidencia.

Seguimiento del roadmap de `skills/ESPECIFICACION_PROYECTO_RUST_TUI.md` §23 y de la
checklist de `skills/Performance.md` §8.

## Fases

| Fase | Entregable | Estado |
|------|------------|--------|
| M0 | Scaffold, Cargo.toml (perfiles), logging, rutas | ✅ |
| M1 | Esquema, migraciones, seeds, `Database`, tests | ✅ |
| M2 | Shell TUI: layout, menú, navegación, temas, status bar | ✅ |
| M3 | Camisetas + Canchas + Botines (CRUD + listados) | ✅ Camisetas, Canchas y Botines con CRUD, búsqueda, filtros, sorteo y predeterminado |
| M4 | Equipos + jugadores | ✅ Equipos con plantel, dorsales únicos, capitán único y borrado con guardas |
| M5 | Partidos (wizard + listado + filtros + transacción) | 🔄 CRUD + listado paginado + filtros + favoritos + transacción automática ✅; wizard por pasos y submenú «Último» ⏳ |
| M6 | Lesiones | ✅ CRUD, filtros, búsqueda, estadísticas (total/activas/promedio entre lesiones) y advertencia de fecha futura |
| M7 | Estadísticas (hub + métricas + gráficos) | ✅ Hub con 6 categorías, generales/récords, series mensuales y anuales, 20 métricas de rendimiento, canchas y goles/asistencias; sparkline de tendencia |
| M8 | Financiamiento + presupuestos | ✅ Transacciones con filtros y resumen, totales por categoría, presupuesto mensual con alertas (límite y total) |
| M9 | Carrera (identidad, hitos, timeline, hall of fame) | ✅ Identidad, hitos, resumen con badges, timeline, Hall of Fame y logros; perfil dinámico y modo retro implementados en el servicio (pantallas en M12) |
| M10 | Resumen Compartible (HTML/MD + preview) | ✅ Datos, plantillas Markdown/HTML/texto, vista previa y exportación a Exportaciones/ |
| M11 | Ajustes (temas, modo, backup, integridad) | ✅ Tema/idioma/modo/tamaño/visor/botín, backup, exportación de BD, integridad y restauración en vivo |
| M12 | Pulido, accesibilidad, importación legacy, documentación | ✅ Wizard por pasos, importación legacy, pantallas de perfil/retro, CI multiplataforma, benchmarks y build release verificado |

## Checklist de Performance.md

### Arquitectura

- [x] Separar TUI, servicios y repositorios (estructura creada)
- [x] Centralizar la conexión SQLite (`Database`)
- [x] Implementar migraciones SQL (`migrations/`, `user_version`)
- [x] Separar modelos de entidades y vistas (modelos de dominio; DTOs por módulo)

### Rendimiento

- [x] Configurar perfil release (`opt-level=3`, `lto="thin"`, `codegen-units=1`, `strip`)
- [x] PRAGMAs recomendados (WAL, foreign_keys, synchronous=NORMAL, temp_store=MEMORY)
- [ ] Evitar clones y asignaciones innecesarias (revisión continua por módulo)
- [x] Usar consultas preparadas (repos con `prepare`/`query_map`)
- [x] Consultas preparadas reutilizadas en bucles (repasar por módulo)
- [x] Transacciones en operaciones multi-tabla (cancha + tipos/superficies, partido + financiamiento)
- [ ] Agrupar escrituras con transacciones (partidos, equipos, importación)
- [ ] Crear índices según las consultas reales (`002_indexes.sql` ya cubre las previstas)
- [ ] Evitar consultas SQL durante el renderizado (regla de la capa `ui`)

### Calidad

- [x] Pruebas unitarias (dinero, fechas, enums, validaciones, stats)
- [x] Pruebas de repositorios/migraciones (en memoria y archivo)
- [x] Clippy `-D warnings` y `cargo fmt` limpios
- [x] Revisar dependencias (`cargo tree`: sin Tokio/Actix/Axum ni ORM)
- [x] Benchmarks (Criterion) de listado, búsqueda, conteo y estadísticas (`benches/queries.rs`)
- [ ] `cargo audit` (requiere instalación aparte; queda documentado)

### Multiplataforma

- [x] Directorios de datos multiplataforma (`directories::ProjectDirs`)
- [x] CI multiplataforma en GitHub Actions (ubuntu/windows/macos): fmt, clippy, test y release

## Decisiones tomadas

- `edition = "2024"`, `rusqlite` con `bundled` sin feature `chrono` (se guardan
  fechas ISO como texto y se parsean en `domain::dates`).
- Tema inicial `Light` por el seed de la especificación (`theme = 0`).
- `Theme` es el enum legacy de `settings`; la paleta visual se llama `Palette`
  en `ui::theme` para evitar colisión de nombres.
- La capa `services` se completa por módulo; la UI solo la usará a través de `App`.

## Benchmarks (10 000 partidos)

Medidos con `cargo bench --bench queries` (Criterion, release):

| Consulta | Tiempo |
|----------|--------|
| Listado paginado (20 filas) | ~215 µs |
| Búsqueda por texto | ~640 µs |
| Conteo del filtro | ~0,99 ms |
| Estadísticas completas | ~71 ms |

Todo dentro del criterio de respuesta < 100 ms para la UI (§22.4), incluida la
carga completa de estadísticas.
