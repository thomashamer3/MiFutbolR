//! Codificación legacy de selección múltiple de canchas (§9.2).
//!
//! El proyecto C guarda un único entero; aquí se traduce a `Vec<TipoCancha>` y
//! `Vec<Superficie>` sin perder compatibilidad bidireccional.

use super::models::{Superficie, TipoCancha};

const TIPO_MASK_OFFSET: i64 = 1000;
const SUPERFICIE_MASK_OFFSET: i64 = 2000;

/// Decodifica el código legacy de tipos de cancha.
#[must_use]
pub fn decode_tipos(code: i64) -> Vec<TipoCancha> {
    if code >= TIPO_MASK_OFFSET {
        let mask = code - TIPO_MASK_OFFSET;
        let mut tipos = Vec::new();
        for (bit, tipo) in [
            (1 << 0, TipoCancha::F5),
            (1 << 1, TipoCancha::F7),
            (1 << 2, TipoCancha::F8),
            (1 << 3, TipoCancha::F11),
            (1 << 4, TipoCancha::Otro),
        ] {
            if mask & bit != 0 {
                tipos.push(tipo);
            }
        }
        tipos
    } else {
        match code {
            5 => vec![TipoCancha::F5],
            7 => vec![TipoCancha::F7],
            8 => vec![TipoCancha::F8],
            11 => vec![TipoCancha::F11],
            99 => vec![TipoCancha::Otro],
            _ => Vec::new(),
        }
    }
}

/// Codifica los tipos seleccionados al entero legacy.
#[must_use]
pub fn encode_tipos(tipos: &[TipoCancha]) -> i64 {
    if tipos.is_empty() {
        return 0;
    }
    if tipos.len() == 1 {
        return tipos[0].as_i64();
    }
    let mut mask = 0;
    for tipo in tipos {
        mask |= match tipo {
            TipoCancha::F5 => 1 << 0,
            TipoCancha::F7 => 1 << 1,
            TipoCancha::F8 => 1 << 2,
            TipoCancha::F11 => 1 << 3,
            TipoCancha::Otro => 1 << 4,
        };
    }
    TIPO_MASK_OFFSET + mask
}

/// Decodifica el código legacy de superficies.
#[must_use]
pub fn decode_superficies(code: i64) -> Vec<Superficie> {
    if code >= SUPERFICIE_MASK_OFFSET {
        let mask = code - SUPERFICIE_MASK_OFFSET;
        let mut superficies = Vec::new();
        for (bit, superficie) in [
            (1 << 0, Superficie::Natural),
            (1 << 1, Superficie::Sintetico),
            (1 << 2, Superficie::Cemento),
            (1 << 3, Superficie::Otra),
        ] {
            if mask & bit != 0 {
                superficies.push(superficie);
            }
        }
        superficies
    } else {
        match code {
            0 => vec![Superficie::NoSe],
            1 => vec![Superficie::Natural],
            2 => vec![Superficie::Sintetico],
            3 => vec![Superficie::Cemento],
            4 => vec![Superficie::Otra],
            _ => vec![Superficie::NoSe],
        }
    }
}

/// Codifica las superficies seleccionadas al entero legacy.
#[must_use]
pub fn encode_superficies(superficies: &[Superficie]) -> i64 {
    let mut valores: Vec<i64> = superficies
        .iter()
        .map(|superficie| superficie.as_i64())
        .filter(|codigo| *codigo > 0)
        .collect();
    valores.sort_unstable();
    valores.dedup();
    match valores.as_slice() {
        [] => 0,
        [unico] => *unico,
        _ => {
            let mut mask = 0;
            for codigo in &valores {
                mask |= match codigo {
                    1 => 1 << 0,
                    2 => 1 << 1,
                    3 => 1 << 2,
                    4 => 1 << 3,
                    _ => 0,
                };
            }
            SUPERFICIE_MASK_OFFSET + mask
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipos_roundtrip() {
        for code in [5, 7, 8, 11, 99, 1000 + 1 + 2, 1000 + 8 + 16] {
            let tipos = decode_tipos(code);
            assert_eq!(encode_tipos(&tipos), code, "código {code}");
        }
        assert_eq!(encode_tipos(&[]), 0);
        assert!(decode_tipos(0).is_empty());
        assert_eq!(decode_tipos(1000), Vec::new());
    }

    #[test]
    fn superficies_roundtrip() {
        for code in [0, 1, 2, 3, 4, 2000 + 1 + 2, 2000 + 4 + 8] {
            let superficies = decode_superficies(code);
            assert_eq!(encode_superficies(&superficies), code, "código {code}");
        }
        assert_eq!(decode_superficies(0), vec![Superficie::NoSe]);
        assert_eq!(encode_superficies(&[Superficie::NoSe]), 0);
    }

    #[test]
    fn multiple_selection_sets_mask() {
        let tipos = vec![TipoCancha::F5, TipoCancha::F11];
        assert_eq!(encode_tipos(&tipos), 1000 + 1 + 8);
        let superficies = vec![Superficie::Natural, Superficie::Cemento];
        assert_eq!(encode_superficies(&superficies), 2000 + 1 + 4);
    }
}
