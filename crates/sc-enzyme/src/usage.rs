//! Site usage: how often each enzyme cuts.

use sc_seq::Topology;

use crate::{Enzyme, Library, find_sites};

/// One row of the Site Usage table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteUsage<'a> {
    /// The enzyme.
    pub enzyme: &'a Enzyme,
    /// Number of cuts.
    pub count: usize,
}

/// Counts the sites of every enzyme in the library. Rows are in library order.
pub fn site_usage<'a>(
    bases: &[u8],
    topology: Topology,
    library: &'a Library,
) -> Vec<SiteUsage<'a>> {
    library
        .enzymes
        .iter()
        .map(|enzyme| SiteUsage {
            enzyme,
            count: find_sites(bases, topology, enzyme).len(),
        })
        .collect()
}

impl SiteUsage<'_> {
    /// Cuts exactly once (shown in bold in every enzyme list).
    pub fn is_unique(&self) -> bool {
        self.count == 1
    }

    /// Does not cut (shown in italic).
    pub fn is_absent(&self) -> bool {
        self.count == 0
    }
}
