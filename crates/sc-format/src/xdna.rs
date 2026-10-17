//! The native format: DNA Strider `.xdna`, extended by Serial Cloner.
//!
//! Layout, verified byte for byte against the seven files in
//! `reference/sequences/` (`docs/file-formats.md`):
//!
//! ```text
//! offset size  meaning
//! 0      1     version, always 0
//! 1      1     sequence type (see `SeqType::from_xdna_byte`)
//! 2      1     topology (0 linear, 1 circular)
//! 3      25    DNA Strider fields we do not interpret (timestamps, window
//!              geometry, selection). Zero in Serial Cloner files, populated
//!              in genuine Strider files. Preserved verbatim.
//! 28     4     u32 big-endian sequence length
//! 32     64    more Strider fields, preserved verbatim
//! 96     4     u32 big-endian comment length
//! 100    12    reserved; Serial Cloner writes 0xFF at offset 111 (Strider
//!              writes it at offset 92 instead)
//! 112    n     sequence, ASCII, case preserved
//! 112+n  c     comment, ISO-8859-1, lines separated by 0x0D
//! ```
//!
//! A genuine Strider file ends there. A Serial Cloner file continues with the
//! annotation block; all strings are Pascal strings (`u8` length + bytes):
//!
//! ```text
//! u8   ends flag       0 blunt, 5 left end cohesive, 3 right end cohesive,
//!                      35 both (the digits "5"/"3" read as a number)
//! pstr right-overhang length as decimal text (+ = 5', − = 3'), then the
//!                      overhang bases (abs(length) raw bytes, 5'→3' on the
//!                      strand that carries them) if non-zero
//! pstr left-overhang   same
//! u8   feature count   the original always writes it (0 when none); it is
//!                      tolerated absent
//! per feature:
//!   pstr name
//!   pstr description   qualifiers separated by 0x0D
//!   pstr kind          GenBank key, e.g. "rep_origin"
//!   pstr start         1-based decimal, first base in feature direction
//!   pstr end           last base in feature direction
//!   u8×4 flags         strand (1 = forward), show, protect, arrow
//!   pstr colour        "r,g,b," decimal with trailing comma
//! ```
//!
//! The cohesive-end encoding is attested by files the original wrote
//! (`reference/original/golden/ends/`, see
//! `reference/docs/original-behaviour.md`); [`read`]'s tests check them.

use sc_seq::{
    Ends, Feature, Overhang, Rgb, SeqType, Sequence, Source, Strand, Topology, XDNA_HEADER_LEN,
};

use crate::{Error, Result, decode_latin1, encode_latin1};

const LEN_OFFSET: usize = 28;
const COMMENT_LEN_OFFSET: usize = 96;
const SC_MARKER_OFFSET: usize = 111;

