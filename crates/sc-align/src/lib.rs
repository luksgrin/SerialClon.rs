//! Sequence alignment for SerialClon.rs (Align two Sequences, ⌘7).
//!
//! A local Smith–Waterman alignment with affine gaps (Gotoh), for DNA
//! (match / mismatch, degenerate codes match when their sets intersect) and
//! for protein (BLOSUM62). DNA is aligned against both strands of the second
//! sequence and the better alignment is kept, so an inverted segment such as
//! the pBluescript KS(+) / SK(+) polylinker shows up as a reverse-strand hit.
//! [`consensus`] extracts the aligned consensus (changelog 1.3) and
//! [`difference_columns`] drives the highlight-differences toggle (2.6).
//!
//! The plan named the `bio` crate; a self-contained implementation was
//! preferred: ~150 lines, no dependency, and full control of the traceback.

use sc_seq::alphabet::{from_mask, mask, reverse_complement};
use serde::{Deserialize, Serialize};

pub mod blosum;
pub mod multiple;

/// DNA or protein scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    /// Nucleotides: +5 / −4, gap open −10, gap extend −1 (EMBOSS `water` defaults, rounded).
    Dna,
    /// Amino acids: BLOSUM62, gap open −10, gap extend −1.
    Protein,
}

/// Why an alignment could not be computed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AlignError {
    /// One sequence is empty.
    #[error("both sequences must contain at least one residue")]
    Empty,
    /// The dynamic-programming matrix would be too large.
    #[error(
        "sequences too long to align locally ({0} × {1}); select regions of at most a few thousand residues"
    )]
    TooLong(usize, usize),
}

/// Largest matrix (cells) accepted: 60 M cells ≈ 180 MB of traceback.
pub const MAX_CELLS: usize = 60_000_000;

/// A local alignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alignment {
    /// Scoring used.
    pub kind: Kind,
    /// Smith–Waterman score.
    pub score: i32,
    /// 0-based start in the first sequence.
    pub a_start: usize,
    /// 0-based end (exclusive) in the first sequence.
    pub a_end: usize,
    /// 0-based start in the second sequence, forward coordinates.
    pub b_start: usize,
    /// 0-based end (exclusive) in the second sequence, forward coordinates.
    pub b_end: usize,
    /// True if the second sequence aligned as its reverse complement.
    pub b_reverse: bool,
    /// First sequence with `-` for gaps.
    pub a_line: String,
    /// Second sequence (as aligned, i.e. reverse-complemented when `b_reverse`) with `-` for gaps.
    pub b_line: String,
    /// `|` identical, `:` similar (protein), space otherwise.
    pub midline: String,
    /// Identical columns.
    pub matches: usize,
    /// Aligned columns, gaps included.
    pub length: usize,
    /// `matches / length` as a percentage.
    pub identity: f64,
}

fn score_pair(kind: Kind, a: u8, b: u8) -> i32 {
    match kind {
        Kind::Dna => {
            let (ma, mb) = (mask(a), mask(b));
            if ma == 0 || mb == 0 {
                -4
            } else if ma == mb && ma.count_ones() == 1 {
                5
            } else if ma & mb != 0 {
                1
            } else {
                -4
            }
        }
        Kind::Protein => blosum::score(a, b),
    }
}

const GAP_OPEN: i32 = 10;
const GAP_EXTEND: i32 = 1;

// Traceback states.
const STOP: u8 = 0;
const DIAG: u8 = 1;
const UP: u8 = 2; // gap in b (consume a)
const LEFT: u8 = 3; // gap in a (consume b)

