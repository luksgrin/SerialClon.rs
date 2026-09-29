//! Run a PCR (User Manual §III.E, changelog 1.1 / 1.3 / 2.6).

use sc_seq::alphabet::{matches_at, reverse_complement};
use sc_seq::{Ends, Feature, Overhang, Rgb, Sequence, Source, Strand, Topology};

use crate::{first_line, inherit_into};

/// Shortest 3' portion of a primer that must anneal.
pub const MIN_CORE: usize = 10;

/// Why the reaction failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PcrError {
    /// A primer's 3' end does not anneal anywhere.
    #[error(
        "the {0} primer does not anneal to the matrix (at least {MIN_CORE} bases of its 3' end must match)"
    )]
    NoAnnealing(&'static str),
    /// A primer anneals at more than one place.
    #[error("the {0} primer anneals at {1} places")]
    Ambiguous(&'static str, usize),
    /// The primer anneals, but to the other strand: it was entered in the
    /// wrong box. Positions are 1-based on the top strand.
    #[error(
        "the {which} primer anneals to the {strand} strand at {start}–{end}: it is a {other} primer"
    )]
    WrongStrand {
        /// `forward` or `reverse`.
        which: &'static str,
        /// The strand it does anneal to.
        strand: &'static str,
        /// 1-based start on the top strand.
        start: usize,
        /// 1-based end on the top strand.
        end: usize,
        /// The box it belongs in.
        other: &'static str,
    },
    /// The reverse primer anneals before the forward one on a linear matrix.
    #[error("the primers face away from each other: no product on a linear matrix")]
    Reversed,
}

/// Where a primer anneals: annealing region on the top strand (0-based,
/// half-open) and the un-annealed 5' tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annealing {
    /// Start on the top strand.
    pub start: usize,
    /// End on the top strand.
    pub end: usize,
    /// The tail (5'→3' on the primer).
    pub tail: Vec<u8>,
    /// The annealed core (5'→3' on the primer).
    pub core: Vec<u8>,
}

fn clean(primer: &str) -> Vec<u8> {
    primer
        .bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect()
}

fn occurrences(hay: &[u8], needle: &[u8], n: usize, circular: bool) -> Vec<usize> {
    let len = needle.len();
    if len == 0 || len > hay.len() {
        return Vec::new();
    }
    let last = if circular { n } else { hay.len() - len + 1 };
    (0..last)
        .filter(|&i| i + len <= hay.len() && matches_at(needle, &hay[i..i + len]))
        .collect()
}

/// Finds where a primer anneals: the longest 3' portion (down to
/// [`MIN_CORE`]) found exactly once on the given strand.
pub fn anneal(
    matrix: &[u8],
    circular: bool,
    primer: &[u8],
    reverse: bool,
    which: &'static str,
) -> Result<Annealing, PcrError> {
    let n = matrix.len();
    let mut hay = matrix.to_vec();
    if circular {
        hay.extend_from_slice(matrix);
    }
    let mut ambiguous = 0;
    for k in (MIN_CORE.min(primer.len())..=primer.len()).rev() {
        let core = &primer[primer.len() - k..];
        let probe = if reverse {
            reverse_complement(core)
        } else {
            core.to_vec()
        };
        let hits = occurrences(&hay, &probe, n, circular);
        match hits.len() {
            0 => continue,
            1 => {
                return Ok(Annealing {
                    start: hits[0],
                    end: hits[0] + k,
                    tail: primer[..primer.len() - k].to_vec(),
                    core: core.to_vec(),
                });
            }
            m => {
                ambiguous = m;
                break;
            }
        }
    }
    if ambiguous > 1 {
        return Err(PcrError::Ambiguous(which, ambiguous));
    }
    // Not on this strand: is it on the other one? Then the user put it in
    // the wrong box, which is worth saying.
    if let Ok(a) = anneal_strand(matrix, circular, primer, !reverse) {
        return Err(PcrError::WrongStrand {
            which,
            strand: if reverse { "top" } else { "bottom" },
            start: a.start + 1,
            end: a.end,
            other: if reverse { "forward" } else { "reverse" },
        });
    }
    Err(PcrError::NoAnnealing(which))
}

