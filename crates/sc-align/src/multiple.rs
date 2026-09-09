//! Multiple sequence alignment (Generate ▸ Align Multiple Sequences, M6b)
//! on the pure-Rust port of MAFFT 7.526 (`mafft` crate, byte-identical to
//! the C reference). The original never had this window; its "Copy All
//! Sequences To Multifasta … for CLUSTAL" workflow points at it.
//!
//! The engine receives lower-case nucleotides / upper-case amino acids, as
//! `mafft` does, and index names so that rows can be mapped back to the
//! input order whatever `--adjustdirection` did to the names.

use mafft::adjust_direction::adjust_direction;
use mafft::{AlignmentMode, MafftEngine, SeqType as MafftType, Sequence as MafftSeq, SequenceSet};
use sc_seq::SeqType;
use sc_seq::alphabet::{from_mask, mask};
use serde::{Deserialize, Serialize};

use crate::Kind;

/// MAFFT strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// MAFFT's `--auto` rule: L-INS-i for ≤ 200 sequences of ≤ 2 kb, FFT-NS-2 otherwise.
    #[default]
    Auto,
    /// Fast progressive, two guide-tree rounds.
    FftNs2,
    /// Progressive plus limited iterative refinement.
    FftNsi,
    /// Local pairwise information + iterative refinement (most accurate for ≤ 200 sequences).
    LInsi,
    /// Global pairwise + iterative refinement.
    GInsi,
    /// Generalised affine gaps for sequences with long unalignable regions.
    EInsi,
}

impl Mode {
    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Auto => "Auto",
            Mode::FftNs2 => "FFT-NS-2",
            Mode::FftNsi => "FFT-NS-i",
            Mode::LInsi => "L-INS-i",
            Mode::GInsi => "G-INS-i",
            Mode::EInsi => "E-INS-i",
        }
    }

    /// Resolves `Auto` for a given input (MAFFT's `--auto`: L-INS-i when
    /// there are at most 200 sequences none longer than 2 000 residues).
    pub fn resolve(self, nseq: usize, max_len: usize) -> Mode {
        match self {
            Mode::Auto => {
                if nseq <= 200 && max_len <= 2_000 {
                    Mode::LInsi
                } else {
                    Mode::FftNs2
                }
            }
            m => m,
        }
    }

    fn engine_mode(self) -> AlignmentMode {
        match self {
            Mode::Auto | Mode::FftNs2 => AlignmentMode::FftNs2,
            Mode::FftNsi => AlignmentMode::FftNsi { iterations: 2 },
            Mode::LInsi => AlignmentMode::LInsi { iterations: 1000 },
            Mode::GInsi => AlignmentMode::GInsi { iterations: 1000 },
            Mode::EInsi => AlignmentMode::EInsi { iterations: 1000 },
        }
    }
}

/// Options of the Align Multiple Sequences window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Options {
    /// Strategy.
    pub mode: Mode,
    /// `--adjustdirection`: reverse-complement sequences that are on the other strand.
    pub adjust_direction: bool,
    /// Keep the input's letter case in the rows (else MAFFT's lower/upper convention).
    pub keep_case: bool,
}

/// One input sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    /// Name.
    pub name: String,
    /// Residues, case preserved, no gaps.
    pub bases: Vec<u8>,
    /// DNA / degenerate DNA / RNA / protein.
    pub seq_type: SeqType,
}

/// One aligned row, in the input order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    /// Name.
    pub name: String,
    /// Residues with `-` for gaps.
    pub aligned: String,
    /// True if `--adjustdirection` reverse-complemented this sequence.
    pub reversed: bool,
    /// Ungapped length.
    pub length: usize,
}

/// A multiple alignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Msa {
    /// DNA or protein scoring.
    pub kind: Kind,
    /// The strategy actually used (`Auto` resolved).
    pub mode: Mode,
    /// Rows, in the input order.
    pub rows: Vec<Row>,
    /// Clustal-style conservation marks per column: `*` identical, `:` similar, ` ` different.
    pub marks: String,
    /// Majority consensus (IUPAC for DNA ties, `X` for protein), gap-majority columns dropped.
    pub consensus: String,
    /// Number of columns.
    pub length: usize,
    /// MAFFT's alignment score.
    pub score: f64,
}

