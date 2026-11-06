//! Dinero en **centavos** (`i64`), sin coma flotante.
//!
//! Regla del proyecto (ADR-07): todos los importes se guardan y calculan como
//! enteros de centavos. Este módulo concentra el parseo tolerante de la entrada
//! del usuario y el formateo para la UI.

use std::fmt;
use std::str::FromStr;

/// Importe monetario expresado en centavos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Cents(pub i64);

impl Cents {
    /// Importe cero.
    pub const ZERO: Self = Self(0);

    /// Construye desde unidades enteras (pesos/dólares).
    #[must_use]
    pub const fn from_units(units: i64) -> Self {
        Self(units.saturating_mul(100))
    }

    /// Unidades enteras (truncando los centavos).
    #[must_use]
    pub const fn units(self) -> i64 {
        self.0 / 100
    }

    /// Centavos restantes (puede ser negativo).
    #[must_use]
    pub const fn remainder(self) -> i64 {
        self.0 % 100
    }

    /// Convierte un `REAL` legacy (pesos con decimales) a centavos redondeados.
    #[must_use]
    pub fn from_legacy_real(value: f64) -> Self {
        if !value.is_finite() {
            return Self::ZERO;
        }
        Self((value * 100.0).round() as i64)
    }

    /// Suma segura (satura en los límites de `i64`).
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Resta segura.
    #[must_use]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Multiplica por un entero (por ejemplo, horas de alquiler).
    #[must_use]
    pub const fn saturating_mul(self, factor: i64) -> Self {
        Self(self.0.saturating_mul(factor))
    }

    /// `true` si el importe es cero.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Formato argentino: `$1.234,56`.
    #[must_use]
    pub fn display_ars(self) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        let units = abs / 100;
        let cents = abs % 100;
        let mut grouped = String::new();
        let digits = units.to_string();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                grouped.push('.');
            }
            grouped.push(ch);
        }
        format!("{sign}${grouped},{cents:02}")
    }

    /// Parsea lo que escribe el usuario en un campo `Money`.
    ///
    /// Acepta `1500`, `1500.50`, `1.500,50`, `1,500.50`, `$1.500,50` y
    /// variantes con signo negativo. Si hay dos separadores, el último es el
    /// decimal; con un solo separador decide por la cantidad de dígitos finales.
    pub fn parse_user_input(input: &str) -> Result<Self, MoneyError> {
        let cleaned: String = input
            .trim()
            .chars()
            .filter(|c| !matches!(c, '$' | ' ' | '_') && !c.is_alphabetic())
            .collect();
        if cleaned.is_empty() {
            return Err(MoneyError::Empty);
        }

        let negative = cleaned.starts_with('-');
        let body = cleaned.trim_start_matches(['-', '+']);
        if body.is_empty() {
            return Err(MoneyError::Invalid(cleaned.clone()));
        }
        if !body
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
        {
            return Err(MoneyError::Invalid(cleaned.clone()));
        }

        let dots = body.matches('.').count();
        let commas = body.matches(',').count();
        let (int_part, dec_part) = if dots > 0 && commas > 0 {
            let last_dot = body.rfind('.').unwrap_or(0);
            let last_comma = body.rfind(',').unwrap_or(0);
            let (decimal_sep, thousands_sep) = if last_dot > last_comma {
                ('.', ',')
            } else {
                (',', '.')
            };
            let (int_text, dec_text) = body.rsplit_once(decimal_sep).unwrap_or((body, ""));
            if !valid_thousands(int_text, thousands_sep) {
                return Err(MoneyError::Invalid(cleaned));
            }
            (int_text, dec_text)
        } else if dots == 1 && commas == 0 {
            let (a, b) = body.split_once('.').unwrap_or((body, ""));
            if b.len() == 3
                && b.chars().all(|c| c.is_ascii_digit())
                && (1..=3).contains(&a.len())
                && !a.is_empty()
            {
                // "1.500" es miles, no 1.5
                if !valid_thousands(body, '.') {
                    return Err(MoneyError::Invalid(cleaned));
                }
                (body, "")
            } else {
                (a, b)
            }
        } else if commas == 1 && dots == 0 {
            let (a, b) = body.split_once(',').unwrap_or((body, ""));
            if b.len() == 3
                && b.chars().all(|c| c.is_ascii_digit())
                && (1..=3).contains(&a.len())
                && !a.is_empty()
            {
                if !valid_thousands(body, ',') {
                    return Err(MoneyError::Invalid(cleaned));
                }
                (body, "")
            } else {
                (a, b)
            }
        } else if dots > 0 || commas > 0 {
            let sep = if dots > 0 { '.' } else { ',' };
            if !valid_thousands(body, sep) {
                return Err(MoneyError::Invalid(cleaned));
            }
            (body, "")
        } else {
            (body, "")
        };

        let int_digits: String = int_part.chars().filter(char::is_ascii_digit).collect();
        if int_digits.is_empty() && dec_part.is_empty() {
            return Err(MoneyError::Invalid(cleaned));
        }
        let units: i64 = if int_digits.is_empty() {
            0
        } else {
            int_digits
                .parse::<i64>()
                .map_err(|_| MoneyError::OutOfRange(cleaned.clone()))?
        };

        let mut cents: i64 = 0;
        if !dec_part.is_empty() {
            let digits: String = dec_part.chars().filter(char::is_ascii_digit).collect();
            if digits.len() > 2 {
                return Err(MoneyError::Invalid(cleaned));
            }
            let padded = format!("{digits:0<2}");
            cents = padded
                .parse::<i64>()
                .map_err(|_| MoneyError::Invalid(cleaned.clone()))?;
        }

        let total = units
            .checked_mul(100)
            .and_then(|v| v.checked_add(cents))
            .ok_or_else(|| MoneyError::OutOfRange(cleaned.clone()))?;
        Ok(Self(if negative { -total } else { total }))
    }
}

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}${}.{:02}", abs / 100, abs % 100)
    }
}

