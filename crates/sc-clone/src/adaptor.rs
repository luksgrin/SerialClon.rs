//! Create an Adaptor (User Manual §III.E): a double-stranded core with
//! optional cohesive extensions, and the two oligonucleotides to order.

use sc_seq::alphabet::reverse_complement;
use sc_seq::{Ends, Overhang, Sequence, Source, Topology};

use crate::ligate::describe;

/// The adaptor: oligos plus an in-memory sequence usable in Build a
/// Construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adaptor {
    /// Top-strand oligo, 5'→3'.
    pub top_oligo: Vec<u8>,
    /// Bottom-strand oligo, 5'→3'.
    pub bottom_oligo: Vec<u8>,
    /// The adaptor as a linear sequence with its cohesive ends.
    pub sequence: Sequence,
}

fn clean(s: &str) -> Vec<u8> {
    s.bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect()
}

/// Builds the adaptor. `left` / `right` are the cohesive extensions as
/// [`Overhang`]s: bases 5'→3' on the strand that carries them.
pub fn adaptor(name: &str, core: &str, left: Overhang, right: Overhang) -> Adaptor {
    let core = clean(core);
    let core_rc = reverse_complement(&core);
    let mut extent = Vec::new();
    let mut top = Vec::new();
    let mut bottom = Vec::new();
    if left.is_five_prime() {
        extent.extend_from_slice(&left.bases);
        top.extend_from_slice(&left.bases);
    } else if !left.is_blunt() {
        extent.extend_from_slice(&reverse_complement(&left.bases));
    }
    extent.extend_from_slice(&core);
    top.extend_from_slice(&core);
    if right.is_five_prime() {
        extent.extend_from_slice(&reverse_complement(&right.bases));
        bottom.extend_from_slice(&right.bases);
    } else if !right.is_blunt() {
        extent.extend_from_slice(&right.bases);
        top.extend_from_slice(&right.bases);
    }
    bottom.extend_from_slice(&core_rc);
    if left.length < 0 {
        bottom.extend_from_slice(&left.bases);
    }
    let sequence = Sequence {
        name: name.to_owned(),
        bases: extent,
        seq_type: sc_seq::SeqType::Dna,
        topology: Topology::Linear,
        comment: format!(
            "Adaptor: core {} with left end {} and right end {}\rTop oligo 5'-{}-3'\rBottom oligo 5'-{}-3'",
            String::from_utf8_lossy(&core),
            describe(&left),
            describe(&right),
            String::from_utf8_lossy(&top),
            String::from_utf8_lossy(&bottom)
        ),
        features: Vec::new(),
        ends: Ends { left, right },
        source: Source::New,
        strider_header: None,
        snapgene_extras: None,
    };
    Adaptor {
        top_oligo: top,
        bottom_oligo: bottom,
        sequence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bamhi_ecori_adaptor() {
        let a = adaptor(
            "ad",
            "gcgcgc",
            Overhang::five_prime(b"GATC"),
            Overhang::five_prime(b"AATT"),
        );
        assert_eq!(a.top_oligo, b"GATCGCGCGC");
        assert_eq!(a.bottom_oligo, b"AATTGCGCGC");
        assert_eq!(a.sequence.bases, b"GATCGCGCGCAATT");
        assert_eq!(a.sequence.ends.left, Overhang::five_prime(b"GATC"));
        assert!(a.sequence.comment.contains("Top oligo 5'-GATCGCGCGC-3'"));
    }

    #[test]
    fn three_prime_and_blunt() {
        let a = adaptor(
            "ad",
            "ACGT",
            Overhang::three_prime(b"TGCA"),
            Overhang::BLUNT,
        );
        assert_eq!(a.top_oligo, b"ACGT");
        assert_eq!(a.bottom_oligo, b"ACGTTGCA");
        assert_eq!(a.sequence.bases, b"TGCAACGT");
    }
}