/// Why an alignment could not be computed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MsaError {
    /// Fewer than two sequences.
    #[error("select at least two sequences")]
    TooFew,
    /// DNA and protein together.
    #[error("cannot align DNA and protein together: {0} is {1}")]
    Mixed(String, String),
    /// An empty sequence.
    #[error("{0} is empty")]
    Empty(String),
    /// More residues than the cap.
    #[error("too much sequence to align ({0} residues); the limit is {1}")]
    TooLong(usize, usize),
    /// The engine returned something unexpected.
    #[error("the alignment engine returned {0} rows for {1} sequences")]
    Engine(usize, usize),
}

/// Largest total input accepted, in residues.
pub const MAX_TOTAL: usize = 2_000_000;

/// The Clustal "strong" groups of amino acids.
const STRONG_GROUPS: [&[u8]; 9] = [
    b"STA", b"NEQK", b"NHQK", b"NDEQ", b"QHRK", b"MILV", b"MILF", b"HY", b"FYW",
];

fn mark(kind: Kind, column: &[u8]) -> u8 {
    if column.is_empty() || column.contains(&b'-') {
        return b' ';
    }
    let first = column[0].to_ascii_uppercase();
    if column.iter().all(|&c| c.to_ascii_uppercase() == first) {
        return b'*';
    }
    match kind {
        Kind::Dna => {
            let m = column.iter().fold(0xFu8, |acc, &c| acc & mask(c));
            if m != 0 { b':' } else { b' ' }
        }
        Kind::Protein => {
            let strong = STRONG_GROUPS
                .iter()
                .any(|g| column.iter().all(|&c| g.contains(&c.to_ascii_uppercase())));
            if strong { b':' } else { b' ' }
        }
    }
}

fn consensus_residue(kind: Kind, column: &[u8]) -> Option<u8> {
    let residues: Vec<u8> = column
        .iter()
        .filter(|&&c| c != b'-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if residues.len() * 2 < column.len() {
        return None; // gap majority
    }
    let mut counts: Vec<(u8, usize)> = Vec::new();
    for &r in &residues {
        match counts.iter_mut().find(|(c, _)| *c == r) {
            Some((_, n)) => *n += 1,
            None => counts.push((r, 1)),
        }
    }
    let best = counts.iter().map(|(_, n)| *n).max()?;
    let tied: Vec<u8> = counts
        .iter()
        .filter(|(_, n)| *n == best)
        .map(|(c, _)| *c)
        .collect();
    if tied.len() == 1 {
        return Some(tied[0]);
    }
    Some(match kind {
        Kind::Dna => from_mask(tied.iter().fold(0u8, |acc, &c| acc | mask(c))),
        Kind::Protein => b'X',
    })
}

