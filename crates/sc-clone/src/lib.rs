//! Cloning operations for SerialClon.rs (User Manual §III.E).
//!
//! Every product carries a provenance note in its comment, exactly as the
//! original did ("the comments associated with it summarizes how the
//! fragment was obtained"), and inherits the features that lie inside it.
//!
//! - [`extract`]: Extract Fragment (M3).
//! - [`ligate`]: blunting and ligation of fragments with cohesive ends (Build
//!   a Construct).
//! - [`pcr`]: Run a PCR with un-annealed tails and TA overhangs.
//! - [`adaptor`]: Create an Adaptor — the oligos to order.
//! - [`shrna`]: Make shRNA from a stored layout.
//! - [`gateway`]: BP / LR reactions and yeast-style homologous recombination.
//! - [`cutter`]: Virtual Cutter — fragment sizes and gel migration.
//! - [`silent`]: Silent Map — sites introducible without changing the peptide.
//!
//! Fragments use the *extent* representation of `sc-enzyme`: `bases` is the
//! top-strand reading of everything either strand covers, and the
//! [`sc_seq::Overhang`]s say which strand is missing at each end.

pub mod adaptor;
pub mod cutter;
pub mod extract;
pub mod gateway;
pub mod ligate;
pub mod pcr;
pub mod shrna;
pub mod silent;

pub use extract::{SiteRef, extract_between, extract_selection};

/// First line of a comment, for provenance notes.
pub(crate) fn first_line(comment: &str) -> &str {
    comment.split(['\r', '\n']).next().unwrap_or("")
}

/// Appends the features of `src` that lie entirely inside `lo..hi` (0-based,
/// half-open, non-wrapping) to `out`, shifted so that `lo` maps to `offset`
/// (0-based), skipping exact duplicates.
pub(crate) fn inherit_into(
    out: &mut Vec<sc_seq::Feature>,
    src: &sc_seq::Sequence,
    lo: usize,
    hi: usize,
    offset: usize,
) {
    for f in &src.features {
        let (a, b, wraps) = f.lo_hi();
        if wraps {
            continue;
        }
        let (a, b) = (a as usize - 1, b as usize);
        if a < lo || b > hi {
            continue;
        }
        let shift = |p: u32| (p as usize - 1 - lo + offset) as u32 + 1;
        let g = sc_seq::Feature {
            start: shift(f.start),
            end: shift(f.end),
            ..f.clone()
        };
        if !out.iter().any(|o| {
            o.name == g.name && o.start == g.start && o.end == g.end && o.strand == g.strand
        }) {
            out.push(g);
        }
    }
}

/// A sequence reverse-complemented with its features flipped and its ends
/// swapped.
pub(crate) fn antiparallel(seq: &sc_seq::Sequence) -> sc_seq::Sequence {
    let n = seq.len() as u32;
    let mut out = seq.clone();
    out.bases = sc_seq::alphabet::reverse_complement(&seq.bases);
    for f in &mut out.features {
        let (s, e) = (n + 1 - f.start, n + 1 - f.end);
        f.start = s;
        f.end = e;
        f.strand = if f.strand == sc_seq::Strand::Forward {
            sc_seq::Strand::Reverse
        } else {
            sc_seq::Strand::Forward
        };
    }
    std::mem::swap(&mut out.ends.left, &mut out.ends.right);
    out
}
