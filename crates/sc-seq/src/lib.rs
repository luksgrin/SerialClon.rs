//! Sequence model for SerialClon.rs.
//!
//! This crate owns everything about a sequence that does not touch a file or a
//! window: the [`Sequence`] and [`Feature`] model, the IUPAC [`alphabet`],
//! [`translate`] (genetic code, ORFs) and [`stats`] (base counts, %GC, Tm,
//! protein MW). The file formats live in `sc-format`, the enzymes in
//! `sc-enzyme`.
//!
//! Conventions shared by every crate:
//! - Bases are ASCII bytes with **case preserved** — Serial Cloner can restrict
//!   translation to upper-case, so case is data.
//! - Positions shown to users are 1-based and inclusive; internal indices are
//!   0-based. Functions say which they take.
//! - Cut positions are the index of the base *after* the cut: a cut at `p` lies
//!   between `bases[p-1]` and `bases[p]`.

pub mod alphabet;
pub mod bisulfite;
pub mod codon;
pub mod genetic_codes;
pub mod pi;
pub mod sequence;
pub mod stats;
pub mod translate;

pub use sequence::{
    Ends, Feature, Overhang, Rgb, SeqType, Sequence, Source, Strand, Topology, XDNA_HEADER_LEN,
};
