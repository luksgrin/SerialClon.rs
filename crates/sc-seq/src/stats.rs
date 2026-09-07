//! Composition, melting temperature and molecular weight, exactly as the
//! Sequence Window's "Selection Information" panel computes them.

/// Base composition of a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BaseCounts {
    /// A count.
    pub a: u32,
    /// C count.
    pub c: u32,
    /// G count.
    pub g: u32,
    /// T (or U) count.
    pub t: u32,
    /// Anything else (degenerate codes, gaps).
    pub other: u32,
}

impl BaseCounts {
    /// Counts the bases of `bases`, case-insensitively.
    pub fn of(bases: &[u8]) -> Self {
        let mut c = BaseCounts::default();
        for &b in bases {
            match b.to_ascii_uppercase() {
                b'A' => c.a += 1,
                b'C' => c.c += 1,
                b'G' => c.g += 1,
                b'T' | b'U' => c.t += 1,
                _ => c.other += 1,
            }
        }
        c
    }

    /// Total number of bases.
    pub fn total(&self) -> u32 {
        self.a + self.c + self.g + self.t + self.other
    }

    /// %GC over all bases (0 for an empty selection).
    pub fn gc_percent(&self) -> f64 {
        let n = self.total();
        if n == 0 {
            0.0
        } else {
            100.0 * f64::from(self.g + self.c) / f64::from(n)
        }
    }
}

/// The salt concentration the original assumed, in mol/L. Derived from the
/// manual's Info-panel screenshot: 666 nt, 50.0 %GC, Tm 79.7 °C solves to
/// [Na+] = 0.050 M, which also makes the Wallace-rule salt terms cancel.
pub const DEFAULT_NA_MOLAR: f64 = 0.050;

/// Melting temperature in °C, with the three formulas the User Manual gives
/// (Part I, §IV.A "Selection Information and Translation"):
///
/// - `n < 14`: `(A+T)·2 + (G+C)·4 − 16.6·log10(0.050) + 16.6·log10([Na+])`
/// - `14 ≤ n ≤ 50`: `100.5 + 41·(G+C)/n − 820/n + 16.6·log10([Na+])`
/// - `n > 50`: `81.5 + 41·(G+C)/n − 500/n + 16.6·log10([Na+])`
///
/// The manual's own boundaries read "<14", ">51" and "larger", which cannot
/// all be right; the split at 50 is the textbook one and the screenshot's
/// 666-nt selection confirms the last formula for long fragments. Degenerate
/// bases count towards `n` but not towards A+T or G+C.
pub fn melting_temperature(counts: &BaseCounts, na_molar: f64) -> f64 {
    let n = f64::from(counts.total());
    if n == 0.0 {
        return 0.0;
    }
    let at = f64::from(counts.a + counts.t);
    let gc = f64::from(counts.g + counts.c);
    let salt = 16.6 * na_molar.log10();
    if n < 14.0 {
        at * 2.0 + gc * 4.0 - 16.6 * 0.050f64.log10() + salt
    } else if n <= 50.0 {
        100.5 + 41.0 * gc / n - 820.0 / n + salt
    } else {
        81.5 + 41.0 * gc / n - 500.0 / n + salt
    }
}

/// Integer residue mass (Da) of an amino acid. `*` and unknown letters
/// weigh nothing.
///
/// These are the average residue masses truncated to whole daltons. The
/// original sums exactly these with **no water term**: it is the only common
/// variant that reproduces "MW: 24,12 kDa" for the 221-aa peptide in the
/// manual's Info-panel screenshot (fractional tables give 24.13–24.17).
fn residue_mass(aa: u8) -> u32 {
    match aa.to_ascii_uppercase() {
        b'A' => 71,
        b'R' => 156,
        b'N' => 114,
        b'D' => 115,
        b'C' => 103,
        b'E' => 129,
        b'Q' => 128,
        b'G' => 57,
        b'H' => 137,
        b'I' => 113,
        b'L' => 113,
        b'K' => 128,
        b'M' => 131,
        b'F' => 147,
        b'P' => 97,
        b'S' => 87,
        b'T' => 101,
        b'W' => 186,
        b'Y' => 163,
        b'V' => 99,
        _ => 0,
    }
}

/// Molecular weight of a peptide in Da, computed as Serial Cloner does (see
/// [`residue_mass`]). Stops and unknown letters are ignored.
pub fn protein_mw(peptide: &[u8]) -> f64 {
    f64::from(peptide.iter().map(|&aa| residue_mass(aa)).sum::<u32>())
}

/// Number of amino acids in a peptide, not counting stops.
pub fn peptide_len(peptide: &[u8]) -> usize {
    peptide.iter().filter(|&&aa| aa != b'*').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_gc() {
        let c = BaseCounts::of(b"acgtACGTNN");
        assert_eq!((c.a, c.c, c.g, c.t, c.other), (2, 2, 2, 2, 2));
        assert_eq!(c.total(), 10);
        assert!((c.gc_percent() - 40.0).abs() < 1e-9);
    }

    #[test]
    fn tm_matches_the_manual_screenshot() {
        // reference/ui/04-selection-info-and-translation.png:
        // A: 179  C: 157  G: 176  T: 154  %GC: 50.0  Tm: 79.7
        let c = BaseCounts {
            a: 179,
            c: 157,
            g: 176,
            t: 154,
            other: 0,
        };
        assert!((c.gc_percent() - 50.0).abs() < 0.05);
        let tm = melting_temperature(&c, DEFAULT_NA_MOLAR);
        assert_eq!(format!("{tm:.1}"), "79.7");
    }

    #[test]
    fn wallace_rule_for_short_oligos() {
        // 12-mer, 6 AT + 6 GC: 12 + 24 = 36, salt terms cancel at 0.05 M.
        let c = BaseCounts::of(b"AAATTTGGGCCC");
        assert!((melting_temperature(&c, DEFAULT_NA_MOLAR) - 36.0).abs() < 1e-9);
    }

    #[test]
    fn mid_length_formula() {
        let c = BaseCounts::of(&[b'G'; 20]);
        let expected = 100.5 + 41.0 - 820.0 / 20.0 + 16.6 * 0.05f64.log10();
        assert!((melting_temperature(&c, DEFAULT_NA_MOLAR) - expected).abs() < 1e-9);
    }

    #[test]
    fn mw_matches_the_manual_screenshot() {
        // Same screenshot: "Size (aa): 221   MW: 24,12 kDa". The peptide is
        // pBluescript II KS(+) 324..989 (the Sequence Map screenshot's
        // "Translate From 324 To 989"), whose base counts are exactly the
        // screenshot's 179/157/176/154.
        let pep = b"MMSTFKVLLCGAVLSRIDAGQEQLGRRIHYSQNDLVEYSPVTEKHLTDGMTVRELCSAAITMSDNTAANLLLTTIGGPKELTAFLHNMGDHVTRLDRWEPELNEAIPNDERDTTMPVAMATTLRKLLTGELLTLASRQQLIDWMEADKVAGPLLRSALPAGWFIADKSGAGERGSRGIIAALGPDGKPSRIVVIYTTGSQATMDERNRQIAEIGASLIKHW*";
        assert_eq!(peptide_len(pep), 221);
        assert_eq!(format!("{:.2}", protein_mw(pep) / 1000.0), "24.12");
    }
}
