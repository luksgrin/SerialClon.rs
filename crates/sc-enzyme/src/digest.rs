//! Cutting a sequence into fragments with cohesive ends.

use sc_seq::{Overhang, Topology};

use crate::Site;

/// A fragment produced by a digest, or selected between two sites on the
/// Graphic Map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    /// 0-based top-strand cut the fragment starts at.
    pub start: usize,
    /// 0-based top-strand cut the fragment ends at (`> start`, or the whole
    /// length back round for a single cut in a circle).
    pub end: usize,
    /// Double-stranded body plus the protruding bases, top strand 5'→3'.
    pub bases: Vec<u8>,
    /// Left (5') end.
    pub left: Overhang,
    /// Right (3') end.
    pub right: Overhang,
}

impl Fragment {
    /// Length of the top strand, overhangs included.
    pub fn len(&self) -> usize {
        self.bases.len()
    }

    /// True if there are no bases.
    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }
}

/// Cuts `bases` at every site. Cuts are ordered by top-strand position;
/// coincident cuts are merged. A linear sequence with no cuts is one blunt
/// fragment; a circular one with no cuts yields nothing. Overhangs come from
/// the difference between the top- and bottom-strand cuts of each site: a
/// bottom cut to the right of the top cut leaves a 5' overhang, to the left a
/// 3' overhang.
pub fn digest(bases: &[u8], topology: Topology, sites: &[Site]) -> Vec<Fragment> {
    let n = bases.len();
    let mut cuts: Vec<(usize, usize)> = sites.iter().map(|s| (s.cut_top, s.cut_bottom)).collect();
    cuts.sort_unstable();
    cuts.dedup_by_key(|c| c.0);
    let base = |i: usize| bases[i % n];
    let slice = |from: usize, len: usize| -> Vec<u8> { (0..len).map(|k| base(from + k)).collect() };
    let rc = sc_seq::alphabet::reverse_complement;
    // Overhangs. With the top cut at `t` and the bottom cut at `b`: when
    // `b > t` the bottom strand is the longer one on the left fragment and
    // the top strand on the right fragment, and both protrude at their 5'
    // ends; when `b < t` it is the other way round and they protrude at
    // their 3' ends. Bases are given 5'→3' on the strand that carries them.
    let overhang_left = |(t, b): (usize, usize)| -> Overhang {
        if b > t {
            Overhang::five_prime(&slice(t, b - t))
        } else if b < t {
            Overhang::three_prime(&rc(&slice(b, t - b)))
        } else {
            Overhang::BLUNT
        }
    };
    let overhang_right = |(t, b): (usize, usize)| -> Overhang {
        if b > t {
            Overhang::five_prime(&rc(&slice(t, b - t)))
        } else if b < t {
            Overhang::three_prime(&slice(b, t - b))
        } else {
            Overhang::BLUNT
        }
    };
    let mut out = Vec::new();
    if n == 0 {
        return out;
    }
    match topology {
        Topology::Linear => {
            let mut bounds = vec![(0usize, 0usize)];
            bounds.extend(cuts.iter().copied());
            bounds.push((n, n));
            for w in bounds.windows(2) {
                let (a, b) = (w[0], w[1]);
                let from = a.0.min(a.1);
                let to = b.0.max(b.1);
                out.push(Fragment {
                    start: a.0,
                    end: b.0,
                    bases: bases[from..to.min(n)].to_vec(),
                    left: if a.0 == 0 && a.1 == 0 {
                        Overhang::BLUNT
                    } else {
                        overhang_left(a)
                    },
                    right: if b.0 == n && b.1 == n {
                        Overhang::BLUNT
                    } else {
                        overhang_right(b)
                    },
                });
            }
        }
        Topology::Circular => {
            if cuts.is_empty() {
                return out;
            }
            for i in 0..cuts.len() {
                let a = cuts[i];
                let b = cuts[(i + 1) % cuts.len()];
                let from = a.0.min(a.1);
                let to = b.0.max(b.1);
                // Body length going forward from `from` to `to`; a single
                // cut yields the whole circle plus its two overhangs.
                let mut len = (to as i64 - from as i64).rem_euclid(n as i64) as usize;
                if cuts.len() == 1 {
                    len += n;
                }
                out.push(Fragment {
                    start: a.0,
                    end: b.0,
                    bases: slice(from, len),
                    left: overhang_left(a),
                    right: overhang_right(b),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find_sites;
    use crate::library::parse_entry;

    #[test]
    fn ecori_leaves_5_prime_aatt_overhangs() {
        let eco = parse_entry("EcoRI,g/aattc,").unwrap();
        let dna = b"cccGAATTCggg";
        let sites = find_sites(dna, Topology::Linear, &eco);
        let frags = digest(dna, Topology::Linear, &sites);
        assert_eq!(frags.len(), 2);
        assert_eq!(frags[0].bases, b"cccGAATT");
        assert_eq!(frags[0].left, Overhang::BLUNT);
        assert_eq!(frags[0].right, Overhang::five_prime(b"AATT"));
        assert_eq!(frags[1].bases, b"AATTCggg");
        assert_eq!(frags[1].left, Overhang::five_prime(b"AATT"));
        assert_eq!(frags[1].right, Overhang::BLUNT);
    }

    #[test]
    fn psti_leaves_3_prime_tgca_overhangs() {
        let pst = parse_entry("PstI,ctgca/g,").unwrap();
        let dna = b"aaCTGCAGtt";
        let frags = digest(
            dna,
            Topology::Linear,
            &find_sites(dna, Topology::Linear, &pst),
        );
        assert_eq!(frags[0].bases, b"aaCTGCA");
        assert_eq!(frags[0].right, Overhang::three_prime(b"TGCA"));
        assert_eq!(frags[1].bases, b"TGCAGtt");
        assert_eq!(frags[1].left, Overhang::three_prime(b"TGCA"));
    }

    #[test]
    fn single_cut_linearises_a_plasmid() {
        let eco = parse_entry("EcoRI,g/aattc,").unwrap();
        let dna = b"ggGAATTCcc";
        let frags = digest(
            dna,
            Topology::Circular,
            &find_sites(dna, Topology::Circular, &eco),
        );
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].bases, b"AATTCccggGAATT");
        assert_eq!(frags[0].left, Overhang::five_prime(b"AATT"));
        assert_eq!(frags[0].right, Overhang::five_prime(b"AATT"));
    }

    #[test]
    fn blunt_and_no_cut_cases() {
        let sma = parse_entry("SmaI,ccc/ggg,").unwrap();
        let dna = b"aCCCGGGt";
        let frags = digest(
            dna,
            Topology::Linear,
            &find_sites(dna, Topology::Linear, &sma),
        );
        assert_eq!(frags.len(), 2);
        assert!(frags[0].right.is_blunt() && frags[1].left.is_blunt());
        assert_eq!(digest(dna, Topology::Linear, &[]).len(), 1);
        assert!(digest(dna, Topology::Circular, &[]).is_empty());
    }
}
