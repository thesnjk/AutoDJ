//! Camelot wheel helpers for harmonic mixing.

use serde::{Deserialize, Serialize};

/// Camelot key like `8A` (minor) or `8B` (major).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CamelotKey {
    /// Number 1–12 on the wheel.
    pub number: u8,
    /// `true` = A (minor), `false` = B (major).
    pub minor: bool,
}

impl CamelotKey {
    /// Parse strings like `8A`, `12B` (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_uppercase();
        if s.len() < 2 {
            return None;
        }
        let (num, letter) = s.split_at(s.len() - 1);
        let number: u8 = num.parse().ok()?;
        if !(1..=12).contains(&number) {
            return None;
        }
        let minor = match letter {
            "A" => true,
            "B" => false,
            _ => return None,
        };
        Some(Self { number, minor })
    }

    /// Display as `8A` / `8B`.
    pub fn as_str(self) -> String {
        format!("{}{}", self.number, if self.minor { "A" } else { "B" })
    }
}

/// Harmonic distance on the Camelot wheel (0 = same / relative, higher = worse).
pub fn camelot_distance(a: CamelotKey, b: CamelotKey) -> u8 {
    if a == b {
        return 0;
    }
    // Relative major/minor (same number, flip letter) is a classic mix.
    if a.number == b.number {
        return 1;
    }
    let dn = {
        let d = (a.number as i16 - b.number as i16).abs();
        d.min(12 - d) as u8
    };
    if a.minor == b.minor {
        dn
    } else {
        dn.saturating_add(1)
    }
}

/// Map rough pitch-class energy peak (0–11, C=0) + major/minor guess to Camelot.
pub fn from_pitch_class(pc: u8, minor: bool) -> CamelotKey {
    // Open Key / Camelot mapping for C major = 8B, A minor = 8A.
    // Pitch class 0 = C → major 8B, minor would be related differently;
    // Simplified table for MVP:
    const MAJOR: [u8; 12] = [8, 3, 10, 5, 12, 7, 2, 9, 4, 11, 6, 1]; // C..B major
    const MINOR: [u8; 12] = [5, 12, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10]; // C..B minor
    let number = if minor {
        MINOR[(pc % 12) as usize]
    } else {
        MAJOR[(pc % 12) as usize]
    };
    CamelotKey { number, minor }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_distance() {
        let a = CamelotKey::parse("8A").unwrap();
        let b = CamelotKey::parse("8B").unwrap();
        let c = CamelotKey::parse("9A").unwrap();
        assert_eq!(camelot_distance(a, a), 0);
        assert_eq!(camelot_distance(a, b), 1);
        assert_eq!(camelot_distance(a, c), 1);
    }
}
