//! Extract Fragment (⌘6): from a selection, or between one or two
//! restriction sites chosen on the Graphic Map (User Manual §IV.C).

use sc_enzyme::{Fragment, fragment_between};
use sc_seq::alphabet::reverse_complement;
use sc_seq::{Ends, Feature, Overhang, Sequence, Source, Strand, Topology};

/// A site chosen on the map: the enzyme and the cut it makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteRef {
    /// Enzyme name, for the comment.
    pub enzyme: String,
    /// 1-based first base of the recognition site, for the comment.
    pub position: usize,
    /// 0-based top-strand cut.
    pub cut_top: usize,
    /// 0-based bottom-strand cut.
    pub cut_bottom: usize,
}

/// Features of `seq` that lie entirely within `lo..hi` (0-based, half-open,
/// no wrap), re-based to the fragment whose base 0 is `lo - lead` (`lead`
/// is the number of overhang bases before `lo`).
fn inherit(seq: &Sequence, lo: usize, hi: usize, lead: usize) -> Vec<Feature> {
    seq.features
        .iter()
        .filter_map(|f| {
            let (a, b, wraps) = f.lo_hi();
            if wraps {
                return None;
            }
            let (a, b) = (a as usize - 1, b as usize);
            if a < lo || b > hi {
                return None;
            }
            let shift = |p: u32| (p as usize - 1 - lo + lead) as u32 + 1;
            Some(Feature {
                start: shift(f.start),
                end: shift(f.end),
                ..f.clone()
            })
        })
        .collect()
}

fn product(
    seq: &Sequence,
    fragment: Fragment,
    name: String,
    comment: String,
    lo: usize,
    hi: usize,
    wrap_hi: Option<usize>,
) -> Sequence {
    let lead = fragment.start.saturating_sub(lo);
    let mut features = inherit(seq, lo, hi, lead);
    // A fragment crossing the origin also carries the features of its
    // second part, which starts at base 1 of the source.
    if let Some(wrap_hi) = wrap_hi {
        let n = seq.len();
        for f in inherit(seq, 0, wrap_hi, 0) {
            let shift = (n - lo + lead) as u32;
            let g = Feature {
                start: f.start + shift,
                end: f.end + shift,
                ..f
            };
            if g.start as usize <= fragment.bases.len() && g.end as usize <= fragment.bases.len() {
                features.push(g);
            }
        }
    }
    Sequence {
        name,
        bases: fragment.bases,
        seq_type: seq.seq_type,
        topology: Topology::Linear,
        comment,
        features,
        ends: Ends {
            left: fragment.left,
            right: fragment.right,
        },
        source: Source::New,
        strider_header: None,
        snapgene_extras: None,
    }
}

/// Extracts the selected bases `from..to` (0-based, half-open; wrapping the
/// origin of a circular sequence when `from > to`) as a blunt fragment.
pub fn extract_selection(seq: &Sequence, from: usize, to: usize) -> Option<Sequence> {
    let n = seq.len();
    if n == 0 || from > n || to > n {
        return None;
    }
    let bases = if from <= to {
        seq.bases[from..to].to_vec()
    } else if seq.topology.is_circular() {
        let mut v = seq.bases[from..].to_vec();
        v.extend_from_slice(&seq.bases[..to]);
        v
    } else {
        return None;
    };
    if bases.is_empty() {
        return None;
    }
    let (lo, hi, wrap_hi) = if from <= to {
        (from, to, None)
    } else {
        (from, n, Some(to))
    };
    let fragment = Fragment {
        start: from,
        end: to,
        bases,
        left: Overhang::BLUNT,
        right: Overhang::BLUNT,
    };
    // The original's Sequence Window (`SeqWindow.GoGenerateFragment`):
    // `Fragment extracted from <name> [between <from> and <to>]`.
    let comment = format!(
        "Fragment extracted from {} [between {} and {}]",
        seq.name,
        from + 1,
        if from <= to { to } else { to + n },
    );
    Some(product(
        seq,
        fragment,
        format!("{} [{}-{}]", seq.name, from + 1, to),
        comment,
        lo,
        hi,
        wrap_hi,
    ))
}

