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

    /// Parse Camelot (`8A`) or common musical / Rekordbox `Tonality` strings (`Am`, `C#m`, `F`).
    pub fn parse_flexible(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some(k) = Self::parse(trimmed) {
            return Some(k);
        }
        musical_to_camelot(trimmed)
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

/// Map musical key names (Rekordbox Tonality) to Camelot.
fn musical_to_camelot(s: &str) -> Option<CamelotKey> {
    let t = s.trim().to_lowercase().replace('♭', "b").replace('♯', "#");
    // Normalize "a minor" / "amin" / "am" / "a-m".
    let compact: String = t
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
        .collect();
    let (root, minor) = if let Some(r) = compact.strip_suffix("minor") {
        (r, true)
    } else if let Some(r) = compact.strip_suffix("maj") {
        (r, false)
    } else if let Some(r) = compact.strip_suffix("major") {
        (r, false)
    } else if compact.ends_with('m') && !compact.ends_with("am") && compact.len() > 1 {
        // "cm", "c#m", "bbm" — but not lone "am" handled below
        (&compact[..compact.len() - 1], true)
    } else if compact.ends_with('m') {
        (&compact[..compact.len() - 1], true)
    } else {
        (compact.as_str(), false)
    };
    let pc = pitch_class_from_name(root)?;
    Some(from_pitch_class(pc, minor))
}

fn pitch_class_from_name(name: &str) -> Option<u8> {
    match name {
        "c" => Some(0),
        "c#" | "db" => Some(1),
        "d" => Some(2),
        "d#" | "eb" => Some(3),
        "e" => Some(4),
        "f" => Some(5),
        "f#" | "gb" => Some(6),
        "g" => Some(7),
        "g#" | "ab" => Some(8),
        "a" => Some(9),
        "a#" | "bb" => Some(10),
        "b" => Some(11),
        _ => None,
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

    #[test]
    fn parse_flexible_musical_and_camelot() {
        assert_eq!(
            CamelotKey::parse_flexible("8A").unwrap(),
            CamelotKey::parse("8A").unwrap()
        );
        // A minor → 8A; C major → 8B
        assert_eq!(
            CamelotKey::parse_flexible("Am").unwrap().as_str(),
            "8A"
        );
        assert_eq!(
            CamelotKey::parse_flexible("C").unwrap().as_str(),
            "8B"
        );
        assert_eq!(
            CamelotKey::parse_flexible("F#m").unwrap().as_str(),
            from_pitch_class(6, true).as_str()
        );
        assert!(CamelotKey::parse_flexible("").is_none());
        assert!(CamelotKey::parse_flexible("not-a-key").is_none());
    }
}