/// Where a primer anneals on whichever strand it matches. Returns the
/// annealing and `true` when it is the strand the box did *not* expect
/// (a forward primer on the bottom strand, or a reverse one on the top):
/// the region of interest runs on the file's reverse strand, which is fine
/// as long as the other primer does the same.
pub fn anneal_either(
    matrix: &[u8],
    circular: bool,
    primer: &[u8],
    reverse: bool,
    which: &'static str,
) -> Result<(Annealing, bool), PcrError> {
    match anneal(matrix, circular, primer, reverse, which) {
        Ok(a) => Ok((a, false)),
        Err(PcrError::WrongStrand { .. }) => {
            anneal_strand(matrix, circular, primer, !reverse).map(|a| (a, true))
        }
        Err(e) => Err(e),
    }
}

/// [`anneal`] on one strand only, without the other-strand diagnosis.
fn anneal_strand(
    matrix: &[u8],
    circular: bool,
    primer: &[u8],
    reverse: bool,
) -> Result<Annealing, PcrError> {
    let n = matrix.len();
    let mut hay = matrix.to_vec();
    if circular {
        hay.extend_from_slice(matrix);
    }
    for k in (MIN_CORE.min(primer.len())..=primer.len()).rev() {
        let core = &primer[primer.len() - k..];
        let probe = if reverse {
            reverse_complement(core)
        } else {
            core.to_vec()
        };
        let hits = occurrences(&hay, &probe, n, circular);
        match hits.len() {
            0 => continue,
            1 => {
                return Ok(Annealing {
                    start: hits[0],
                    end: hits[0] + k,
                    tail: primer[..primer.len() - k].to_vec(),
                    core: core.to_vec(),
                });
            }
            m => return Err(PcrError::Ambiguous("", m)),
        }
    }
    Err(PcrError::NoAnnealing(""))
}