/// Extracts the fragment between two sites (or, on a circular sequence, the
/// whole linearised plasmid when both sites are the same one). `clockwise`
/// is the CW/CCW toggle: counter-clockwise takes the complementary arc.
/// On a linear sequence `clockwise == false` means "antisense": the fragment
/// is returned antiparallel with its ends swapped.
pub fn extract_between(
    seq: &Sequence,
    begin: &SiteRef,
    end: &SiteRef,
    clockwise: bool,
) -> Option<Sequence> {
    let n = seq.len();
    let (a, b) = (
        (begin.cut_top, begin.cut_bottom),
        (end.cut_top, end.cut_bottom),
    );
    // CW takes the arc from `begin` to `end`, CCW the complementary one; on
    // a linear sequence "antisense" returns the antiparallel fragment.
    let (first, second) = match (seq.topology, clockwise) {
        (Topology::Circular, true) | (Topology::Linear, _) => (a, b),
        (Topology::Circular, false) => (b, a),
    };
    let mut fragment = fragment_between(&seq.bases, seq.topology, first, second)?;
    let lo = first.0.min(first.1);
    let hi = second.0.max(second.1);
    // Crossing the origin: the first part runs to the end, the second from
    // base 1 to `hi` (a single site gives the whole circle).
    let wraps = seq.topology.is_circular() && (hi < lo || first == second);
    let (lo, hi, wrap_hi) = if wraps {
        (lo, n, Some(hi))
    } else {
        (lo, hi, None)
    };
    // The original's Build a Construct window (`CircGraphMap.GoGenerateFragment`)
    // writes `Fragment extracted from <name> [<size>  nt] : (<enzyme 5'>[<pos>] /
    // <enzyme 3'>[<pos>])` from its labels: the source name, the fragment size
    // label (`<n>  nt`, two spaces) and the two extremity labels, which get
    // `-Blunt(filled)` / `-Blunt(trimmed)` when an end is processed (see
    // [`crate::ligate::blunt_with`]). The orientation is not recorded.
    let comment = format!(
        "Fragment extracted from {} [{}  nt] : ({}[{}] / {}[{}])",
        seq.name,
        fragment.bases.len(),
        begin.enzyme,
        begin.position,
        end.enzyme,
        end.position,
    );
    let name = if begin == end {
        format!("{} ({})", seq.name, begin.enzyme)
    } else {
        format!("{} ({}-{})", seq.name, begin.enzyme, end.enzyme)
    };
    if seq.topology == Topology::Linear && !clockwise {
        fragment.bases = reverse_complement(&fragment.bases);
        std::mem::swap(&mut fragment.left, &mut fragment.right);
        let mut out = product(seq, fragment, name, comment, lo, hi, wrap_hi);
        // Features flip with the strand.
        let len = out.len() as u32;
        for f in &mut out.features {
            let (s, e) = (len + 1 - f.start, len + 1 - f.end);
            f.start = s;
            f.end = e;
            f.strand = if f.strand == Strand::Forward {
                Strand::Reverse
            } else {
                Strand::Forward
            };
        }
        return Some(out);
    }
    Some(product(seq, fragment, name, comment, lo, hi, wrap_hi))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_enzyme::{find_sites, library::parse_entry};
    use sc_seq::Rgb;

    fn site(seq: &Sequence, entry: &str) -> SiteRef {
        let e = parse_entry(entry).unwrap();
        let s = &find_sites(&seq.bases, seq.topology, &e)[0];
        SiteRef {
            enzyme: e.name.clone(),
            position: s.position + 1,
            cut_top: s.cut_top,
            cut_bottom: s.cut_bottom,
        }
    }

    fn plasmid() -> Sequence {
        let mut seq = Sequence::new("pTest", b"ttGAATTCaaaGGATCCcc".to_vec());
        seq.topology = Topology::Circular;
        seq.comment = "a test plasmid\rmore".into();
        seq.features.push(Feature {
            name: "ins".into(),
            description: String::new(),
            kind: "misc_feature".into(),
            start: 9,
            end: 11,
            strand: Strand::Forward,
            show: true,
            protect: false,
            arrow: true,
            color: Rgb(1, 2, 3),
        });
        seq
    }

    #[test]
    fn selection_is_blunt_with_provenance() {
        let seq = plasmid();
        let f = extract_selection(&seq, 2, 8).unwrap();
        assert_eq!(f.bases, b"GAATTC");
        assert!(f.ends.is_blunt());
        assert_eq!(f.topology, Topology::Linear);
        // The original's Sequence Window comment.
        assert_eq!(f.comment, "Fragment extracted from pTest [between 3 and 8]");
        // Wrapping the origin.
        assert_eq!(extract_selection(&seq, 17, 2).unwrap().bases, b"cctt");
        assert!(extract_selection(&Sequence::new("l", b"ACGT".to_vec()), 3, 1).is_none());
    }

    #[test]
    fn between_sites_carries_cohesive_ends_and_features() {
        let seq = plasmid();
        let eco = site(&seq, "EcoRI,g/aattc,");
        let bam = site(&seq, "BamHI,g/gatcc,");
        let f = extract_between(&seq, &eco, &bam, true).unwrap();
        assert_eq!(f.bases, b"AATTCaaaGGATC");
        assert_eq!(f.ends.left, Overhang::five_prime(b"AATT"));
        assert_eq!(f.ends.right, Overhang::five_prime(b"GATC"));
        // The original's Build a Construct comment: source name, size label
        // (`<n>  nt`), then the two extremity labels with their positions.
        assert_eq!(
            f.comment,
            "Fragment extracted from pTest [13  nt] : (EcoRI[3] / BamHI[12])"
        );
        // The feature at 9..11 sits inside: fragment base 1 is sequence base 4.
        assert_eq!(f.features.len(), 1);
        assert_eq!((f.features[0].start, f.features[0].end), (6, 8));
        assert_eq!(&f.bases[5..8], b"aaa");
        // Counter-clockwise takes the other arc.
        let g = extract_between(&seq, &eco, &bam, false).unwrap();
        assert_eq!(g.bases, b"GATCCccttGAATT");
        assert!(g.features.is_empty());
        // One site: whole linearised plasmid.
        let w = extract_between(&seq, &eco, &eco, true).unwrap();
        assert_eq!(w.len(), 19 + 4);
        assert_eq!(w.name, "pTest (EcoRI)");
    }

    #[test]
    fn linear_antisense_is_antiparallel_with_swapped_ends() {
        let mut seq = plasmid();
        seq.topology = Topology::Linear;
        let eco = site(&seq, "EcoRI,g/aattc,");
        let bam = site(&seq, "BamHI,g/gatcc,");
        let f = extract_between(&seq, &eco, &bam, false).unwrap();
        assert_eq!(f.bases, reverse_complement(b"AATTCaaaGGATC"));
        assert_eq!(f.ends.left, Overhang::five_prime(b"GATC"));
        assert_eq!(f.ends.right, Overhang::five_prime(b"AATT"));
        assert_eq!(f.features[0].strand, Strand::Reverse);
        assert_eq!(f.features[0].lo_hi(), (6, 8, false));
    }
}
