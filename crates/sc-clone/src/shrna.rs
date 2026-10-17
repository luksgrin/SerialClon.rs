//! Make shRNA (User Manual §III.E, changelog 1.01 / 1.3): from a stored
//! layout and an siRNA core, the oligonucleotides and the insert.

use sc_seq::alphabet::reverse_complement;
use sc_seq::{Ends, Feature, Overhang, Rgb, Sequence, Source, Strand, Topology};
use serde::{Deserialize, Serialize};

/// A vector-specific layout (pSUPER, pSIREN…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    /// Name.
    pub name: String,
    /// 5' overhang of the top oligo (e.g. `GATC` for BglII).
    pub left_overhang: String,
    /// Sequence before the sense arm.
    pub prefix: String,
    /// The loop.
    pub loop_seq: String,
    /// Sequence after the antisense arm (typically the pol III terminator).
    pub suffix: String,
    /// 5' overhang of the bottom oligo (e.g. `AGCT` for HindIII).
    pub right_overhang: String,
}

impl Layout {
    /// The pSUPER layout exactly as the original stores it
    /// (`pSUPER,5GATC,CCC,TTCAAGAGA,TTTTTG,5AGCT`): BglII / HindIII 5' ends,
    /// CCC prefix, TTCAAGAGA loop, TTTTTG terminator.
    pub fn psuper() -> Layout {
        Layout {
            name: "pSUPER".into(),
            left_overhang: "GATC".into(),
            prefix: "CCC".into(),
            loop_seq: "TTCAAGAGA".into(),
            suffix: "TTTTTG".into(),
            right_overhang: "AGCT".into(),
        }
    }
}

/// The result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shrna {
    /// Top oligo 5'→3'.
    pub top_oligo: Vec<u8>,
    /// Bottom oligo 5'→3'.
    pub bottom_oligo: Vec<u8>,
    /// The insert with its cohesive ends, ready for Build a Construct.
    pub sequence: Sequence,
}

fn clean(s: &str) -> Vec<u8> {
    s.bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect()
}

/// Builds the hairpin: prefix + sense + loop + antisense + suffix.
/// `mutations` are 1-based positions in the sense arm whose base is swapped
/// (A↔G, C↔T) to weaken the hairpin, as the original allowed.
pub fn shrna(name: &str, layout: &Layout, core: &str, mutations: &[usize]) -> Shrna {
    let core = clean(core);
    let mut sense = core.clone();
    for &p in mutations {
        if p >= 1 && p <= sense.len() {
            sense[p - 1] = match sense[p - 1] {
                b'A' => b'G',
                b'G' => b'A',
                b'C' => b'T',
                b'T' => b'C',
                other => other,
            };
        }
    }
    let antisense = reverse_complement(&core);
    let (prefix, lp, suffix) = (
        clean(&layout.prefix),
        clean(&layout.loop_seq),
        clean(&layout.suffix),
    );
    let mut insert = prefix.clone();
    insert.extend_from_slice(&sense);
    insert.extend_from_slice(&lp);
    insert.extend_from_slice(&antisense);
    insert.extend_from_slice(&suffix);
    let (l, r) = (clean(&layout.left_overhang), clean(&layout.right_overhang));
    let mut top = l.clone();
    top.extend_from_slice(&insert);
    let mut bottom = r.clone();
    bottom.extend_from_slice(&reverse_complement(&insert));
    let mut extent = l.clone();
    extent.extend_from_slice(&insert);
    extent.extend_from_slice(&reverse_complement(&r));
    let off = l.len() as u32;
    let mk = |name: &str, s: usize, e: usize, color: Rgb| Feature {
        name: name.into(),
        description: String::new(),
        kind: "misc_feature".into(),
        start: off + s as u32 + 1,
        end: off + e as u32,
        strand: Strand::Forward,
        show: true,
        protect: false,
        arrow: false,
        color,
    };
    let p = prefix.len();
    let features = vec![
        mk("sense", p, p + sense.len(), Rgb(46, 160, 67)),
        mk(
            "loop",
            p + sense.len(),
            p + sense.len() + lp.len(),
            Rgb(240, 165, 58),
        ),
        mk(
            "antisense",
            p + sense.len() + lp.len(),
            p + sense.len() + lp.len() + antisense.len(),
            Rgb(209, 36, 47),
        ),
    ];
    let sequence = Sequence {
        name: name.to_owned(),
        bases: extent,
        seq_type: sc_seq::SeqType::Dna,
        topology: Topology::Linear,
        comment: format!(
            "shRNA insert ({} layout) for siRNA {}{}\rTop oligo 5'-{}-3'\rBottom oligo 5'-{}-3'",
            layout.name,
            String::from_utf8_lossy(&core),
            if mutations.is_empty() {
                String::new()
            } else {
                format!(", point mutations at {mutations:?} in the sense arm")
            },
            String::from_utf8_lossy(&top),
            String::from_utf8_lossy(&bottom)
        ),
        features,
        ends: Ends {
            left: if l.is_empty() {
                Overhang::BLUNT
            } else {
                Overhang::five_prime(&l)
            },
            right: if r.is_empty() {
                Overhang::BLUNT
            } else {
                Overhang::five_prime(&r)
            },
        },
        source: Source::New,
        strider_header: None,
        snapgene_extras: None,
    };
    Shrna {
        top_oligo: top,
        bottom_oligo: bottom,
        sequence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psuper_hairpin() {
        let core = "GACTTCAGCTTGGATCCAA";
        let s = shrna("sh", &Layout::psuper(), core, &[]);
        let expected_insert = format!(
            "CCC{core}TTCAAGAGA{}TTTTTG",
            String::from_utf8(reverse_complement(core.as_bytes())).unwrap()
        );
        assert_eq!(s.top_oligo, format!("GATC{expected_insert}").as_bytes());
        assert!(s.bottom_oligo.starts_with(b"AGCT"));
        assert_eq!(s.bottom_oligo.len(), 4 + expected_insert.len());
        assert_eq!(s.sequence.ends.left, Overhang::five_prime(b"GATC"));
        assert_eq!(s.sequence.ends.right, Overhang::five_prime(b"AGCT"));
        assert_eq!(s.sequence.features.len(), 3);
        let m = shrna("sh", &Layout::psuper(), core, &[1]);
        assert_eq!(&m.top_oligo[7..8], b"A");
        assert!(m.sequence.comment.contains("point mutations"));
    }
}
