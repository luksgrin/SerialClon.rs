//! Cloning operations for SerialClon.rs.
//!
//! Every product carries a provenance note in its comment, exactly as the
//! original did ("the comments associated with it summarizes how the
//! fragment was obtained"). [`extract`] arrives with M3; Build a Construct,
//! PCR, adaptors, shRNA, Gateway, Silent Map and the Virtual Cutter follow in
//! M5 (`PLAN.md` §4).

pub mod extract;

pub use extract::{SiteRef, extract_between, extract_selection};