/// Aligns `inputs` with `opts`. Rows come back in the input order.
pub fn align_multiple(inputs: &[Input], opts: &Options) -> Result<Msa, MsaError> {
    if inputs.len() < 2 {
        return Err(MsaError::TooFew);
    }
    let protein = inputs[0].seq_type == SeqType::Protein;
    for i in inputs {
        if (i.seq_type == SeqType::Protein) != protein {
            return Err(MsaError::Mixed(
                i.name.clone(),
                i.seq_type.label().to_owned(),
            ));
        }
        if !i.bases.iter().any(u8::is_ascii_alphabetic) {
            return Err(MsaError::Empty(i.name.clone()));
        }
    }
    let total: usize = inputs.iter().map(|i| i.bases.len()).sum();
    if total > MAX_TOTAL {
        return Err(MsaError::TooLong(total, MAX_TOTAL));
    }
    let kind = if protein { Kind::Protein } else { Kind::Dna };
    let seq_type = if protein {
        MafftType::Protein
    } else if inputs[0].seq_type == SeqType::Rna {
        MafftType::Rna
    } else {
        MafftType::Dna
    };
    // Engine input: MAFFT's case convention, index names.
    let cleaned: Vec<Vec<u8>> = inputs
        .iter()
        .map(|i| {
            i.bases
                .iter()
                .filter(|b| b.is_ascii_alphabetic())
                .map(|&b| {
                    if protein {
                        b.to_ascii_uppercase()
                    } else {
                        b.to_ascii_lowercase()
                    }
                })
                .collect()
        })
        .collect();
    let mut set = SequenceSet::new(seq_type);
    for (i, data) in cleaned.iter().enumerate() {
        set.sequences.push(MafftSeq {
            name: i.to_string(),
            data: data.clone(),
        });
    }
    if opts.adjust_direction && !protein {
        set = adjust_direction(&set);
    }
    let max_len = cleaned.iter().map(Vec::len).max().unwrap_or(0);
    let mode = opts.mode.resolve(inputs.len(), max_len);
    let engine = MafftEngine::new(mode.engine_mode());
    let msa = engine.align(&set);
    if msa.sequences.len() != inputs.len() || msa.names.len() != inputs.len() {
        return Err(MsaError::Engine(msa.sequences.len(), inputs.len()));
    }
    // Map rows back to the input order by their index name.
    let mut rows: Vec<Option<Row>> = vec![None; inputs.len()];
    for (name, aligned) in msa.names.iter().zip(&msa.sequences) {
        let reversed = name.starts_with("_R_");
        let index: usize = name
            .trim_start_matches("_R_")
            .parse()
            .map_err(|_| MsaError::Engine(0, inputs.len()))?;
        let input = inputs
            .get(index)
            .ok_or(MsaError::Engine(index, inputs.len()))?;
        let mut text: Vec<u8> = aligned.clone();
        if opts.keep_case && !reversed {
            // Re-apply the input's case residue by residue.
            let mut src = input.bases.iter().filter(|b| b.is_ascii_alphabetic());
            for c in text.iter_mut().filter(|c| **c != b'-') {
                if let Some(&orig) = src.next() {
                    *c = orig;
                }
            }
        }
        rows[index] = Some(Row {
            name: input.name.clone(),
            aligned: String::from_utf8_lossy(&text).into_owned(),
            reversed,
            length: cleaned[index].len(),
        });
    }
    let rows: Vec<Row> = rows
        .into_iter()
        .map(|r| r.ok_or(MsaError::Engine(0, inputs.len())))
        .collect::<Result<_, _>>()?;
    let length = rows.iter().map(|r| r.aligned.len()).max().unwrap_or(0);
    let mut marks = Vec::with_capacity(length);
    let mut consensus = Vec::new();
    let mut column = Vec::with_capacity(rows.len());
    for col in 0..length {
        column.clear();
        column.extend(
            rows.iter()
                .map(|r| r.aligned.as_bytes().get(col).copied().unwrap_or(b'-')),
        );
        marks.push(mark(kind, &column));
        if let Some(c) = consensus_residue(kind, &column) {
            consensus.push(c);
        }
    }
    Ok(Msa {
        kind,
        mode,
        rows,
        marks: String::from_utf8_lossy(&marks).into_owned(),
        consensus: String::from_utf8_lossy(&consensus).into_owned(),
        length,
        score: msa.score,
    })
}

/// The alignment as FASTA (60-column lines).
pub fn to_fasta(msa: &Msa) -> String {
    let mut out = String::new();
    for r in &msa.rows {
        out.push('>');
        out.push_str(&r.name);
        out.push('\n');
        for chunk in r.aligned.as_bytes().chunks(60) {
            out.push_str(&String::from_utf8_lossy(chunk));
            out.push('\n');
        }
    }
    out
}

