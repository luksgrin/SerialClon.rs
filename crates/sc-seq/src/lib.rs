//! Sequence model for SerialClon.rs.
//!
//! This crate owns everything about a sequence that does not touch a file or a
//! window: topology, alphabets, translation and genetic codes, Tm / MW / pI /
//! %GC, ORF search, bisulfite conversion and reverse translation.
//!
//! Milestone **M1** fills it in. What is here now is the vocabulary shared by
//! the other crates: the sequence *type* and *topology* codes, which are the
//! first bytes of the `.xdna` header (`PLAN.md` §1.4) and are displayed as-is in
//! the Sequence Window (`reference/ui/01-sequence-window.png`, fields 3 and 5).

use serde::{Deserialize, Serialize};

/// The kind of molecule a sequence describes, as encoded by DNA Strider and
/// shown in the Sequence Window "Type" field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeqType {
    /// Type code 0.
    Unknown,
    /// Type code 1: plain DNA (`ACGT` only).
    Dna,
    /// Type code 2: degenerate DNA (full IUPAC alphabet).
    DegenerateDna,
    /// Type code 3.
    Rna,
    /// Type code 4.
    Protein,
}

impl SeqType {
    /// Decodes the header byte at offset 1 of an `.xdna` file.
    pub fn from_xdna_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Unknown),
            1 => Some(Self::Dna),
            2 => Some(Self::DegenerateDna),
            3 => Some(Self::Rna),
            4 => Some(Self::Protein),
            _ => None,
        }
    }

    /// Encodes the header byte at offset 1 of an `.xdna` file.
    pub fn to_xdna_byte(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Dna => 1,
            Self::DegenerateDna => 2,
            Self::Rna => 3,
            Self::Protein => 4,
        }
    }
}

/// Whether the ends of a sequence are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Topology {
    /// Topology code 0.
    Linear,
    /// Topology code 1.
    Circular,
}

impl Topology {
    /// Decodes the header byte at offset 2 of an `.xdna` file.
    pub fn from_xdna_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Linear),
            1 => Some(Self::Circular),
            _ => None,
        }
    }

    /// Encodes the header byte at offset 2 of an `.xdna` file.
    pub fn to_xdna_byte(self) -> u8 {
        match self {
            Self::Linear => 0,
            Self::Circular => 1,
        }
    }

    /// The other topology — what the Sequence Window's Linearize / Circularize
    /// context-menu entry switches to.
    pub fn toggled(self) -> Self {
        match self {
            Self::Linear => Self::Circular,
            Self::Circular => Self::Linear,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_type_round_trips_through_xdna_bytes() {
        for byte in 0..=4u8 {
            let t = SeqType::from_xdna_byte(byte).expect("valid code");
            assert_eq!(t.to_xdna_byte(), byte);
        }
        assert_eq!(SeqType::from_xdna_byte(5), None);
    }

    #[test]
    fn topology_round_trips_and_toggles() {
        assert_eq!(Topology::from_xdna_byte(0), Some(Topology::Linear));
        assert_eq!(Topology::from_xdna_byte(1), Some(Topology::Circular));
        assert_eq!(Topology::from_xdna_byte(2), None);
        assert_eq!(Topology::Linear.toggled(), Topology::Circular);
        assert_eq!(Topology::Circular.toggled().to_xdna_byte(), 0);
    }

    #[test]
    fn reference_fixtures_use_known_codes() {
        // Every shipped .xdna fixture must decode with the codes above.
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");
        let mut seen = 0;
        for entry in std::fs::read_dir(dir).expect("reference/sequences exists") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("xdna") {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(bytes[0], 0, "{}: version byte", path.display());
            assert!(
                SeqType::from_xdna_byte(bytes[1]).is_some(),
                "{}",
                path.display()
            );
            assert!(
                Topology::from_xdna_byte(bytes[2]).is_some(),
                "{}",
                path.display()
            );
            seen += 1;
        }
        assert_eq!(
            seen, 7,
            "expected the 7 .xdna fixtures listed in reference/README.md"
        );
    }
}
