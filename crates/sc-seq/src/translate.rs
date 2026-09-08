//! Translation and open reading frames.

use crate::alphabet::{mask, reverse_complement};
use crate::sequence::{Strand, Topology};

/// A genetic code: 64 amino acids indexed by codon (`TCAG` order, T=0 … G=3,
/// first base most significant), plus the codons accepted as starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneticCode {
    /// NCBI translation table id (1 = standard).
    pub id: u8,
    /// Human-readable name.
    pub name: &'static str,
    /// Amino acids in `TTT TTC TTA TTG TCT …` order; `*` is stop.
    pub amino_acids: [u8; 64],
    /// Alternative start codons, in the same indexing, `M` where a start.
    pub starts: [u8; 64],
}

const TCAG: &[u8; 4] = b"TCAG";

/// NCBI table 1, the standard code (Window ▸ Standard Genetic Code).
pub static STANDARD: GeneticCode = GeneticCode {
    id: 1,
    name: "Standard",
    amino_acids: *b"FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    starts: *b"---M------**--*----M---------------M----------------------------",
};

impl GeneticCode {
    /// Looks a table up by NCBI id (Protein ▸ Choose genetic code).
    pub fn by_id(id: u8) -> Option<&'static GeneticCode> {
        crate::genetic_codes::GENETIC_CODES
            .iter()
            .find(|c| c.id == id)
    }

    /// Every codon that starts translation in this code, as `ATG`-style text.
    pub fn start_codons(&self) -> Vec<String> {
        (0..64usize)
            .filter(|&i| self.starts[i] == b'M')
            .map(|i| {
                String::from_utf8_lossy(&[TCAG[i >> 4], TCAG[(i >> 2) & 3], TCAG[i & 3]])
                    .into_owned()
            })
            .collect()
    }

    /// Every stop codon of this code.
    pub fn stop_codons(&self) -> Vec<String> {
        (0..64usize)
            .filter(|&i| self.amino_acids[i] == b'*')
            .map(|i| {
                String::from_utf8_lossy(&[TCAG[i >> 4], TCAG[(i >> 2) & 3], TCAG[i & 3]])
                    .into_owned()
            })
            .collect()
    }
}

/// Index of a codon in a [`GeneticCode`] table, or `None` if any base is not
/// a plain `ACGTU`.
pub fn codon_index(codon: &[u8]) -> Option<usize> {
    if codon.len() != 3 {
        return None;
    }
    let mut i = 0;
    for &b in codon {
        let b = b.to_ascii_uppercase();
        let b = if b == b'U' { b'T' } else { b };
        i = i * 4 + TCAG.iter().position(|&x| x == b)?;
    }
    Some(i)
}

impl GeneticCode {
    /// Translates one codon. Degenerate codons yield the amino acid if every
    /// possible expansion agrees, otherwise `X`. Codons shorter than three
    /// bases yield `X` as well.
    pub fn amino_acid(&self, codon: &[u8]) -> u8 {
        if let Some(i) = codon_index(codon) {
            return self.amino_acids[i];
        }
        if codon.len() != 3 {
            return b'X';
        }
        let masks: Vec<u8> = codon.iter().map(|&b| mask(b)).collect();
        if masks.contains(&0) {
            return b'X';
        }
        let mut result = None;
        for (i, &aa) in self.amino_acids.iter().enumerate() {
            // Codon i expands to bases TCAG[i>>4], TCAG[(i>>2)&3], TCAG[i&3].
            let bits = [TCAG[i >> 4], TCAG[(i >> 2) & 3], TCAG[i & 3]].map(mask);
            if bits.iter().zip(&masks).all(|(b, m)| b & m != 0) {
                match result {
                    None => result = Some(aa),
                    Some(r) if r == aa => {}
                    Some(_) => return b'X',
                }
            }
        }
        result.unwrap_or(b'X')
    }

    /// True if the codon is a start codon in this code.
    pub fn is_start(&self, codon: &[u8]) -> bool {
        codon_index(codon).is_some_and(|i| self.starts[i] == b'M')
    }

    /// True if the codon is a stop codon.
    pub fn is_stop(&self, codon: &[u8]) -> bool {
        codon_index(codon).is_some_and(|i| self.amino_acids[i] == b'*')
    }