impl FromStr for Cents {
    type Err = MoneyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_user_input(s)
    }
}

impl From<i64> for Cents {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<Cents> for i64 {
    fn from(value: Cents) -> Self {
        value.0
    }
}

/// Errores de parseo de importes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    /// La entrada quedó vacía después de limpiar separadores.
    #[error("importe vacío")]
    Empty,
    /// Texto con formato no reconocido.
    #[error("importe inválido: {0}")]
    Invalid(String),
    /// El valor no cabe en `i64`.
    #[error("importe fuera de rango: {0}")]
    OutOfRange(String),
}

/// Valida el agrupamiento de miles: primer grupo de 1 a 3 dígitos y el
/// resto de exactamente 3.
fn valid_thousands(text: &str, separator: char) -> bool {
    let parts: Vec<&str> = text.split(separator).collect();
    if parts.len() == 1 {
        return !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
    }
    if parts
        .iter()
        .any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    (1..=3).contains(&parts[0].len()) && parts[1..].iter().all(|part| part.len() == 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_spec_format() {
        assert_eq!(Cents(150_050).to_string(), "$1500.50");
        assert_eq!(Cents(-5).to_string(), "-$0.05");
    }

    #[test]
    fn display_ars_groups_thousands() {
        assert_eq!(Cents(123_456).display_ars(), "$1.234,56");
        assert_eq!(Cents(-123_456).display_ars(), "-$1.234,56");
    }

    #[test]
    fn parses_plain_units() {
        assert_eq!(Cents::parse_user_input("1500").unwrap(), Cents(150_000));
        assert_eq!(Cents::parse_user_input("0").unwrap(), Cents::ZERO);
    }

    #[test]
    fn parses_decimal_forms() {
        assert_eq!(Cents::parse_user_input("1500.50").unwrap(), Cents(150_050));
        assert_eq!(Cents::parse_user_input("1.500,50").unwrap(), Cents(150_050));
        assert_eq!(Cents::parse_user_input("1,500.50").unwrap(), Cents(150_050));
        assert_eq!(
            Cents::parse_user_input("$ 1.500,5").unwrap(),
            Cents(150_050)
        );
    }

    #[test]
    fn thousands_separator_is_not_decimal() {
        assert_eq!(Cents::parse_user_input("1.500").unwrap(), Cents(150_000));
        assert_eq!(Cents::parse_user_input("12.345").unwrap(), Cents(1_234_500));
        assert_eq!(
            Cents::parse_user_input("1.234.567").unwrap(),
            Cents(123_456_700)
        );
        assert_eq!(
            Cents::parse_user_input("1,234,567.89").unwrap(),
            Cents(123_456_789)
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(Cents::parse_user_input("").is_err());
        assert!(Cents::parse_user_input("abc").is_err());
        assert!(Cents::parse_user_input("1.2.3.4").is_err());
        assert!(Cents::parse_user_input("12,345.678").is_err());
    }

    #[test]
    fn negatives_and_legacy_real() {
        assert_eq!(Cents::parse_user_input("-12,50").unwrap(), Cents(-1_250));
        assert_eq!(Cents::from_legacy_real(12.5), Cents(1_250));
        assert_eq!(Cents::from_legacy_real(f64::NAN), Cents::ZERO);
    }
}