/// Cheap sniff: could these bytes be an `.xdna` file?
pub fn looks_like(bytes: &[u8]) -> bool {
    if bytes.len() < XDNA_HEADER_LEN || bytes[0] != 0 || bytes[1] > 4 || bytes[2] > 1 {
        return false;
    }
    let n = u32_at(bytes, LEN_OFFSET) as usize;
    let c = u32_at(bytes, COMMENT_LEN_OFFSET) as usize;
    XDNA_HEADER_LEN + n + c <= bytes.len()
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Sequential reader over the annotation block.
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn take(&mut self, n: usize, what: &str) -> Result<&'a [u8]> {
        if self.remaining() < n {
            return Err(Error::Malformed(format!(
                "xdna: truncated {what} at byte {}",
                self.pos
            )));
        }
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self, what: &str) -> Result<u8> {
        Ok(self.take(1, what)?[0])
    }

    fn pstr(&mut self, what: &str) -> Result<String> {
        let len = self.u8(what)? as usize;
        Ok(decode_latin1(self.take(len, what)?))
    }

    fn overhang(&mut self, what: &str) -> Result<Overhang> {
        let text = self.pstr(what)?;
        let length: i32 = text
            .trim()
            .parse()
            .map_err(|_| Error::Malformed(format!("xdna: bad {what} overhang length {text:?}")))?;
        let bases = if length == 0 {
            Vec::new()
        } else {
            self.take(length.unsigned_abs() as usize, what)?.to_vec()
        };
        Ok(Overhang { length, bases })
    }

    fn feature(&mut self) -> Result<Feature> {
        let name = self.pstr("feature name")?;
        let description = self.pstr("feature description")?;
        let kind = self.pstr("feature type")?;
        let start = parse_u32(&self.pstr("feature start")?, "start")?;
        let end = parse_u32(&self.pstr("feature end")?, "end")?;
        let flags = self.take(4, "feature flags")?;
        let color = parse_rgb(&self.pstr("feature colour")?)?;
        Ok(Feature {
            name,
            description,
            kind,
            start,
            end,
            strand: if flags[0] == 1 {
                Strand::Forward
            } else {
                Strand::Reverse
            },
            show: flags[1] != 0,
            protect: flags[2] != 0,
            arrow: flags[3] != 0,
            color,
        })
    }
}

fn parse_u32(text: &str, what: &str) -> Result<u32> {
    text.trim()
        .parse()
        .map_err(|_| Error::Malformed(format!("xdna: bad feature {what} {text:?}")))
}

fn parse_rgb(text: &str) -> Result<Rgb> {
    let parts: Vec<u8> = text
        .split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().parse::<u8>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| Error::Malformed(format!("xdna: bad colour {text:?}")))?;
    match parts[..] {
        [r, g, b] => Ok(Rgb(r, g, b)),
        _ => Err(Error::Malformed(format!("xdna: bad colour {text:?}"))),
    }
}

/// Parses an `.xdna` file. `name` is the file stem (the format has no name).
pub fn read(bytes: &[u8], name: &str) -> Result<Sequence> {
    if bytes.len() < XDNA_HEADER_LEN {
        return Err(Error::Malformed(
            "xdna: shorter than the 112-byte header".into(),
        ));
    }
    if bytes[0] != 0 {
        return Err(Error::Malformed(format!(
            "xdna: unknown version {}",
            bytes[0]
        )));
    }
    let header = &bytes[..XDNA_HEADER_LEN];
    let seq_type = SeqType::from_xdna_byte(header[1]).unwrap_or(SeqType::Unknown);
    let topology = Topology::from_xdna_byte(header[2]).unwrap_or(Topology::Linear);
    let n = u32_at(header, LEN_OFFSET) as usize;
    let c = u32_at(header, COMMENT_LEN_OFFSET) as usize;
    if XDNA_HEADER_LEN + n + c > bytes.len() {
        return Err(Error::Malformed(format!(
            "xdna: header announces {n} bases and {c} comment bytes but the file has only {} bytes",
            bytes.len()
        )));
    }
    let bases = bytes[XDNA_HEADER_LEN..XDNA_HEADER_LEN + n].to_vec();
    let comment = decode_latin1(&bytes[XDNA_HEADER_LEN + n..XDNA_HEADER_LEN + n + c]);
    let mut seq = Sequence {
        name: name.to_owned(),
        bases,
        seq_type,
        topology,
        comment,
        features: Vec::new(),
        ends: Ends::default(),
        source: Source::Strider,
        strider_header: Some(header.to_vec()),
        snapgene_extras: None,
    };

    let rest = &bytes[XDNA_HEADER_LEN + n + c..];
    if rest.is_empty() {
        return Ok(seq);
    }
    let mut cur = Cursor {
        bytes: rest,
        pos: 0,
    };
    // The ends flag: 0 blunt, 5 left end cohesive, 3 right end cohesive,
    // 35 both. The overhangs that follow carry the same information, so the
    // flag is only validated.
    let flag = cur.u8("ends flag")?;
    if !matches!(flag, 0 | 3 | 5 | 35) {
        return Err(Error::Malformed(format!(
            "xdna: annotation block starts with ends flag {flag}, not 0, 3, 5 or 35"
        )));
    }
    seq.source = Source::SerialCloner;
    seq.ends.right = cur.overhang("right")?;
    seq.ends.left = cur.overhang("left")?;
    if cur.remaining() > 0 {
        let count = cur.u8("feature count")?;
        for _ in 0..count {
            let f = cur.feature()?;
            seq.features.push(f);
        }
    }
    Ok(seq)
}