    /// Translates `bases` from the first base, codon by codon.
    ///
    /// With `stop_through` false the peptide ends at the first in-frame stop
    /// (which is included as `*`, as the Info drawer shows it); with it true
    /// every stop is written as `*` and translation continues (Info drawer
    /// "Show only 1st ORF" unchecked). A trailing partial codon is ignored.
    pub fn translate(&self, bases: &[u8], stop_through: bool) -> String {
        let mut out = String::with_capacity(bases.len() / 3 + 1);
        for codon in bases.chunks_exact(3) {
            let aa = self.amino_acid(codon);
            out.push(aa as char);
            if aa == b'*' && !stop_through {
                break;
            }
        }
        out
    }
}

/// An open reading frame found by [`find_orfs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orf {
    /// 1-based position of the first base of the start codon, in sequence
    /// coordinates (for reverse ORFs this is the higher coordinate).
    pub start: u32,
    /// 1-based position of the last base of the stop codon (or of the last
    /// complete codon for an incomplete ORF).
    pub end: u32,
    /// Strand.
    pub strand: Strand,
    /// Reading frame 1, 2 or 3 on its strand (Serial Cloner labels frame 0 as 3).
    pub frame: u8,
    /// Length in bases including the stop codon.
    pub length: u32,
    /// False if the sequence ended before a stop codon (shown in italic).
    pub complete: bool,
}

