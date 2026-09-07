//! Raw text: whatever letters are in the file.

use sc_seq::alphabet::{is_acgt, is_iupac};
use sc_seq::{SeqType, Sequence, Source};

/// Extracts the bases from free text, keeping case. With `degenerate` false
/// only `ACGT` survive (the manual: "Only ACGT letters are accepted. If other
/// characters are filtered out a message will be displayed reporting the total
/// number of excluded characters"); with it true the whole IUPAC alphabet
/// does. Whitespace and digits are never counted as excluded.
pub fn extract_bases(text: &str, degenerate: bool) -> (Vec<u8>, usize) {
    let mut bases = Vec::with_capacity(text.len());
    let mut excluded = 0;
    for ch in text.bytes() {
        if ch.is_ascii_whitespace() || ch.is_ascii_digit() {
            continue;
        }
        let keep = if degenerate {
            is_iupac(ch)
        } else {
            is_acgt(ch)
        };
        if keep { bases.push(ch) } else { excluded += 1 }
    }
    (bases, excluded)
}

/// Reads a text file as a sequence. Degenerate codes are kept, and the
/// sequence type is set accordingly, so nothing is silently lost; the
/// Sequence Window can still offer to filter.
pub fn read(bytes: &[u8], name: &str) -> Sequence {
    let text = String::from_utf8_lossy(bytes);
    let (bases, _) = extract_bases(&text, true);
    let plain = bases.iter().all(|&b| is_acgt(b));
    Sequence {
        name: name.to_owned(),
        seq_type: if plain {
            SeqType::Dna
        } else {
            SeqType::DegenerateDna
        },
        source: Source::RawText,
        bases,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_counts() {
        assert_eq!(
            extract_bases("ac gt\n12 ACGT-x N", false),
            (b"acgtACGT".to_vec(), 3)
        );
        assert_eq!(extract_bases("acgt N-", true), (b"acgtN".to_vec(), 1));
    }

    #[test]
    fn degenerate_text_is_typed_as_such() {
        assert_eq!(read(b"ACGT", "a").seq_type, SeqType::Dna);
        assert_eq!(read(b"ACGTN", "a").seq_type, SeqType::DegenerateDna);
    }
}
