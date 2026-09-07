//! Sequence file formats for SerialClon.rs.
//!
//! Filled in at milestone **M1** (see `PLAN.md` §4). Every format sits behind one `SeqFile` trait: `.xdna` read + write (byte layout in `PLAN.md` §1.4), FASTA, GenBank/EMBL with Serial Cloner's colour qualifiers, ApE, Vector NTI `.ma4`, MacVector `.nucl`, DNAStar `.seq`, pDRAW32 `.pdw` and raw text. Round-trip fixtures live in `reference/sequences/`.