/// Finds every ORF (ATG … stop) of at least `min_codons` codons on both
/// strands, wrapping the origin of circular sequences. Sorted longest first,
/// as the Find drawer lists them.
pub fn find_orfs(
    bases: &[u8],
    topology: Topology,
    code: &GeneticCode,
    min_codons: usize,
) -> Vec<Orf> {
    let n = bases.len();
    let mut orfs = Vec::new();
    if n < 3 {
        return orfs;
    }
    let rc = reverse_complement(bases);
    for (strand, s) in [(Strand::Forward, bases), (Strand::Reverse, rc.as_slice())] {
        // For circular sequences scan a doubled copy so ORFs may cross the
        // origin, but only start in the first copy.
        let scan: Vec<u8> = if topology.is_circular() {
            let mut v = s.to_vec();
            v.extend_from_slice(s);
            v
        } else {
            s.to_vec()
        };
        let mut i = 0;
        while i + 3 <= n.min(scan.len()) {
            if code.is_start(&scan[i..i + 3]) {
                let mut j = i;
                let mut complete = false;
                while j + 3 <= scan.len() {
                    if code.is_stop(&scan[j..j + 3]) {
                        complete = true;
                        j += 3;
                        break;
                    }
                    j += 3;
                    if topology.is_circular() && j - i >= n {
                        break; // went all the way around
                    }
                }
                let len = j - i;
                if len / 3 >= min_codons && (complete || !topology.is_circular()) {
                    let (start, end) = match strand {
                        Strand::Forward => (i as u32 + 1, ((j - 1) % n) as u32 + 1),
                        // Map reverse-strand indices back to top-strand coordinates.
                        Strand::Reverse => ((n - i) as u32, (n - ((j - 1) % n)) as u32),
                    };
                    orfs.push(Orf {
                        start,
                        end,
                        strand,
                        frame: (i % 3) as u8 + 1,
                        length: len as u32,
                        complete,
                    });
                }
            }
            i += 1;
        }
    }
    orfs.sort_by(|a, b| b.length.cmp(&a.length).then(a.start.cmp(&b.start)));
    orfs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_code_basics() {
        assert_eq!(STANDARD.amino_acid(b"ATG"), b'M');
        assert_eq!(STANDARD.amino_acid(b"atg"), b'M');
        assert_eq!(STANDARD.amino_acid(b"TGG"), b'W');
        assert_eq!(STANDARD.amino_acid(b"TAA"), b'*');
        assert_eq!(STANDARD.amino_acid(b"TAG"), b'*');
        assert_eq!(STANDARD.amino_acid(b"TGA"), b'*');
        assert!(STANDARD.is_start(b"ATG"));
        assert!(
            !STANDARD.is_start(b"CTG") || STANDARD.starts[codon_index(b"CTG").unwrap()] == b'M'
        );
    }

    #[test]
    fn ncbi_tables_translate_their_documented_starts_and_stops() {
        let codes = crate::genetic_codes::GENETIC_CODES;
        assert_eq!(codes.len(), 27);
        assert_eq!(
            GeneticCode::by_id(1).unwrap().amino_acids,
            STANDARD.amino_acids
        );
        // Vertebrate mitochondrial: TGA = Trp, AGA/AGG = stop, ATA = Met start.
        let mito = GeneticCode::by_id(2).unwrap();
        assert_eq!(mito.amino_acid(b"TGA"), b'W');
        assert_eq!(mito.amino_acid(b"AGA"), b'*');
        assert_eq!(mito.amino_acid(b"ATA"), b'M');
        assert!(mito.is_start(b"ATA") && mito.is_start(b"ATT"));
        // Bacterial (11): GTG and TTG are starts.
        let bact = GeneticCode::by_id(11).unwrap();
        assert!(bact.is_start(b"GTG") && bact.is_start(b"TTG") && bact.is_start(b"ATG"));
        assert_eq!(bact.stop_codons(), ["TAA", "TAG", "TGA"]);
        // Ciliate (6): TAA/TAG = Gln, only TGA stops.
        let cil = GeneticCode::by_id(6).unwrap();
        assert_eq!(cil.amino_acid(b"TAA"), b'Q');
        assert_eq!(cil.stop_codons(), ["TGA"]);
        // Every table: 64 valid entries and at least one start codon.
        for c in codes {
            for (i, &aa) in c.amino_acids.iter().enumerate() {
                assert!(
                    aa.is_ascii_uppercase() || aa == b'*',
                    "{} entry {i}",
                    c.name
                );
            }
            assert!(
                !c.start_codons().is_empty(),
                "{} has no start codon",
                c.name
            );
        }
    }

    #[test]
    fn degenerate_codons() {
        assert_eq!(STANDARD.amino_acid(b"GCN"), b'A'); // four-fold degenerate
        assert_eq!(STANDARD.amino_acid(b"CTN"), b'L');
        assert_eq!(STANDARD.amino_acid(b"AAR"), b'K');
        assert_eq!(STANDARD.amino_acid(b"ANN"), b'X');
        assert_eq!(STANDARD.amino_acid(b"AT"), b'X');
    }

    #[test]
    fn translate_stops_or_reads_through() {
        let dna = b"ATGGCCTAAGGGTGA";
        assert_eq!(STANDARD.translate(dna, false), "MA*");
        assert_eq!(STANDARD.translate(dna, true), "MA*G*");
        assert_eq!(STANDARD.translate(b"ATGGC", false), "M");
    }

    #[test]
    fn orfs_on_both_strands() {
        // ATG GCC TAA on top; reverse complement of "ATGAAATAG" is at the end.
        let dna = b"ccATGGCCTAAggCTATTTCATgg";
        let orfs = find_orfs(dna, Topology::Linear, &STANDARD, 1);
        let fwd: Vec<_> = orfs
            .iter()
            .filter(|o| o.strand == Strand::Forward)
            .collect();
        let rev: Vec<_> = orfs
            .iter()
            .filter(|o| o.strand == Strand::Reverse)
            .collect();
        assert_eq!(
            (fwd[0].start, fwd[0].end, fwd[0].length, fwd[0].complete),
            (3, 11, 9, true)
        );
        assert_eq!(
            (rev[0].start, rev[0].end, rev[0].length, rev[0].complete),
            (22, 14, 9, true)
        );
        assert_eq!(fwd[0].frame, 3);
    }

    #[test]
    fn incomplete_orf_is_reported_on_linear_sequences() {
        let orfs = find_orfs(b"ATGGCCGGG", Topology::Linear, &STANDARD, 1);
        assert_eq!(orfs.len(), 1);
        assert!(!orfs[0].complete);
        assert_eq!((orfs[0].start, orfs[0].end), (1, 9));
    }

    #[test]
    fn orf_crosses_the_origin_of_a_circular_sequence() {
        // "GCCTAA" + "ggg" + "ATG": ATG at 10..12 continues at 1.
        let dna = b"GCCTAAgggATG";
        let orfs = find_orfs(dna, Topology::Circular, &STANDARD, 1);
        let fwd: Vec<_> = orfs
            .iter()
            .filter(|o| o.strand == Strand::Forward)
            .collect();
        assert_eq!((fwd[0].start, fwd[0].end, fwd[0].length), (10, 6, 9));
        assert!(fwd[0].complete);
    }
}
