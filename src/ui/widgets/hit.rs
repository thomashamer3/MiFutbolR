//! Cálculo de zonas clicables para las filas de una tabla.

use ratatui::layout::Rect;

/// Devuelve un rectángulo por fila visible (borde + header consumen 3 filas).
#[must_use]
pub fn table_hits(area: Rect, offset: usize, total: usize) -> Vec<(Rect, usize)> {
    let visibles = area.height.saturating_sub(3) as usize;
    (0..visibles)
        .filter_map(|fila| {
            let indice = offset + fila;
            (indice < total).then_some((
                Rect {
                    x: area.x + 1,
                    y: area.y + 2 + fila as u16,
                    width: area.width.saturating_sub(2),
                    height: 1,
                },
                indice,
            ))
        })
        .collect()
}