/// The alignment in Clustal format (60-column blocks with conservation marks).
pub fn to_clustal(msa: &Msa) -> String {
    let width = msa
        .rows
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .clamp(10, 30);
    let mut out = String::from("CLUSTAL W multiple sequence alignment (SerialClon, MAFFT)\n\n");
    let mut col = 0;
    while col < msa.length {
        let end = (col + 60).min(msa.length);
        for r in &msa.rows {
            let name: String = r.name.chars().take(width).collect();
            let name = name.replace(' ', "_");
            let slice = r.aligned.get(col..end).unwrap_or("");
            out.push_str(&format!("{name:<width$} {slice}\n"));
        }
        out.push_str(&format!(
            "{:<width$} {}\n\n",
            "",
            msa.marks.get(col..end).unwrap_or("")
        ));
        col = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_seq::alphabet::reverse_complement;

    fn dna(name: &str, s: &str) -> Input {
        Input {
            name: name.into(),
            bases: s.as_bytes().to_vec(),
            seq_type: SeqType::Dna,
        }
    }
    fn pep(name: &str, s: &str) -> Input {
        Input {
            name: name.into(),
            bases: s.as_bytes().to_vec(),
            seq_type: SeqType::Protein,
        }
    }

    #[test]
    fn three_related_dna_sequences_gap_where_expected() {
        let core = "ATGGCTAGCTTGGACCGTAGCTAGCTTAGGCCAATTGGCCTAGGCTTAGCCATGGCTAAGCTTGGCC";
        let a = core.to_owned();
        let b = format!("{}{}", &core[..30], &core[36..]); // 6-base deletion
        let c = format!("{}TTTT{}", &core[..50], &core[50..]); // 4-base insertion
        let msa = align_multiple(
            &[dna("a", &a), dna("b", &b), dna("c", &c)],
            &Options::default(),
        )
        .unwrap();
        assert_eq!(msa.mode, Mode::LInsi);
        assert_eq!(msa.rows.len(), 3);
        assert_eq!(msa.rows[0].name, "a");
        assert_eq!(
            msa.rows[1].aligned.matches('-').count(),
            10,
            "{}",
            msa.rows[1].aligned
        );
        assert_eq!(
            msa.rows[0].aligned.matches('-').count(),
            4,
            "{}",
            msa.rows[0].aligned
        );
        assert_eq!(msa.rows[0].aligned.replace('-', "").to_uppercase(), a);
        assert_eq!(msa.marks.len(), msa.length);
        assert!(msa.marks.matches('*').count() >= 60, "{}", msa.marks);
        assert!(msa.consensus.len() >= a.len());
    }

    #[test]
    fn adjust_direction_flips_a_reversed_copy() {
        let s = "ATGGCTAGCTTGGACCGTAGCTAGCTTAGGCCAATTGGCCTAGGCTTAGCCATGGCTAAGCTTGGCCATCGATCGGATCCGAATTC";
        let rc = String::from_utf8(reverse_complement(s.as_bytes())).unwrap();
        let inputs = [dna("fwd", s), dna("rev", &rc), dna("fwd2", &s[5..])];
        let msa = align_multiple(
            &inputs,
            &Options {
                adjust_direction: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(msa.rows[1].reversed);
        assert!(!msa.rows[0].reversed);
        assert_eq!(msa.rows[1].aligned.replace('-', "").to_uppercase(), s);
        let plain = align_multiple(&inputs, &Options::default()).unwrap();
        assert!(!plain.rows[1].reversed);
    }

    #[test]
    fn protein_mode_marks_similar_columns() {
        let inputs = [
            pep("p1", "MKWVTFISLLFLFSSAYSRGVFRR"),
            pep("p2", "MKWVTFLSLLFLFSSAYSRGVFRR"),
            pep("p3", "MKWVTFISLLFLFSSAYSKGVFRR"),
        ];
        let msa = align_multiple(
            &inputs,
            &Options {
                mode: Mode::GInsi,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(msa.kind, Kind::Protein);
        assert_eq!(msa.mode, Mode::GInsi);
        assert_eq!(msa.length, 24);
        assert_eq!(msa.marks.as_bytes()[6], b':'); // I/L
        assert_eq!(msa.marks.as_bytes()[18], b':'); // R/K
        assert_eq!(msa.marks.matches('*').count(), 22);
        assert_eq!(msa.consensus, "MKWVTFISLLFLFSSAYSRGVFRR");
    }

    #[test]
    fn consensus_ties_and_writers() {
        let msa = align_multiple(
            &[
                dna("a", "ACGTACGTAC"),
                dna("b", "ACGAACGTAC"),
                dna("c", "ACGTACGTAC"),
                dna("d", "ACGAACGTAC"),
            ],
            &Options {
                mode: Mode::FftNs2,
                keep_case: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(msa.consensus, "ACGWACGTAC");
        assert_eq!(msa.rows[0].aligned, "ACGTACGTAC"); // keep_case
        let fasta = to_fasta(&msa);
        assert!(fasta.starts_with(">a\nACGTACGTAC\n>b\n"));
        let clustal = to_clustal(&msa);
        assert!(clustal.starts_with("CLUSTAL W"));
        assert!(clustal.contains("a          ACGTACGTAC\n"));
        assert!(clustal.contains("           *** ******\n"));
    }

    #[test]
    fn errors() {
        assert_eq!(
            align_multiple(&[dna("a", "ACGT")], &Options::default()),
            Err(MsaError::TooFew)
        );
        assert!(matches!(
            align_multiple(&[dna("a", "ACGT"), pep("p", "MKV")], &Options::default()),
            Err(MsaError::Mixed(..))
        ));
        assert_eq!(
            align_multiple(&[dna("a", "ACGT"), dna("b", "")], &Options::default()),
            Err(MsaError::Empty("b".into()))
        );
        let big = "A".repeat(MAX_TOTAL / 2 + 1);
        assert!(matches!(
            align_multiple(&[dna("a", &big), dna("b", &big)], &Options::default()),
            Err(MsaError::TooLong(..))
        ));
        assert_eq!(Mode::Auto.resolve(3, 5000), Mode::FftNs2);
        assert_eq!(Mode::Auto.resolve(200, 2000), Mode::LInsi);
    }
}
