//! Finding recognition sites on both strands.

use sc_seq::alphabet::{matches_at, reverse_complement};
use sc_seq::{Strand, Topology};

use crate::Enzyme;

/// One occurrence of a recognition site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// 0-based index of the first base of the site on the top strand
    /// (for a reverse-strand hit, of the site's reverse complement).
    pub position: usize,
    /// Strand the site reads on. Always [`Strand::Forward`] for palindromes.
    pub strand: Strand,
    /// 0-based top-strand cut: the cut lies before base `cut_top`. Reduced
    /// modulo the length on circular sequences.
    pub cut_top: usize,
    /// 0-based bottom-strand cut in top-strand coordinates.
    pub cut_bottom: usize,
}

impl Site {
    /// 1-based position of the first base of the site, as maps show it.
    pub fn position_1(&self) -> usize {
        self.position + 1
    }
}

/// Finds every site of `enzyme` in `bases`.
///
/// Non-palindromic sites (type IIs and asymmetric type II) are searched on
/// both strands and reported with the strand they read on; the maps draw
/// those with `>` / `<` arrows. Circular sequences are searched across the
/// origin. Results are sorted by position, then forward before reverse. Hits
/// whose cut would fall outside a linear sequence are dropped (the enzyme
/// binds but cannot cut).
pub fn find_sites(bases: &[u8], topology: Topology, enzyme: &Enzyme) -> Vec<Site> {
    let n = bases.len();
    let len = enzyme.site.len();
    let mut out = Vec::new();
    if n == 0 || len == 0 || (n < len && !topology.is_circular()) {
        return out;
    }
    // Window over the sequence, extended past the origin for circular ones.
    let extended: Vec<u8> = if topology.is_circular() {
        let mut v = bases.to_vec();
        v.extend_from_slice(&bases[..bases.len().min(len - 1)]);
        v
    } else {
        bases.to_vec()
    };
    let rc_site = reverse_complement(&enzyme.site);
    let both = !enzyme.is_palindromic();
    let last = if topology.is_circular() {
        n
    } else {
        n - len + 1
    };
    for pos in 0..last {
        let window = &extended[pos..pos + len];
        if matches_at(&enzyme.site, window) {
            push(&mut out, pos, Strand::Forward, enzyme, n, topology);
        }
        if both && matches_at(&rc_site, window) {
            push(&mut out, pos, Strand::Reverse, enzyme, n, topology);
        }
    }
    out.sort_by_key(|s| (s.position, s.strand == Strand::Reverse));
    out
}

fn push(
    out: &mut Vec<Site>,
    pos: usize,
    strand: Strand,
    enzyme: &Enzyme,
    n: usize,
    topology: Topology,
) {
    let len = enzyme.site.len() as i64;
    let (top, bottom) = match strand {
        Strand::Forward => (
            pos as i64 + enzyme.cut_top as i64,
            pos as i64 + enzyme.cut_bottom as i64,
        ),
        // On the bottom strand the enzyme's own "top" cut is our bottom cut,
        // measured from the far end of the site.
        Strand::Reverse => (
            pos as i64 + len - enzyme.cut_bottom as i64,
            pos as i64 + len - enzyme.cut_top as i64,
        ),
    };
    if topology.is_circular() {
        let m = |x: i64| x.rem_euclid(n as i64) as usize;
        out.push(Site {
            position: pos,
            strand,
            cut_top: m(top),
            cut_bottom: m(bottom),
        });
    } else if (0..=n as i64).contains(&top) && (0..=n as i64).contains(&bottom) {
        out.push(Site {
            position: pos,
            strand,
            cut_top: top as usize,
            cut_bottom: bottom as usize,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::parse_entry;

    #[test]
    fn palindromic_site_once_per_position() {
        let eco = parse_entry("EcoRI,g/aattc,").unwrap();
        let sites = find_sites(b"ccGAATTCccgaattc", Topology::Linear, &eco);
        assert_eq!(sites.len(), 2);
        assert_eq!(
            (sites[0].position, sites[0].cut_top, sites[0].cut_bottom),
            (2, 3, 7)
        );
        assert_eq!(sites[1].position, 10);
        assert!(sites.iter().all(|s| s.strand == Strand::Forward));
    }

    #[test]
    fn type_iis_on_both_strands() {
        let bsa = parse_entry("#BsaI,ggtctc,1,5,").unwrap();
        // forward: GGTCTC N^NNNN_ ; reverse complement GAGACC.
        let dna = b"aaGGTCTCaTGCAtttGAGACCaaaaaa";
        let sites = find_sites(dna, Topology::Linear, &bsa);
        assert_eq!(sites.len(), 2);
        let f = &sites[0];
        assert_eq!(
            (f.position, f.strand, f.cut_top, f.cut_bottom),
            (2, Strand::Forward, 9, 13)
        );
        let r = &sites[1];
        // Reverse hit at 16..22: the enzyme cuts 1 and 5 bases *before* the
        // site on the top strand: bottom cut at 16-1=15, top cut at 16-5=11.
        assert_eq!(
            (r.position, r.strand, r.cut_top, r.cut_bottom),
            (16, Strand::Reverse, 11, 15)
        );
    }

    #[test]
    fn circular_sites_cross_the_origin() {
        let eco = parse_entry("EcoRI,g/aattc,").unwrap();
        // "ATTCxxxxGA" wraps to GAATTC.
        let dna = b"ATTCttttGA";
        assert!(find_sites(dna, Topology::Linear, &eco).is_empty());
        let sites = find_sites(dna, Topology::Circular, &eco);
        assert_eq!(sites.len(), 1);
        assert_eq!(
            (sites[0].position, sites[0].cut_top, sites[0].cut_bottom),
            (8, 9, 3)
        );
    }

    #[test]
    fn linear_hits_that_cannot_be_cut_are_dropped() {
        let bsa = parse_entry("#BsaI,ggtctc,1,5,").unwrap();
        assert!(find_sites(b"GGTCTCa", Topology::Linear, &bsa).is_empty());
        assert_eq!(find_sites(b"GGTCTCaaaaa", Topology::Linear, &bsa).len(), 1);
    }
}
