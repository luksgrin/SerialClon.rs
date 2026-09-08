//! IUPAC nucleotide alphabet: complementing and degenerate matching.

/// The four-bit mask of a nucleotide code: A=1, C=2, G=4, T=8.
///
/// Returns 0 for anything that is not an IUPAC code (gaps, digits, protein).
pub fn mask(base: u8) -> u8 {
    match base.to_ascii_uppercase() {
        b'A' => 0b0001,
        b'C' => 0b0010,
        b'G' => 0b0100,
        b'T' | b'U' => 0b1000,
        b'R' => 0b0101, // A G
        b'Y' => 0b1010, // C T
        b'S' => 0b0110, // C G
        b'W' => 0b1001, // A T
        b'K' => 0b1100, // G T
        b'M' => 0b0011, // A C
        b'B' => 0b1110, // not A
        b'D' => 0b1101, // not C
        b'H' => 0b1011, // not G
        b'V' => 0b0111, // not T
        b'N' | b'X' => 0b1111,
        _ => 0,
    }
}

/// The IUPAC code for a four-bit mask (upper case); `N` for an empty mask.
pub fn from_mask(mask: u8) -> u8 {
    match mask & 0b1111 {
        0b0001 => b'A',
        0b0010 => b'C',
        0b0100 => b'G',
        0b1000 => b'T',
        0b0101 => b'R',
        0b1010 => b'Y',
        0b0110 => b'S',
        0b1001 => b'W',
        0b1100 => b'K',
        0b0011 => b'M',
        0b1110 => b'B',
        0b1101 => b'D',
        0b1011 => b'H',
        0b0111 => b'V',
        _ => b'N',
    }
}

/// True for the plain alphabet Serial Cloner's "DNA" windows accept.
pub fn is_acgt(base: u8) -> bool {
    matches!(base.to_ascii_uppercase(), b'A' | b'C' | b'G' | b'T')
}

/// True for any IUPAC nucleotide code (the "degenerate DNA" alphabet).
pub fn is_iupac(base: u8) -> bool {
    mask(base) != 0
}

/// Watson–Crick complement of one IUPAC code, preserving case. Unknown bytes
/// are returned unchanged.
pub fn complement(base: u8) -> u8 {
    let c = match base.to_ascii_uppercase() {
        b'A' => b'T',
        b'T' => b'A',
        b'U' => b'A',
        b'C' => b'G',
        b'G' => b'C',
        b'R' => b'Y',
        b'Y' => b'R',
        b'S' => b'S',
        b'W' => b'W',
        b'K' => b'M',
        b'M' => b'K',
        b'B' => b'V',
        b'V' => b'B',
        b'D' => b'H',
        b'H' => b'D',
        _ => return base,
    };
    if base.is_ascii_lowercase() {
        c.to_ascii_lowercase()
    } else {
        c
    }
}

/// The reverse complement ("Antiparallel", ⌘/), case preserved.
pub fn reverse_complement(bases: &[u8]) -> Vec<u8> {
    bases.iter().rev().map(|&b| complement(b)).collect()
}

/// The complement without reversing (Sequence ▸ Complement).
pub fn complement_all(bases: &[u8]) -> Vec<u8> {
    bases.iter().map(|&b| complement(b)).collect()
}

/// True if `pattern` (IUPAC, e.g. an enzyme site) can match `bases` exactly,
/// i.e. every position of `bases` is a subset of the pattern's possibilities.
///
/// A degenerate base *in the sequence* (`N` in a plasmid) only matches if the
/// pattern accepts every one of its possibilities: Serial Cloner treats
/// non-ACGT nucleotides as unknown for restriction maps, and an unknown base
/// must not create sites out of thin air.
pub fn matches_at(pattern: &[u8], bases: &[u8]) -> bool {
    pattern.len() == bases.len()
        && pattern.iter().zip(bases).all(|(&p, &b)| {
            let (pm, bm) = (mask(p), mask(b));
            bm != 0 && bm & !pm == 0
        })
}

/// True if the pattern is its own reverse complement (a palindromic site such
/// as `gaattc` or `gtmkac`), in which case it occurs at the same positions on
/// both strands.
pub fn is_palindromic(pattern: &[u8]) -> bool {
    let rc = reverse_complement(pattern);
    pattern.iter().zip(&rc).all(|(&a, &b)| mask(a) == mask(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn antiparallel_matches_the_manual_example() {
        // "AAGGAAGG becomes CCTTCCTT" (User Manual, Sequence menu).
        assert_eq!(reverse_complement(b"AAGGAAGG"), b"CCTTCCTT");
        assert_eq!(complement_all(b"AAGGAAGG"), b"TTCCTTCC");
    }

    #[test]
    fn complement_preserves_case_and_degeneracy() {
        assert_eq!(reverse_complement(b"acgtRYKMbdhvn"), b"nbdhvKMRYacgt");
    }

    #[test]
    fn degenerate_matching() {
        assert!(matches_at(b"gtmkac", b"GTAGAC"));
        assert!(matches_at(b"gtmkac", b"gtctac"));
        assert!(!matches_at(b"gtmkac", b"GTAAAC"));
        // N in the sequence never satisfies a specific pattern base...
        assert!(!matches_at(b"gaattc", b"GAATTN"));
        // ...but does satisfy an N in the pattern.
        assert!(matches_at(b"gacnnnnnngtc", b"GACAAANNNGTC"));
    }

    #[test]
    fn palindromes() {
        assert!(is_palindromic(b"gaattc"));
        assert!(is_palindromic(b"gtmkac"));
        assert!(is_palindromic(b"gacnnnnnngtc"));
        assert!(!is_palindromic(b"cacctgc"));
        assert!(!is_palindromic(b"ggtctc"));
    }
}
