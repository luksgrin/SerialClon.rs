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
        Err(PcrError::Ambiguous(which, ambiguous))
    } else {
        Err(PcrError::NoAnnealing(which))
    }
}

/// Runs the reaction. `ta` adds the 3' A overhangs of a Taq product.
pub fn pcr(
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
    fn circular_matrix_wraps() {
        let mut m = Sequence::new("m", MATRIX.as_bytes().to_vec());
        m.topology = Topology::Circular;
        let p = pcr(&m, "GCTTGATATCGAATTC", "CGTTACTAGTGGATCC", false).unwrap();
        assert!(p.bases.starts_with(b"GCTTGATATCGAATTC"));
        assert!(p.bases.ends_with(b"GGATCCACTAGTAACG"));
    }
}
