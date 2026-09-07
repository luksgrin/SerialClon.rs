//! Silent Map (changelog 1.3): restriction sites that can be introduced
//! into a reading frame by mutations that do not change the peptide.

use sc_enzyme::Library;
use sc_seq::Sequence;
use sc_seq::alphabet::mask;
use sc_seq::translate::STANDARD;

/// A site that could be introduced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SilentSite {
    /// Enzyme name.
    pub enzyme: String,
    /// 1-based first base of the would-be site.
    pub position: usize,
    /// Number of base changes needed (≥ 1).
    pub changes: usize,
    /// The mutated stretch (top strand, the site's span).
    pub mutated: String,
}

const BASES: [u8; 4] = *b"ACGT";

/// Scans the reading frame `from..to` (0-based, half-open, `from` on a codon
/// boundary) of `seq` for every enzyme of the library whose site can be
/// created by silent mutations. Sites already present are not reported.
pub fn silent_map(seq: &Sequence, library: &Library, from: usize, to: usize) -> Vec<SilentSite> {
    let n = seq.len();
    let (from, to) = (from.min(n), to.min(n));
    let region: Vec<u8> = seq.bases[from..to]
        .iter()
        .map(|b| b.to_ascii_uppercase())
        .collect();
    let len = region.len();
    let mut out = Vec::new();
    for enzyme in &library.enzymes {
        let site = &enzyme.site;
        let l = site.len();
        if l == 0 || l > len {
            continue;
        }
        'pos: for i in 0..=len - l {
            let window = &region[i..i + l];
            if window
                .iter()
                .zip(site)
                .all(|(&b, &p)| mask(b) != 0 && mask(b) & !mask(p) == 0)
            {
                continue;
            }
            let c0 = i / 3;
            let c1 = (i + l - 1) / 3;
            let mut mutated = window.to_vec();
            let mut changes = 0;
            for c in c0..=c1 {
                let cs = c * 3;
                if cs + 3 > len {
                    continue 'pos;
                }
                let original = &region[cs..cs + 3];
                let aa = STANDARD.amino_acid(original);
                if aa == b'X' {
                    continue 'pos;
                }
                let req: Vec<u8> = (0..3)
                    .map(|k| {
                        let g = cs + k;
                        if g >= i && g < i + l {
                            mask(site[g - i])
                        } else {
                            mask(original[k])
                        }
                    })
                    .collect();
                let mut best: Option<([u8; 3], usize)> = None;
                for &a in &BASES {
                    for &b in &BASES {
                        for &d in &BASES {
                            let codon = [a, b, d];
                            if codon.iter().zip(&req).any(|(&x, &m)| mask(x) & m == 0) {
                                continue;
                            }
                            if STANDARD.amino_acid(&codon) != aa {
                                continue;
                            }
                            let diff = codon.iter().zip(original).filter(|(x, y)| x != y).count();
                            if best.is_none_or(|(_, d0)| diff < d0) {
                                best = Some((codon, diff));
                            }
                        }
                    }
                }
                let Some((codon, diff)) = best else {
                    continue 'pos;
                };
                changes += diff;
                for (k, &base) in codon.iter().enumerate() {
                    let g = cs + k;
                    if g >= i && g < i + l {
                        mutated[g - i] = base;
                    }
                }
            }
            if changes == 0 {
                continue;
            }
            out.push(SilentSite {
                enzyme: enzyme.name.clone(),
                position: from + i + 1,
                changes,
                mutated: String::from_utf8_lossy(&mutated).into_owned(),
            });
        }
    }
    out.sort_by(|a, b| {
        a.position
            .cmp(&b.position)
            .then(a.changes.cmp(&b.changes))
            .then(a.enzyme.cmp(&b.enzyme))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_silent_sites_only() {
        // ATG GCC CTG GAA TTC GGC AGC TAA: GAATTC (EcoRI) already present;
        // GGC AGC (G S) can become GGA TCC (BamHI) silently.
        let lib = Library::parse("EcoRI,g/aattc,\nBamHI,g/gatcc,\n");
        let seq = Sequence::new("t", b"ATGGCCCTGGAATTCGGCAGCTAA".to_vec());
        let sites = silent_map(&seq, &lib, 0, seq.len());
        let bam = sites
            .iter()
            .find(|s| s.enzyme == "BamHI")
            .expect("BamHI can be introduced silently");
        assert_eq!(bam.position, 16);
        assert_eq!(bam.mutated, "GGATCC");
        assert_eq!(bam.changes, 3); // GGC→GGA (1) and AGC→TCC (2)
        assert!(
            !sites
                .iter()
                .any(|s| s.enzyme == "EcoRI" && s.position == 10),
            "existing sites are not reported"
        );
        let mut mutated = seq.bases.clone();
        mutated[15..21].copy_from_slice(bam.mutated.as_bytes());
        assert_eq!(
            STANDARD.translate(&mutated, true),
            STANDARD.translate(&seq.bases, true)
        );
    }
}