fn pstr(out: &mut Vec<u8>, text: &str) {
    let bytes = encode_latin1(text);
    let len = bytes.len().min(255);
    out.push(len as u8);
    out.extend_from_slice(&bytes[..len]);
}

fn write_overhang(out: &mut Vec<u8>, o: &Overhang) {
    pstr(out, &o.length.to_string());
    if o.length != 0 {
        let n = o.length.unsigned_abs() as usize;
        let mut bases = o.bases.clone();
        bases.resize(n, b'N');
        out.extend_from_slice(&bases);
    }
}

/// Serialises a sequence as `.xdna`.
///
/// The header comes from [`Sequence::strider_header`] when present (so the
/// Strider fields survive) and is otherwise the one Serial Cloner writes:
/// zeros with 0xFF at offset 111. A sequence read from a plain Strider file
/// that has gained no features or cohesive ends is written back as plain
/// Strider, which is what makes the shipped `pBluescript II KS(+).xdna`
/// round-trip byte for byte; anything else gets the Serial Cloner block.
pub fn write(seq: &Sequence) -> Vec<u8> {
    let mut header = match &seq.strider_header {
        Some(h) if h.len() == XDNA_HEADER_LEN => h.clone(),
        _ => {
            let mut h = vec![0u8; XDNA_HEADER_LEN];
            h[SC_MARKER_OFFSET] = 0xff;
            h
        }
    };
    let comment = encode_latin1(&seq.comment);
    header[0] = 0;
    header[1] = seq.seq_type.to_xdna_byte();
    header[2] = seq.topology.to_xdna_byte();
    header[LEN_OFFSET..LEN_OFFSET + 4].copy_from_slice(&(seq.bases.len() as u32).to_be_bytes());
    header[COMMENT_LEN_OFFSET..COMMENT_LEN_OFFSET + 4]
        .copy_from_slice(&(comment.len() as u32).to_be_bytes());

    let mut out = header;
    out.extend_from_slice(&seq.bases);
    out.extend_from_slice(&comment);

    let plain_strider =
        seq.source == Source::Strider && seq.features.is_empty() && seq.ends.is_blunt();
    if plain_strider {
        return out;
    }
    out.push(crate::genbank::ends_flag(&seq.ends));
    write_overhang(&mut out, &seq.ends.right);
    write_overhang(&mut out, &seq.ends.left);
    // 2.6.1 always writes the feature count; older files (the shipped
    // pBR322.xdna) stop after the overhangs when there are no features, and
    // that is what keeps them byte-for-byte on a round trip.
    if !seq.features.is_empty() || !seq.ends.is_blunt() {
        let count = seq.features.len().min(255);
        out.push(count as u8);
        for f in &seq.features[..count] {
            pstr(&mut out, &f.name);
            pstr(&mut out, &f.description);
            pstr(&mut out, &f.kind);
            pstr(&mut out, &f.start.to_string());
            pstr(&mut out, &f.end.to_string());
            out.extend_from_slice(&[
                u8::from(f.strand == Strand::Forward),
                u8::from(f.show),
                u8::from(f.protect),
                u8::from(f.arrow),
            ]);
            pstr(
                &mut out,
                &format!("{},{},{},", f.color.0, f.color.1, f.color.2),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(Path::new(FIXTURES).join(name)).unwrap()
    }

    #[test]
    fn every_shipped_xdna_round_trips_byte_for_byte() {
        let mut seen = 0;
        for entry in std::fs::read_dir(FIXTURES).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("xdna") {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            assert!(looks_like(&bytes), "{}", path.display());
            let seq = read(&bytes, "x").unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(write(&seq), bytes, "{} does not round-trip", path.display());
            seen += 1;
        }
        assert_eq!(seen, 7);
    }

    #[test]
    fn pbr322_annotated_matches_the_plan_worked_example() {
        let seq = read(&fixture("pBR322-annotated.xdna"), "pBR322").unwrap();
        assert_eq!(seq.len(), 4361);
        assert_eq!(seq.topology, Topology::Circular);
        assert_eq!(seq.seq_type, SeqType::Dna);
        assert_eq!(seq.source, Source::SerialCloner);
        assert!(seq.ends.is_blunt());
        assert!(seq.comment.starts_with("LOCUS       SYNPBR322    4361 bp"));
        assert_eq!(seq.comment.matches('\r').count(), 2);
        assert_eq!(seq.features.len(), 3);
        let ori = &seq.features[0];
        assert_eq!(ori.name, "ColE1 origin");
        assert_eq!(ori.kind, "rep_origin");
        assert_eq!((ori.start, ori.end), (3144, 2516));
        assert_eq!(ori.strand, Strand::Reverse);
        assert_eq!((ori.show, ori.protect, ori.arrow), (true, false, true));
        assert_eq!(ori.color, Rgb(70, 130, 180));
        let tet = &seq.features[2];
        assert_eq!(
            (tet.name.as_str(), tet.start, tet.end, tet.strand),
            ("TetR", 89, 1273, Strand::Forward)
        );
        // Lower case is data and must survive.
        assert!(seq.bases.starts_with(b"ttctcatgtttgacagcttatcatcgataa"));
    }

    #[test]
    fn strider_and_serial_cloner_headers_are_told_apart() {
        let strider = read(&fixture("pBluescript-II-KS+.xdna"), "x").unwrap();
        assert_eq!(strider.source, Source::Strider);
        assert_eq!(strider.strider_header.as_ref().unwrap()[92], 0xff);
        assert_eq!(strider.strider_header.as_ref().unwrap()[3], 1);
        let sc = read(&fixture("pBluescript-II-KS+-annotated.xdna"), "x").unwrap();
        assert_eq!(sc.source, Source::SerialCloner);
        assert_eq!(sc.strider_header.as_ref().unwrap()[111], 0xff);
        assert_eq!(sc.features.len(), 9);
        assert_eq!(sc.bases, strider.bases);
        assert_eq!(sc.comment, strider.comment);
    }

    #[test]
    fn pegfp_manual_feature_is_protected_and_wraps_the_origin() {
        let seq = read(&fixture("pEGFP-N1.xdna"), "pEGFP-N1").unwrap();
        assert_eq!(seq.comment, "pEGFP-N1 Sequence");
        let f = &seq.features[0];
        assert_eq!(f.name, "New Feature");
        assert_eq!((f.start, f.end, f.strand), (4000, 200, Strand::Forward));
        assert!(f.protect);
        assert_eq!(f.lo_hi(), (200, 4000, true));
    }

    #[test]
    fn files_without_features_have_no_count_byte() {
        // pDEST14: annotation block is exactly 00 01 '0' 01 '0'.
        let bytes = fixture("pDEST14.xdna");
        assert_eq!(&bytes[bytes.len() - 5..], &[0, 1, b'0', 1, b'0']);
        let seq = read(&bytes, "x").unwrap();
        assert!(seq.features.is_empty());
        assert_eq!(seq.comment, "");
        assert_eq!(seq.source, Source::SerialCloner);
    }

    #[test]
    fn new_sequences_get_the_serial_cloner_header() {
        let mut seq = Sequence::new("new", b"ACGTacgt".to_vec());
        seq.topology = Topology::Circular;
        seq.comment = "hello\rworld".into();
        seq.features.push(Feature {
            name: "f".into(),
            description: "note=x\rgene=y".into(),
            kind: "CDS".into(),
            start: 2,
            end: 5,
            strand: Strand::Forward,
            show: true,
            protect: true,
            arrow: false,
            color: Rgb(1, 2, 3),
        });
        let bytes = write(&seq);
        assert_eq!(bytes[111], 0xff);
        assert_eq!(&bytes[0..3], &[0, 1, 1]);
        let back = read(&bytes, "new").unwrap();
        assert_eq!(back.bases, seq.bases);
        assert_eq!(back.comment, seq.comment);
        assert_eq!(back.features, seq.features);
        assert_eq!(back.source, Source::SerialCloner);
        assert_eq!(write(&back), bytes);
    }

    #[test]
    fn cohesive_ends_round_trip_under_the_attested_encoding() {
        let mut seq = Sequence::new("frag", b"GATCCAAAG".to_vec());
        seq.ends.left = Overhang::five_prime(b"GATC");
        seq.ends.right = Overhang::three_prime(b"AG");
        let bytes = write(&seq);
        let tail = &bytes[bytes.len() - (1 + 3 + 2 + 2 + 4 + 1)..];
        assert_eq!(tail, b"\x23\x02-2AG\x014GATC\0");
        let back = read(&bytes, "frag").unwrap();
        assert_eq!(back.ends, seq.ends);
    }

    /// Files the original wrote after the four single-end combinations were
    /// typed into Sequence Window ▸ Extremities (see
    /// `reference/docs/original-behaviour.md`).
    #[test]
    fn original_cohesive_end_files_are_read_and_written_back_byte_for_byte() {
        let golden =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/original/golden/ends");
        let cases: [(&str, Overhang, Overhang, u8); 6] = [
            (
                "lin-L5.xdna",
                Overhang::five_prime(b"GATC"),
                Overhang::BLUNT,
                5,
            ),
            (
                "lin-L3.xdna",
                Overhang::three_prime(b"CTAG"),
                Overhang::BLUNT,
                5,
            ),
            (
                "lin-L5-3nt.xdna",
                Overhang::five_prime(b"GAT"),
                Overhang::BLUNT,
                5,
            ),
            (
                "lin-R5.xdna",
                Overhang::BLUNT,
                Overhang::five_prime(b"TCGA"),
                3,
            ),
            (
                "lin-R3.xdna",
                Overhang::BLUNT,
                Overhang::three_prime(b"AGCT"),
                3,
            ),
            (
                "pBR322-lin-L5-R5.xdna",
                Overhang::five_prime(b"GATC"),
                Overhang::five_prime(b"TCGA"),
                35,
            ),
        ];
        for (name, left, right, flag) in cases {
            let bytes = std::fs::read(golden.join(name)).unwrap();
            let seq = read(&bytes, name).unwrap();
            assert_eq!(seq.len(), 4374, "{name}");
            assert_eq!(seq.topology, Topology::Linear, "{name}");
            assert_eq!(seq.ends.left, left, "{name}");
            assert_eq!(seq.ends.right, right, "{name}");
            assert_eq!(crate::genbank::ends_flag(&seq.ends), flag, "{name}");
            assert_eq!(write(&seq), bytes, "{name}");
        }
    }

    #[test]
    fn truncated_files_are_rejected_not_panicked() {
        let bytes = fixture("pBR322-annotated.xdna");
        assert!(read(&bytes[..100], "x").is_err());
        assert!(read(&bytes[..bytes.len() - 10], "x").is_err());
        let mut short = bytes.clone();
        short[31] = 0xff; // impossible length
        assert!(read(&short, "x").is_err());
        assert!(!looks_like(&short));
    }
}
