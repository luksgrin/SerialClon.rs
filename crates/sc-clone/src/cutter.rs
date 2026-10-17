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

/// A DNA ladder of the virtual gel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ladder {
    /// Name, as the original labels it (`GeneRuler 1kb Plus [Fermentas]`).
    pub name: String,
    /// Band sizes in bp, in the file's order (the original lists them
    /// ascending), with whether the band is a bright reference band.
    pub bands: Vec<(usize, bool)>,
}

impl Ladder {
    /// Parses one line of `resources/ladders.txt` (the original's layout:
    /// name, then the sizes; a leading `-` marks a bright band). Comment
    /// lines (`#`) and lines without a valid band give `None`.
    pub fn parse_line(line: &str) -> Option<Ladder> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let mut fields = line.split(',');
        let name = fields.next()?.trim();
        let mut bands = Vec::new();
        for f in fields {
            let f = f.trim();
            let (bright, digits) = match f.strip_prefix('-') {
                Some(d) => (true, d),
                None => (false, f),
            };
            let size: usize = digits.parse().ok()?;
            if size == 0 {
                return None;
            }
            bands.push((size, bright));
        }
        if name.is_empty() || bands.is_empty() {
            return None;
        }
        Some(Ladder {
            name: name.to_owned(),
            bands,
        })
    }

    /// Smallest and largest band.
    pub fn range(&self) -> (usize, usize) {
        let lo = self.bands.iter().map(|b| b.0).min().unwrap_or(250);
        let hi = self.bands.iter().map(|b| b.0).max().unwrap_or(10000);
        (lo, hi)
    }
}

/// The ladders shipped with the application: the original's four
/// (GeneRuler 1 kb Plus, Smart Ladder, NEB 1 kb, Invitrogen 1 kb plus) in
/// its order, then our generic 1 kb ladder. The first is the default.
pub fn ladders() -> Vec<Ladder> {
    include_str!("../../../resources/ladders.txt")
        .lines()
        .filter_map(Ladder::parse_line)
        .collect()
}

/// Migration of a fragment on a 1 % agarose gel as a fraction of the lane
/// (0 = well, 1 = front), a log-linear model calibrated on the 1 kb ladder.
pub fn migration(size: usize) -> f64 {
    migration_between(size, 250, 10000)
}

/// [`migration`] calibrated on a ladder's smallest and largest bands.
pub fn migration_between(size: usize, smallest: usize, largest: usize) -> f64 {
    let s = (size.max(50) as f64).log10();
    let (lo, hi) = (
        (smallest.max(50) as f64).log10(),
        (largest.max(smallest + 1) as f64).log10(),
    );
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

    #[test]
    fn ladders_are_the_originals_then_ours() {
        let l = ladders();
        assert_eq!(l.len(), 5);
        assert_eq!(l[0].name, "GeneRuler 1kb Plus [Fermentas]");
        assert_eq!(l[0].bands.len(), 15);
        assert_eq!(l[0].bands[0], (75, false));
        assert_eq!(l[0].bands[4], (500, true));
        assert_eq!(l[0].bands[14], (20000, false));
        assert_eq!(l[0].range(), (75, 20000));
        assert_eq!(l[1].name, "Smart Ladder [Eurogentec]");
        assert_eq!(l[2].name, "1 kb DNA Ladder [NEB]");
        assert_eq!(
            l[2].bands
                .iter()
                .filter(|b| b.1)
                .map(|b| b.0)
                .collect::<Vec<_>>(),
            vec![3000]
        );
        assert_eq!(l[3].name, "1 kb plus DNA Ladder [Invitrogen]");
        assert_eq!(l[3].bands.len(), 20);
        assert_eq!(l[4].name, "1 kb ladder (generic)");
        // Parsing: comments, blanks and malformed sizes are rejected.
        assert!(Ladder::parse_line("# comment").is_none());
        assert!(Ladder::parse_line("").is_none());
        assert!(Ladder::parse_line("x,100,abc").is_none());
        assert!(Ladder::parse_line("x").is_none());
        let (lo, hi) = l[0].range();
        assert!((migration_between(lo, lo, hi) - 1.0).abs() < 1e-9);
        assert!(migration_between(hi, lo, hi).abs() < 1e-9);
    }
}