/// Aligns `a` against `b` as given (no strand search).
pub fn local(a: &[u8], b: &[u8], kind: Kind) -> Result<Alignment, AlignError> {
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 {
        return Err(AlignError::Empty);
    }
    if n.saturating_mul(m) > MAX_CELLS {
        return Err(AlignError::TooLong(n, m));
    }
    let w = m + 1;
    // Direction of the best move into H, and of E / F when they were extended.
    let mut dir_h = vec![STOP; (n + 1) * w];
    let mut ext_e = vec![false; (n + 1) * w];
    let mut ext_f = vec![false; (n + 1) * w];
    let mut h_prev = vec![0i32; w];
    let mut h_cur = vec![0i32; w];
    let mut f = vec![i32::MIN / 2; w];
    let (mut best, mut bi, mut bj) = (0i32, 0usize, 0usize);
    for i in 1..=n {
        let mut e = i32::MIN / 2;
        h_cur[0] = 0;
        for j in 1..=m {
            let idx = i * w + j;
            // E: gap in a (moving left), F: gap in b (moving up).
            let e_open = h_cur[j - 1] - GAP_OPEN;
            let e_ext = e - GAP_EXTEND;
            e = e_open.max(e_ext);
            ext_e[idx] = e_ext > e_open;
            let f_open = h_prev[j] - GAP_OPEN;
            let f_ext = f[j] - GAP_EXTEND;
            f[j] = f_open.max(f_ext);
            ext_f[idx] = f_ext > f_open;
            let diag = h_prev[j - 1] + score_pair(kind, a[i - 1], b[j - 1]);
            let (mut h, mut d) = (0, STOP);
            if diag > h {
                h = diag;
                d = DIAG;
            }
            if f[j] > h {
                h = f[j];
                d = UP;
            }
            if e > h {
                h = e;
                d = LEFT;
            }
            h_cur[j] = h;
            dir_h[idx] = d;
            if h > best {
                best = h;
                bi = i;
                bj = j;
            }
        }
        std::mem::swap(&mut h_prev, &mut h_cur);
    }
    // Traceback.
    let (mut i, mut j) = (bi, bj);
    let mut a_line = Vec::new();
    let mut b_line = Vec::new();
    let mut state = dir_h[i * w + j];
    while i > 0 && j > 0 && state != STOP {
        match state {
            DIAG => {
                a_line.push(a[i - 1]);
                b_line.push(b[j - 1]);
                i -= 1;
                j -= 1;
                state = dir_h[i * w + j];
            }
            UP => {
                let extended = ext_f[i * w + j];
                a_line.push(a[i - 1]);
                b_line.push(b'-');
                i -= 1;
                state = if extended { UP } else { dir_h[i * w + j] };
            }
            LEFT => {
                let extended = ext_e[i * w + j];
                a_line.push(b'-');
                b_line.push(b[j - 1]);
                j -= 1;
                state = if extended { LEFT } else { dir_h[i * w + j] };
            }
            _ => break,
        }
    }
    a_line.reverse();
    b_line.reverse();
    let midline: String = a_line
        .iter()
        .zip(&b_line)
        .map(|(&x, &y)| {
            if x == b'-' || y == b'-' {
                ' '
            } else if x.eq_ignore_ascii_case(&y) || (kind == Kind::Dna && mask(x) & mask(y) != 0) {
                '|'
            } else if score_pair(kind, x, y) > 0 {
                ':'
            } else {
                ' '
            }
        })
        .collect();
    let matches = midline.chars().filter(|&c| c == '|').count();
    let length = a_line.len();
    Ok(Alignment {
        kind,
        score: best,
        a_start: i,
        a_end: bi,
        b_start: j,
        b_end: bj,
        b_reverse: false,
        a_line: String::from_utf8_lossy(&a_line).into_owned(),
        b_line: String::from_utf8_lossy(&b_line).into_owned(),
        midline,
        matches,
        length,
        identity: if length == 0 {
            0.0
        } else {
            100.0 * matches as f64 / length as f64
        },
    })
}

