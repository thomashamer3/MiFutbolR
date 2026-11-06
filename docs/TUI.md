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
