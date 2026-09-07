//! Virtual Cutter (changelog 1.3): fragment sizes after a digest and a
//! simulated gel.

use sc_enzyme::{Enzyme, Site, digest, find_sites};
use sc_seq::Sequence;

/// Sizes of the fragments produced by cutting with every given enzyme,
/// largest first.
pub fn fragment_sizes(seq: &Sequence, enzymes: &[&Enzyme]) -> Vec<usize> {
    let mut sites: Vec<Site> = enzymes
        .iter()
        .flat_map(|e| find_sites(&seq.bases, seq.topology, e))
        .collect();
    sites.sort_by_key(|s| s.cut_top);
    let mut sizes: Vec<usize> = digest(&seq.bases, seq.topology, &sites)
        .iter()
        .map(|f| f.len())
        .collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    sizes
}

/// A 1 kb DNA ladder: sizes in bp.
pub const LADDER_1KB: &[usize] = &[
    10000, 8000, 6000, 5000, 4000, 3000, 2500, 2000, 1500, 1000, 750, 500, 250,
];

/// Migration of a fragment on a 1 % agarose gel as a fraction of the lane
/// (0 = well, 1 = front), a log-linear model calibrated on the ladder.
pub fn migration(size: usize) -> f64 {
    let s = (size.max(50) as f64).log10();
    let (lo, hi) = (250f64.log10(), 10000f64.log10());
    ((hi - s) / (hi - lo)).clamp(0.0, 1.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_enzyme::library::parse_entry;
    use sc_seq::Topology;

    #[test]
    fn sizes_and_migration() {
        let eco = parse_entry("EcoRI,g/aattc,").unwrap();
        let bam = parse_entry("BamHI,g/gatcc,").unwrap();
        let mut seq = Sequence::new("p", b"ttGAATTCaaaaaaaaaaGGATCCcccc".to_vec());
        seq.topology = Topology::Circular;
        let sizes = fragment_sizes(&seq, &[&eco, &bam]);
        assert_eq!(sizes.len(), 2);
        // Each 4-nt overhang duplex is counted on both fragments.
        assert_eq!(sizes.iter().sum::<usize>(), seq.len() + 8);
        assert!(migration(10000) < migration(250));
        assert!((migration(250) - 1.0).abs() < 1e-9);
    }
}