/// Aligns `a` against `b` and, for DNA, against the reverse complement of
/// `b`; returns the better alignment with `b` coordinates mapped back to the
/// forward strand.
pub fn align(a: &[u8], b: &[u8], kind: Kind) -> Result<Alignment, AlignError> {
    let forward = local(a, b, kind)?;
    if kind != Kind::Dna {
        return Ok(forward);
    }
    let rc = reverse_complement(b);
    let mut reverse = local(a, &rc, kind)?;
    if reverse.score > forward.score {
        let m = b.len();
        let (s, e) = (reverse.b_start, reverse.b_end);
        reverse.b_start = m - e;
        reverse.b_end = m - s;
        reverse.b_reverse = true;
        Ok(reverse)
    } else {
        Ok(forward)
    }
}

/// The consensus of the aligned region: identical columns as they are,
/// mismatches as the IUPAC union (DNA) or `X` (protein), gaps dropped.
pub fn consensus(aln: &Alignment) -> Vec<u8> {
    aln.a_line
        .bytes()
        .zip(aln.b_line.bytes())
        .filter(|(x, y)| *x != b'-' && *y != b'-')
        .map(|(x, y)| {
            if x.eq_ignore_ascii_case(&y) {
                x
            } else if aln.kind == Kind::Dna {
                from_mask(mask(x) | mask(y))
            } else {
                b'X'
            }
        })
        .collect()
}

/// Columns (0-based) that are not identical: mismatches and gaps.
pub fn difference_columns(aln: &Alignment) -> Vec<usize> {
    aln.midline
        .chars()
        .enumerate()
        .filter(|(_, c)| *c != '|')
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_and_gapped_dna() {
        let a = b"ACGTACGTACGT";
        let aln = local(a, a, Kind::Dna).unwrap();
        assert_eq!(aln.identity, 100.0);
        assert_eq!(
            (aln.a_start, aln.a_end, aln.b_start, aln.b_end),
            (0, 12, 0, 12)
        );
        assert_eq!(aln.score, 60);
        // A deletion in b opens one gap.
        let b = b"ACGTACACGT";
        let aln = local(b"ACGTACGTACGT", b, Kind::Dna).unwrap();
        assert!(
            aln.a_line.contains("GT") && aln.b_line.contains("--"),
            "{}\n{}",
            aln.a_line,
            aln.b_line
        );
        assert_eq!(aln.matches, 10);
        assert_eq!(difference_columns(&aln).len(), 2);
    }

    #[test]
    fn reverse_strand_is_found_and_mapped_back() {
        let a = b"ttttGGATCCGAATTCAAGCTTGCATGCtttt";
        let b = reverse_complement(b"ccccGGATCCGAATTCAAGCTTGCATGCcccc");
        let aln = align(a, &b, Kind::Dna).unwrap();
        assert!(aln.b_reverse);
        assert_eq!(aln.identity, 100.0);
        assert_eq!((aln.a_start, aln.a_end), (4, 28));
        // b forward coordinates of the segment: rc placed it at 4..28 as well.
        assert_eq!((aln.b_start, aln.b_end), (4, 28));
        assert_eq!(aln.b_line, "GGATCCGAATTCAAGCTTGCATGC");
    }

    #[test]
    fn protein_alignment_uses_blosum() {
        let aln = align(b"MKWVTFISLLFLFSSAYS", b"MKWVTFLSLLFLFSSAYS", Kind::Protein).unwrap();
        assert_eq!(aln.length, 18);
        assert_eq!(aln.matches, 17);
        assert_eq!(aln.midline.chars().nth(6), Some(':')); // I/L similar
        assert!(!aln.b_reverse);
    }

    #[test]
    fn consensus_and_errors() {
        let aln = local(b"ACGTAC", b"ACGAAC", Kind::Dna).unwrap();
        assert_eq!(consensus(&aln), b"ACGWAC");
        assert_eq!(local(b"", b"A", Kind::Dna), Err(AlignError::Empty));
        let long = vec![b'A'; 8000];
        assert!(matches!(
            local(&long, &long, Kind::Dna),
            Err(AlignError::TooLong(8000, 8000))
        ));
        // Degenerate codes match by intersection.
        let aln = local(b"ACGT", b"ACRT", Kind::Dna).unwrap();
        assert_eq!(aln.matches, 4);
    }
}
