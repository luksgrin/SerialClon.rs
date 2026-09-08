//! Bisulfite conversion (changelog 2.5: "Generate sense and antisense
//! strand after C>>T bisulfite conversion"). Unmethylated cytosines read as
//! thymines after sequencing; the converted antisense strand is the reverse
//! complement converted in turn, so that both products read 5'→3'.

use crate::alphabet::reverse_complement;

/// Which strand to convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strand {
    /// C→T on the sequence as written.
    Sense,
    /// C→T on the reverse complement.
    Antisense,
}

/// Converts every C to T (case preserved) on the chosen strand.
pub fn convert(bases: &[u8], strand: Strand) -> Vec<u8> {
    let source = match strand {
        Strand::Sense => bases.to_vec(),
        Strand::Antisense => reverse_complement(bases),
    };
    source
        .into_iter()
        .map(|b| match b {
            b'C' => b'T',
            b'c' => b't',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sense_and_antisense() {
        assert_eq!(convert(b"ACGTacgt", Strand::Sense), b"ATGTatgt");
        // rc(ACGTacgt) = acgtACGT → atgtATGT
        assert_eq!(convert(b"ACGTacgt", Strand::Antisense), b"atgtATGT");
        assert_eq!(convert(b"", Strand::Sense), b"");
    }
}
