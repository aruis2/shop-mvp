//! # u32 ↔ i32 Converter
//!
//! Conversii zero-cost între `u32` și `i32` pentru a stoca 4 miliarde de valori
//! în PostgreSQL folosind doar 4 bytes (INTEGER).
//!
//! ## Problemă
//! PostgreSQL nu are tip unsigned. `INTEGER` e `i32` și stochează doar 2 miliarde.
//! Dar dacă faci o conversie matematică simplă, poți stoca 4 miliarde.
//!
//! ## Soluție
//! Shiftăm domeniul cu 2.147.483.648 (i32::MIN.abs()):
//! - `u32(0)` → `i32(-2147483648)`
//! - `u32(2147483648)` → `i32(0)`
//! - `u32(4294967295)` → `i32(2147483647)`
//!
//! ## Utilizare rapidă
//! ```rust
//! use u32_i32_converter::{u32_to_i32, i32_to_u32};
//!
//! let original: u32 = 4_000_000_000;
//! let stored: i32 = u32_to_i32(original);   // pentru PostgreSQL
//! let recovered: u32 = i32_to_u32(stored);  // înapoi în Rust
//! assert_eq!(original, recovered);
//! ```

use std::fmt;

// ============================================================================
// CONSTANTE
// ============================================================================

/// Constanta magică pentru shiftarea domeniului.
/// i32::MIN = -2_147_483_648, iar valoarea absolută e 2_147_483_648.
const SHIFT: u32 = 2_147_483_648;

// ============================================================================
// FUNCȚII DE CONVERSIE (cele mai rapide, zero-cost)
// ============================================================================

/// Convertește `u32` → `i32` pentru stocare în PostgreSQL.
///
/// # Exemplu
/// ```rust
/// use u32_i32_converter::u32_to_i32;
///
/// assert_eq!(u32_to_i32(0), -2_147_483_648);
/// assert_eq!(u32_to_i32(2_147_483_648), 0);
/// assert_eq!(u32_to_i32(4_294_967_295), 2_147_483_647);
/// ```
#[inline(always)]
pub fn u32_to_i32(val: u32) -> i32 {
    val.wrapping_sub(SHIFT) as i32
}

/// Convertește `i32` → `u32` după citirea din PostgreSQL.
///
/// # Exemplu
/// ```rust
/// use u32_i32_converter::i32_to_u32;
///
/// assert_eq!(i32_to_u32(-2_147_483_648), 0);
/// assert_eq!(i32_to_u32(0), 2_147_483_648);
/// assert_eq!(i32_to_u32(2_147_483_647), 4_294_967_295);
/// ```
#[inline(always)]
pub fn i32_to_u32(val: i32) -> u32 {
    (val as u32).wrapping_add(SHIFT)
}

// ============================================================================
// TIPUL PGU32 (wrapper type-safe)
// ============================================================================

/// Un `u32` care se stochează automat ca `i32` în PostgreSQL.
///
/// Oferă type safety: nu poți confunda accidental un `u32` brut cu o valoare
/// care trebuie convertită pentru baza de date.
///
/// ## Exemplu
/// ```rust
/// use u32_i32_converter::PgU32;
///
/// let id = PgU32::new(42);
/// let pg_val: i32 = id.to_pg_i32();  // pentru PostgreSQL
/// let recovered = PgU32::from_pg_i32(pg_val);
/// assert_eq!(id, recovered);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PgU32(u32);

impl PgU32 {
    /// Creează un `PgU32` dintr-un `u32`.
    #[inline(always)]
    pub const fn new(val: u32) -> Self {
        PgU32(val)
    }

    /// Returnează valoarea `u32`.
    #[inline(always)]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Convertește în `i32` pentru PostgreSQL.
    #[inline(always)]
    pub const fn to_pg_i32(self) -> i32 {
        (self.0.wrapping_sub(SHIFT)) as i32
    }

    /// Creează din `i32` citit din PostgreSQL.
    #[inline(always)]
    pub const fn from_pg_i32(val: i32) -> Self {
        PgU32((val as u32).wrapping_add(SHIFT))
    }
}

// --- Implementări de conversie pentru ergonomie ---

impl From<u32> for PgU32 {
    #[inline(always)]
    fn from(val: u32) -> Self {
        PgU32(val)
    }
}

impl From<PgU32> for u32 {
    #[inline(always)]
    fn from(val: PgU32) -> Self {
        val.0
    }
}

impl From<i32> for PgU32 {
    #[inline(always)]
    fn from(val: i32) -> Self {
        PgU32::from_pg_i32(val)
    }
}

impl From<PgU32> for i32 {
    #[inline(always)]
    fn from(val: PgU32) -> Self {
        val.to_pg_i32()
    }
}

impl fmt::Debug for PgU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PgU32({})", self.0)
    }
}

impl fmt::Display for PgU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// TESTE
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_functions() {
        // Limite
        assert_eq!(u32_to_i32(0), i32::MIN);
        assert_eq!(i32_to_u32(i32::MIN), 0);
        assert_eq!(u32_to_i32(u32::MAX), i32::MAX);
        assert_eq!(i32_to_u32(i32::MAX), u32::MAX);
        assert_eq!(u32_to_i32(SHIFT), 0);
        assert_eq!(i32_to_u32(0), SHIFT);
    }

    #[test]
    fn test_roundtrip_all_boundaries() {
        let test_values = [
            0u32,
            1,
            42,
            1_000_000,
            2_147_483_647,
            2_147_483_648,
            3_000_000_000,
            4_000_000_000,
            u32::MAX,
        ];

        for &original in &test_values {
            let stored = u32_to_i32(original);
            let recovered = i32_to_u32(stored);
            assert_eq!(original, recovered, "Eșec pentru: {}", original);
        }
    }

    #[test]
    fn test_pg_u32_type() {
        let original = PgU32::new(u32::MAX);
        let stored: i32 = original.to_pg_i32();
        let recovered = PgU32::from_pg_i32(stored);
        assert_eq!(original, recovered);

        // From<u32>
        let from_u32: PgU32 = 42u32.into();
        assert_eq!(from_u32.get(), 42);

        // From<i32>
        let from_i32: PgU32 = (-1i32).into();
        assert_eq!(from_i32.get(), 2_147_483_647);

        // Display
        assert_eq!(format!("{}", PgU32::new(100)), "100");
    }

    #[test]
    fn test_no_collisions() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        // Verificăm ~100k valori distribuite uniform
        for val in (0..=u32::MAX).step_by(43_691) {
            let stored = u32_to_i32(val);
            assert!(seen.insert(stored), "Coliziune la valoarea: {}", val);
        }
    }

    #[test]
    fn test_no_panic() {
        // Verifică că funcțiile nu dau panic pentru valori extreme
        for val in (0..=u32::MAX).step_by(1_000_000) {
            let _ = u32_to_i32(val);
        }
        for val in (i32::MIN..=i32::MAX).step_by(1_000_000) {
            let _ = i32_to_u32(val);
        }
    }

    #[test]
    fn test_ordering_preserved() {
        // Ordonarea se păstrează
        let a = PgU32::new(0);
        let b = PgU32::new(1_000_000);
        let c = PgU32::new(u32::MAX);
        assert!(a < b);
        assert!(b < c);
    }
}
