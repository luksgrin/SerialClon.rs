//! FASTA, single and multi-record.
//!
//! Serial Cloner's own headers look like
//! `> [pBluescript II SK(+) - 2961 bp] gi|58063|… phagemid excised from lambda ZAPII`
//! (Copy as Fasta) or `> [pBluescript II KS(+)] gi|58061|… - 2961 bp` (Export to
//! Fasta); the bracketed part is the name and the rest the comment. Generic
//! headers are `>name comment`. Both are read; the first form is written.

use sc_seq::alphabet::{is_acgt, is_iupac};
use sc_seq::{SeqType, Sequence, Source};

use crate::{lines, raw};

/// Splits a header (without the `>`) into name and comment.
pub fn parse_header(header: &str) -> (String, String) {
    let h = header.trim();
    if let Some(rest) = h.strip_prefix('[')
        && let Some(close) = rest.find(']')
    {
        let inside = rest[..close].trim();
        let name = match inside.rsplit_once(" - ") {
            Some((n, size)) if size.trim_end_matches(" bp").trim().parse::<u64>().is_ok() => {
                n.trim()
            }
            _ => inside,
        };
        let mut comment = rest[close + 1..].trim().to_owned();
        if let Some((c, size)) = comment.rsplit_once(" - ")
            && size.trim_end_matches(" bp").trim().parse::<u64>().is_ok()
            && size.ends_with("bp")
        {
            comment = c.trim().to_owned();
        }
        return (name.to_owned(), comment);
    }
    match h.split_once(char::is_whitespace) {
        Some((name, comment)) => (name.to_owned(), comment.trim().to_owned()),
        None => (h.to_owned(), String::new()),
    }
}

/// Reads every record. A file with bases but no `>` header is one unnamed
/// record. Degenerate records are typed as such; lower case is kept.
pub fn read(bytes: &[u8]) -> Vec<Sequence> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    let mut current: Option<(String, String, String)> = None;
    let flush = |current: &mut Option<(String, String, String)>, out: &mut Vec<Sequence>| {
        if let Some((name, comment, body)) = current.take() {
            let (bases, _) = raw::extract_bases(&body, true);
            if bases.is_empty() && name.is_empty() {
                return;
            }
            let plain = bases.iter().all(|&b| is_acgt(b));
            let protein =
                !plain && bases.iter().filter(|&&b| !is_iupac(b)).count() > bases.len() / 10;
            out.push(Sequence {
                name,
                comment,
                seq_type: if protein {
                    SeqType::Protein
                } else if plain {
                    SeqType::Dna
                } else {
                    SeqType::DegenerateDna
                },
                source: Source::Fasta,
                bases,
                ..Default::default()
            });
        }
    };
    for line in lines(&text) {
        if let Some(header) = line.trim_start().strip_prefix('>') {
            flush(&mut current, &mut out);
            let (name, comment) = parse_header(header);
            current = Some((name, comment, String::new()));
        } else if !line.trim().is_empty() {
            let (_, _, body) =
                current.get_or_insert_with(|| (String::new(), String::new(), String::new()));
            body.push_str(line);
            body.push('\n');
        }
    }
    flush(&mut current, &mut out);
    out
}

/// Line width used when writing (the manual: "80-nucleotide-long lines").
pub const LINE_WIDTH: usize = 80;

/// Formats one sequence as FASTA with Serial Cloner's header.
pub fn write(seq: &Sequence) -> String {
    let mut out = format!("> [{} - {} bp]", seq.name, seq.len());
    let comment = seq.comment.replace(['\r', '\n'], " ");
    if !comment.trim().is_empty() {
        out.push(' ');
        out.push_str(comment.trim());
    }
    out.push('\n');
    for chunk in seq.bases.chunks(LINE_WIDTH) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or("?"));
        out.push('\n');
    }
    out
}

/// Formats several sequences as multi-FASTA (Edit ▸ Copy All Sequences To
/// Multifasta), for CLUSTAL and friends.
pub fn write_multi<'a>(seqs: impl IntoIterator<Item = &'a Sequence>) -> String {
    seqs.into_iter().map(write).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");

    #[test]
    fn headers_in_both_serial_cloner_forms_and_generic() {
        assert_eq!(
            parse_header(
                " [pBluescript II SK(+) - 2961 bp] gi|58063|emb|X52328|ARBL2SKP pBluescript II SK(+) vector DNA"
            ),
            (
                "pBluescript II SK(+)".into(),
                "gi|58063|emb|X52328|ARBL2SKP pBluescript II SK(+) vector DNA".into()
            )
        );
        assert_eq!(
            parse_header(" [pBR322] LOCUS       SYNPBR322    4361 bp - 4361 bp"),
            ("pBR322".into(), "LOCUS       SYNPBR322    4361 bp".into())
        );
        assert_eq!(
            parse_header("pEGFP-N1 Sequence - 4733 bp"),
            ("pEGFP-N1".into(), "Sequence - 4733 bp".into())
        );
        assert_eq!(
            parse_header("NM_000001.1"),
            ("NM_000001.1".into(), String::new())
        );
    }

    #[test]
    fn shipped_fasta_files_match_their_xdna_twins() {
        for (fasta, xdna) in [
            ("pBR322.fasta", "pBR322.xdna"),
            ("pBluescript-II-KS+.fasta", "pBluescript-II-KS+.xdna"),
            ("pEGFP-N1.fasta", "pEGFP-N1.xdna"),
        ] {
            let seqs = read(&std::fs::read(Path::new(FIXTURES).join(fasta)).unwrap());
            assert_eq!(seqs.len(), 1, "{fasta}");
            let x = crate::xdna::read(&std::fs::read(Path::new(FIXTURES).join(xdna)).unwrap(), "x")
                .unwrap();
            assert_eq!(
                seqs[0].bases, x.bases,
                "{fasta}: bases (case included) differ from the xdna"
            );
            assert_eq!(seqs[0].seq_type, SeqType::Dna);
        }
        let pbr = &read(&std::fs::read(Path::new(FIXTURES).join("pBR322.fasta")).unwrap())[0];
        assert_eq!(pbr.name, "pBR322");
    }

    #[test]
    fn multi_fasta_and_round_trip() {
        let mut a = Sequence::new("a b", vec![b'A'; 100]);
        a.comment = "first\rline two".into();
        let b = Sequence::new("b", b"acgtn".to_vec());
        let text = write_multi([&a, &b]);
        assert!(text.starts_with("> [a b - 100 bp] first line two\n"));
        assert_eq!(text.lines().nth(1).unwrap().len(), 80);
        let back = read(text.as_bytes());
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].name, "a b");
        assert_eq!(back[0].comment, "first line two");
        assert_eq!(back[0].bases, a.bases);
        assert_eq!(back[1].bases, b"acgtn");
        assert_eq!(back[1].seq_type, SeqType::DegenerateDna);
    }

    #[test]
    fn headerless_text_is_one_record() {
        let seqs = read(b"ACGT\nACGT\n");
        assert_eq!(seqs.len(), 1);
        assert_eq!(seqs[0].bases, b"ACGTACGT");
    }
}