/// Runs the reaction. `ta` adds the 3' A overhangs of a Taq product.
pub fn pcr(
    matrix: &Sequence,
    forward: &str,
    reverse: &str,
    ta: bool,
) -> Result<Sequence, PcrError> {
    let circular = matrix.topology.is_circular();
    let fwd = clean(forward);
    let rev = clean(reverse);
    match (
        anneal(&matrix.bases, circular, &fwd, false, "forward"),
        anneal(&matrix.bases, circular, &rev, true, "reverse"),
    ) {
        (Ok(_), Ok(_)) => pcr_oriented(matrix, forward, reverse, ta),
        // The pair is inverted relative to the file's top strand: the forward
        // primer sits on the bottom strand and the reverse on the top. That is
        // a valid pair; amplify with the roles swapped and give the product
        // from the user's forward primer.
        (Err(PcrError::WrongStrand { .. }), Err(PcrError::WrongStrand { .. })) => {
            let mut product = pcr_oriented(matrix, reverse, forward, ta)?;
            antiparallel(&mut product);
            for f in &mut product.features {
                if let Some(rest) = f.name.strip_prefix("Fwd primer") {
                    f.name = format!("Rev primer{rest}");
                } else if let Some(rest) = f.name.strip_prefix("Rev primer") {
                    f.name = format!("Fwd primer{rest}");
                }
            }
            product.comment.push_str(
                "\rThe amplified region runs on the reverse strand of the matrix; the product is given 5'->3' from the forward primer.",
            );
            Ok(product)
        }
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}

/// Reverse-complements a product in place: bases, ends and features.
fn antiparallel(seq: &mut Sequence) {
    seq.bases = reverse_complement(&seq.bases);
    std::mem::swap(&mut seq.ends.left, &mut seq.ends.right);
    let len = seq.len() as u32;
    for f in &mut seq.features {
        let (s, e) = (len + 1 - f.start, len + 1 - f.end);
        f.start = s;
        f.end = e;
        f.strand = if f.strand == Strand::Forward {
            Strand::Reverse
        } else {
            Strand::Forward
        };
    }
}

/// The reaction with the forward primer on the top strand.
fn pcr_oriented(
    matrix: &Sequence,
    forward: &str,
    reverse: &str,
    ta: bool,
) -> Result<Sequence, PcrError> {
    let n = matrix.len();
    let circular = matrix.topology.is_circular();
    let fwd = clean(forward);
    let rev = clean(reverse);
    let f = anneal(&matrix.bases, circular, &fwd, false, "forward")?;
    let r = anneal(&matrix.bases, circular, &rev, true, "reverse")?;
    let f_start = f.start;
    let r_end = r.end;
    // Amplified region on the top strand, f_start..r_end, wrapping if needed.
    let body: Vec<u8> = if f_start < r_end && r_end <= n {
        matrix.bases[f_start..r_end].to_vec()
    } else if circular {
        let r_end = r_end % n;
        if f_start == r_end {
            return Err(PcrError::Reversed);
        }
        if f_start < r_end {
            matrix.bases[f_start..r_end].to_vec()
        } else {
            let mut v = matrix.bases[f_start..].to_vec();
            v.extend_from_slice(&matrix.bases[..r_end]);
            v
        }
    } else {
        return Err(PcrError::Reversed);
    };
    let mut bases = f.tail.clone();
    bases.extend_from_slice(&body);
    bases.extend_from_slice(&reverse_complement(&r.tail));
    let mut features = Vec::new();
    let offset = f.tail.len();
    if f_start < r_end && r_end <= n {
        inherit_into(&mut features, matrix, f_start, r_end, offset);
    }
    let primer_color = Rgb(0, 139, 139);
    let mk = |name: String, start: u32, end: u32, strand: Strand| Feature {
        name,
        description: "primer".into(),
        kind: "primer_bind".into(),
        start,
        end,
        strand,
        show: true,
        protect: false,
        arrow: true,
        color: primer_color,
    };
    let total = bases.len() as u32;
    if !f.tail.is_empty() {
        features.push(mk(
            "Fwd primer Anchor".into(),
            1,
            f.tail.len() as u32,
            Strand::Forward,
        ));
    }
    features.push(mk(
        "Fwd primer Core".into(),
        f.tail.len() as u32 + 1,
        fwd.len() as u32,
        Strand::Forward,
    ));
    let rev_core_len = r.core.len() as u32;
    let rev_tail_len = r.tail.len() as u32;
    features.push(mk(
        "Rev primer Core".into(),
        total - rev_tail_len,
        total - rev_tail_len - rev_core_len + 1,
        Strand::Reverse,
    ));
    if rev_tail_len > 0 {
        features.push(mk(
            "Rev primer Anchor".into(),
            total,
            total - rev_tail_len + 1,
            Strand::Reverse,
        ));
    }
    let mut product = Sequence {
        name: format!("PCR on {}", matrix.name),
        bases,
        seq_type: matrix.seq_type,
        topology: Topology::Linear,
        comment: format!(
            "PCR product from {} [{} bp] between {} and {}\rForward primer: {} ({} nt tail)\rReverse primer: {} ({} nt tail){}\r{}",
            matrix.name,
            n,
            f_start + 1,
            if r_end > n { r_end - n } else { r_end },
            String::from_utf8_lossy(&fwd),
            f.tail.len(),
            String::from_utf8_lossy(&rev),
            r.tail.len(),
            if ta {
                "\r3' A overhangs (TA cloning)"
            } else {
                ""
            },
            first_line(&matrix.comment)
        ),
        features,
        ends: Ends::default(),
        source: Source::New,
        strider_header: None,
    };
    if ta {
        // A 3' A on each strand: on the top strand's coordinates that is a
        // leading T (the bottom strand's A) and a trailing A.
        product.bases.insert(0, b'T');
        product.bases.push(b'A');
        for f in &mut product.features {
            f.start += 1;
            f.end += 1;
        }
        product.ends = Ends {
            left: Overhang::three_prime(b"A"),
            right: Overhang::three_prime(b"A"),
        };
    }
    Ok(product)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATRIX: &str = "ttttttGGATCCACTAGTAACGGCCGCCAGTGTGCTGGAATTCGCCCTTaaaaaaCCGCTCGAGGTCGACGGTATCGATAAGCTTGATATCGAATTCcccccc";

    #[test]
    fn plain_product_and_tails() {
        let m = Sequence::new("m", MATRIX.as_bytes().to_vec());
        let p = pcr(&m, "GGATCCACTAGTAACG", "GAATTCGATATCAAGC", false).unwrap();
        let start = MATRIX.find("GGATCCACTAGTAACG").unwrap();
        let end = MATRIX.find("GCTTGATATCGAATTC").unwrap() + 16;
        assert_eq!(String::from_utf8_lossy(&p.bases), MATRIX[start..end]);
        assert!(
            p.features
                .iter()
                .any(|f| f.name == "Fwd primer Core" && f.start == 1 && f.end == 16)
        );
        assert!(
            p.features
                .iter()
                .any(|f| f.name == "Rev primer Core" && f.strand == Strand::Reverse)
        );
        let p = pcr(&m, "cccGGATCCACTAGTAACG", "tttGAATTCGATATCAAGC", false).unwrap();
        assert!(p.bases.starts_with(b"CCC"));
        assert!(p.bases.ends_with(b"AAA"));
        assert_eq!(p.bases.len(), end - start + 6);
        assert!(
            p.features
                .iter()
                .any(|f| f.name == "Fwd primer Anchor" && f.end == 3)
        );
        assert!(p.features.iter().any(|f| f.name == "Rev primer Anchor"));
    }

    #[test]
    fn ta_overhangs_and_errors() {
        let m = Sequence::new("m", MATRIX.as_bytes().to_vec());
        let p = pcr(&m, "GGATCCACTAGTAACG", "GAATTCGATATCAAGC", true).unwrap();
        assert_eq!(p.ends.left, Overhang::three_prime(b"A"));
        assert_eq!(p.ends.right, Overhang::three_prime(b"A"));
        assert!(p.bases.starts_with(b"TGGATCC"));
        assert!(p.bases.ends_with(b"GAATTCA"));
        assert_eq!(
            pcr(&m, "GGGGGGGGGGGGGG", "GAATTCGATATCAAGC", false),
            Err(PcrError::NoAnnealing("forward"))
        );
        assert_eq!(
            pcr(&m, "GCTTGATATCGAATTC", "CGTTACTAGTGGATCC", false),
            Err(PcrError::Reversed)
        );
        let rep = Sequence::new(
            "r",
            b"AAAGGATCCACTAGTAtttGGATCCACTAGTAtttCCCCCCCCCCCC".to_vec(),
        );
        assert!(matches!(
            pcr(&rep, "GGATCCACTAGTA", "GGGGGGGGGGGG", false),
            Err(PcrError::Ambiguous("forward", 2))
        ));
    }

    #[test]
    fn an_inverted_pair_amplifies_and_reads_from_the_forward_primer() {
        let m = Sequence::new("m", MATRIX.as_bytes().to_vec());
        let top_f = &MATRIX[6..24]; // GGATCCACTAGTAACGGC
        let top_r = String::from_utf8(reverse_complement(&MATRIX.as_bytes()[80..100]))
            .unwrap()
            .to_ascii_uppercase();
        let straight = pcr(&m, top_f, &top_r, false).unwrap();
        // Same pair named the other way round: "forward" on the bottom strand.
        let inverted = pcr(&m, &top_r, top_f, false).unwrap();
        assert_eq!(inverted.bases, reverse_complement(&straight.bases));
        assert!(
            inverted
                .bases
                .to_ascii_uppercase()
                .starts_with(top_r.as_bytes())
        );
        assert!(inverted.comment.contains("reverse strand of the matrix"));
        let fwd_core = inverted
            .features
            .iter()
            .find(|f| f.name == "Fwd primer Core")
            .unwrap();
        assert_eq!((fwd_core.start, fwd_core.strand), (1, Strand::Forward));
        // Both primers on the same strand is still an error.
        assert!(matches!(
            pcr(&m, top_f, top_f, false),
            Err(PcrError::WrongStrand { .. })
        ));
    }

    #[test]
    fn a_primer_in_the_wrong_box_is_diagnosed_not_just_refused() {
        let m = MATRIX.as_bytes();
        let top: Vec<u8> = m[6..28].to_ascii_uppercase();
        let bottom = reverse_complement(&top);
        assert!(anneal(m, false, &top, false, "forward").is_ok());
        assert!(anneal(m, false, &bottom, true, "reverse").is_ok());
        let e = anneal(m, false, &bottom, false, "forward").unwrap_err();
        assert_eq!(
            e,
            PcrError::WrongStrand {
                which: "forward",
                strand: "bottom",
                start: 7,
                end: 28,
                other: "reverse"
            }
        );
        assert_eq!(
            e.to_string(),
            "the forward primer anneals to the bottom strand at 7–28: it is a reverse primer"
        );
        let e = anneal(m, false, &top, true, "reverse").unwrap_err();
        assert!(matches!(
            e,
            PcrError::WrongStrand {
                strand: "top",
                other: "forward",
                ..
            }
        ));
        // Nowhere at all is still "does not anneal".
        assert_eq!(
            anneal(m, false, b"TTTTTTTTTTTTTTTTTTTT", false, "forward").unwrap_err(),
            PcrError::NoAnnealing("forward")
        );
    }

    #[test]
    fn circular_matrix_wraps() {
        let mut m = Sequence::new("m", MATRIX.as_bytes().to_vec());
        m.topology = Topology::Circular;
        let p = pcr(&m, "GCTTGATATCGAATTC", "CGTTACTAGTGGATCC", false).unwrap();
        assert!(p.bases.starts_with(b"GCTTGATATCGAATTC"));
        assert!(p.bases.ends_with(b"GGATCCACTAGTAACG"));
    }
}
