# Mejoras de TUI (ratatui)

Este documento resume las mejoras aplicadas siguiendo la documentación de
[widgets de ratatui](https://ratatui.rs/concepts/widgets/) y la
[API de ratatui 0.30](https://docs.rs/ratatui/latest/ratatui/).

## Estructura de pantalla

- **Header de 2 líneas**: marca `MiFutbolR` como chip, breadcrumb con el
  título de la pantalla, ruta de la base y chips de tema/modo. En pantallas
  angostas el título se trunca con ancho Unicode.
- **Regla horizontal** que separa el header del cuerpo.
- **Footer contextual**: chip de estado (o buffer de búsqueda en modo `/`) y
  pistas de teclado truncadas al ancho disponible.
- **Modales** centrados con tamaño dinámico según el contenido, icono de tipo
  (`?`, `!`, `i`), `Clear` detrás y pie de aceptar/cancelar.

## Widgets usados

| Widget | Dónde | Detalle |
|--------|-------|---------|
| `Table` | Listados | `column_spacing(1)`, `highlight_spacing(Always)`, símbolo `▶` y filas inactivas atenuadas. El estado `TableState` mantiene la selección visible al desplazarse. |
| `Table` + panel | Camisetas, Canchas, Botines, Equipos, Partidos, Lesiones, Transacciones | A partir de 110 columnas se muestra una **vista previa lateral** con los datos de la fila seleccionada. |
| `Tabs` | Wizard de Partidos | Pasos 1–4 con pestaña activa resaltada; el título conserva “Paso X/4”. |
| `Gauge` | Financiamiento y Estadísticas | Consumo del presupuesto y porcentaje de victorias. |
| `BarChart` | Financiamiento (gastos por categoría), Estadísticas (rendimiento por clima, goles por franja, partidos por cancha). |
| `Sparkline` | Estadísticas · Rendimiento | Tendencia de los últimos rendimientos. |
| `Scrollbar` | Formularios largos, resumen, timeline y estadísticas | Aparece solo si el contenido excede el alto visible. |
| `List` | Menú principal, carreras, estadísticas | Selección con `▶` y descripción por ítem. |

## Formularios

- Icono por tipo de campo (`Aa`, `¶`, `#`, `±`, `$`, `D`, `H`, `DT`, `☑`,
  `▾`, `☰`) y marca `*` para obligatorios.
- Campo enfocado resaltado; en listas desplegables se indican `◂ ▸`.
- Contador `usados/máximo` en campos de texto.
- Pie “Campo X de Y” con ayuda contextual o el error de validación.
- Desplazamiento automático al campo enfocado y barra lateral.

## Responsive

- Menú principal: tarjeta de vista previa al lado de la lista con ≥100 columnas;
  apilado en pantallas angostas.
- Paneles laterales de listados con ≥110 columnas.
- Los gráficos de Estadísticas solo se dibujan si hay alto suficiente
  (≥26 filas) para no sacrificar el texto.
- Todos los tests de UI renderizan también en 40×15 y 100×30.

## Accesibilidad

- Ningún estado depende solo del color: se usan etiquetas (`Activa`/`INACTIVA`),
  símbolos (`★`, `▶`) y texto explícito.
- El tema se puede cambiar en Ajustes, con una **muestra de colores** en vivo.

## Referencias

- <https://ratatui.rs/concepts/widgets/>
- <https://docs.rs/ratatui/latest/ratatui/>
- <https://ratatui.rs/examples/widgets/gauge/>
- <https://ratatui.rs/examples/widgets/barchart/>
- <https://ratatui.rs/examples/widgets/scrollbar/>

## Rediseño visual (ratatui-image / awesome-ratatui / tachyonfx)

- **Paleta RGB**: los 8 temas usan colores curados (fondo, texto, acento,
  éxito, aviso, error y selección) en lugar de colores ANSI básicos.
- **Paneles**: `Palette::panel` devuelve un `Block` con borde redondeado,
  padding horizontal y título; se usa en menú, formularios y widgets.
- **Barra de módulos**: `Tabs` con etiquetas cortas debajo del header indica el
  módulo activo y los demás; se oculta en el menú y en terminales <70 columnas.
- **Sidebar**: en módulos con ≥140 columnas se reemplazan los tabs por una
  columna fija de 18 con los 11 módulos y el activo resaltado. Es **clicable**:
  las zonas se guardan en `App::sidebar_hits` y el clic cambia de módulo.
- **Formularios**: borde redondeado en color de acento para indicar el panel
  activo.
- **Clic en filas**: Camisetas, Canchas, Botines, Equipos, Lesiones,
  Partidos y Financiamiento seleccionan la fila clickeada.
- **Miniaturas en listados**: las vistas previas de Camisetas, Canchas y
  Equipos muestran la imagen de la fila seleccionada.
- **Tablas expresivas**: estados y resultados usan color + texto
  (Victoria/Empate/Derrota, Activa/INACTIVA, estados de lesión).
- **Tipografía**: `tui-big-text` dibuja “MiFutbolRS” con píxeles de medio
  bloque en el menú principal (pantallas ≥110 columnas; si no, texto normal).
- **Animaciones**: `tachyonfx` aplica `fade_from` de 210 ms al cambiar de
  pantalla, 130 ms al abrir un modal y `fade_from_fg` de 150 ms sobre la fila
  seleccionada al moverse (`Effect::with_area`). El event loop baja el timeout
  a 16 ms mientras hay efectos activos y se desactivan en capturas/tests.
- **Logo**: `include_bytes!` del PNG de 256 px, renderizado en el panel de
  vista previa del menú; no requiere archivos externos.
- **Ratón**: la rueda se traduce a `Up`/`Down`; el clic izquierdo abre módulos
  desde el menú, cambia de módulo desde el sidebar y selecciona filas en
  Camisetas/Partidos (zonas guardadas en `menu_hits`/`table_hits`).
- **Imágenes**: `ratatui-image` con `Picker::halfblocks` y caché por tamaño;
  las fichas de camisetas, canchas y equipos muestran una miniatura de 28×10
  celdas; `v` abre un visor grande centrado (`Screen::ImagenViewer`). Funciona
  también en terminales sin gráficos.
- **Inspiración**: catálogo de `awesome-ratatui` para `tui-big-text`,
  `ratatui-image` y `tachyonfx`.


## Ola 0 — panel, calendario, logros, récords, avisos y undo

| Pantalla | Teclas |
|---|---|
| Menú / Hoy | `c` calendario · `Ctrl+F` búsqueda global · `Ctrl+K` paleta · `!` avisos · `l` logros · `R` récords · `t` tutorial |
| Calendario | `h/l` día · `j/k` semana · `n/p` mes · `t` hoy · `Enter` abrir evento · `Esc` volver |
| Logros | `h/l` pestaña · `j/k` mover · `r` refrescar |
| Récords | `j/k` mover · `Enter` abrir partido · `r` recalcular |
| Avisos | `j/k` mover · `m` leída · `M` todas · `Enter` abrir |
| Listados | `x` exporta CSV/JSON/HTML/Markdown a `Exportaciones/` |
| Global | `Ctrl+Z` deshacer la última escritura (con confirmación) |
| Últimos | `U` abre · `j/k` mueve · `Enter` abre el partido · `r` recalcula |
| Comparador | `C` abre · `1/2` camisetas/canchas · `Tab` A/B · `j/k` elige · `Enter` compara |
| Temporadas | `E` abre · `n/e/d` alta/edición/borrado · `Enter` detalle · `c` compara con la anterior |
| Tiendas | `D` abre · `n/e/d` alta/edición/borrado · `r` refresca |
| Amigos | `A` abre ranking · `n/e/d` alta/edición/borrado · `r` refresca |
| Multimedia | `M` abre · `f` filtra por tipo · `n/e/d` alta/edición/borrado |
| Recordatorios | `Y` abre · `n/e/d` alta/edición/borrado · `c` completar · `a` agenda |
| Progresión | `P` abre · `n/e/d` mediciones · `p` planes de entrenamiento |
| Meta-análisis | Estadísticas → Meta · `↑↓`/`PgUp/PgDn` para recorrer |

El panel "Hoy" vive en el menú principal y muestra próximo partido, racha, totales del mes y
alertas. Se refresca al volver con `Esc`/`g` y después de cada guardado.


## Ola 1 — torneos

| Pantalla | Teclas |
|---|---|
| Torneos | `T` abre · `n` nuevo · `Enter` detalle · `e` editar · `d` borrar · `x` exportar |
| Detalle | `g` genera fixture · `r` carga resultado · `a` asocia equipos activos · `j/k` elige partido · `Esc` vuelve |
| Resultado | `Tab` cambia de campo · `Ctrl+S` guarda · `Esc` cancela |

Al crear un torneo se asocian todos los equipos activos. El fixture usa el método del círculo
(simple o ida y vuelta) y la tabla suma 3/1/0 puntos con diferencia de gol como desempate.
