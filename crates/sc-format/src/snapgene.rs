//! SnapGene `.dna` files (GSL Biotech): read and write.
//!
//! The format is undocumented; this follows what Biopython's `SnapGeneIO`
//! and the `sgffp` project reverse-engineered, checked against the 24 files
//! in `reference/sequences/snapgene/`. A file is a sequence of packets,
//! each `u8 type, u32 big-endian length, payload`:
//!
//! | type | content | here |
//! |---|---|---|
//! | 9 | header: `SnapGene`, `u16` molecule (1 DNA, 2 protein, 7 RNA), `u16` export version, `u16` import version | read, kept |
//! | 0 | DNA: a flags byte (bit 0 circular, bit 1 double-stranded, bits 2–4 Dam/Dcm/EcoKI methylation) then the bases, case preserved | read, written |
//! | 21 / 32 | protein / RNA: same layout as 0 | read, written |
//! | 1 | compressed DNA (2-bit / IUPAC-nibble / N-run sections, lower-case ranges); only seen inside history nodes, but decoded when a file has no type 0 | read |
//! | 10 | features, XML: `<Feature name type directionality><Segment range="a-b" color type/>…<Q name><V text|int|predef/></Q></Feature>`; `directionality` 0 none, 1 forward, 2 reverse, 3 both; a range with `a > b` wraps the origin; `type="gap"` segments are not part of the feature | read, written |
//! | 5 | primers, XML: `<Primer name sequence><BindingSite location="A-B" boundStrand annealedBases meltingTemperature simplified/>` with 0-based locations and `<HybridizationParams>` thresholds | read as `primer_bind` features, kept |
//! | 6 | notes, XML: `Description`, `Comments`, `CustomMapLabel`, dates… (text carries `<html>` markup) | `Description` ↔ comment, `CustomMapLabel` → name; kept |
//! | 8 | additional properties (end stickiness / modifications) | kept |
//! | 2, 3, 13, 35 | enzyme caches; 7, 11, 29, 30 history; 16–18, 27 traces; 14, 20, 23, 28 preferences and attachments | kept verbatim |
//!
//! **Round trip.** Every packet of a file that was read is kept in
//! [`Sequence::snapgene_extras`]. [`write`] then reproduces the file byte for
//! byte when nothing was edited, and otherwise rewrites only the packets that
//! carry what changed (sequence, features, comment) — primers, history,
//! traces and notes survive. A sequence that did not come from SnapGene is
//! written as a minimal file: header, sequence, properties, features,
//! primers, notes.
//!
//! What the fixtures showed: every file starts with the 19-byte header; the
//! top-level sequence is always type 0 / 21 / 32 (type 1 appears only inside
//! history); features on the reverse strand carry `swappedSegmentNumbering`
//! and `readingFrame="-1"`; feature and note texts are HTML fragments
//! (`&lt;html>&lt;body>…`); primer binding-site locations are 0-based while
//! feature ranges are 1-based; a 10-bp sequence with `range="3-2"` is a
//! feature wrapping the whole circle.

use std::time::{SystemTime, UNIX_EPOCH};

use roxmltree::{Document, Node};
use sc_seq::alphabet::is_acgt;
use sc_seq::{Ends, Feature, Rgb, SeqType, Sequence, Source, Strand, Topology};

use crate::{Error, Result};

/// The ASCII cookie every file starts with (inside the header packet).
pub const COOKIE: &[u8] = b"SnapGene";
const HEADER: u8 = 9;
const DNA: u8 = 0;
const COMPRESSED: u8 = 1;
const PRIMERS: u8 = 5;
const NOTES: u8 = 6;
const PROPERTIES: u8 = 8;
const FEATURES: u8 = 10;
const PROTEIN: u8 = 21;
const RNA: u8 = 32;
/// Header versions written for new files (SnapGene 7/8 era).
const EXPORT_VERSION: u16 = 15;
const IMPORT_VERSION: u16 = 20;
/// Description line that marks a feature made from a SnapGene primer.
const PRIMER_MARK: &str = "snapgene_primer=";
/// SnapGene's default feature colour.
const DEFAULT_COLOR: Rgb = Rgb(0xa6, 0xac, 0xb3);

/// Cheap sniff: could these bytes be a SnapGene file?
pub fn looks_like(bytes: &[u8]) -> bool {
    bytes.len() >= 19
        && bytes[0] == HEADER
        && bytes[1..5] == [0, 0, 0, 14]
        && &bytes[5..13] == COOKIE
}

/// Splits the file into `(type, payload)` packets.
fn packets(bytes: &[u8]) -> Result<Vec<(u8, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if i + 5 > bytes.len() {
            return Err(Error::Malformed(format!(
                "snapgene: truncated packet header at byte {i}"
            )));
        }
        let t = bytes[i];
        let n =
            u32::from_be_bytes([bytes[i + 1], bytes[i + 2], bytes[i + 3], bytes[i + 4]]) as usize;
        let start = i + 5;
        let end = start.checked_add(n).filter(|&e| e <= bytes.len()).ok_or_else(|| {
            Error::Malformed(format!("snapgene: packet of type {t} at byte {i} announces {n} bytes past the end of the file"))
        })?;
        out.push((t, bytes[start..end].to_vec()));
        i = end;
    }
    Ok(out)
}

fn packet(out: &mut Vec<u8>, t: u8, payload: &[u8]) {
    out.push(t);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
}

// ------------------------------------------------------------------ XML --

/// Drops the HTML markup SnapGene wraps texts in (`<html><body>x</body></html>`
/// → `x`) and decodes the entities left inside it.
fn strip_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
        .trim()
        .to_owned()
}

/// The inverse: a plain text as the HTML fragment SnapGene stores.
fn as_html(text: &str) -> String {
    let inner = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("<html><body>{inner}</body></html>")
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\r' | '\n' => out.push(' '),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

fn parse_xml(data: &[u8], what: &str) -> Result<String> {
    String::from_utf8(data.to_vec())
        .map_err(|_| Error::Malformed(format!("snapgene: {what} packet is not UTF-8")))
}

fn child_text<'a>(node: Node<'a, 'a>, name: &str) -> Option<String> {
    node.children()
        .find(|c| c.has_tag_name(name))
        .and_then(|c| c.text())
        .map(|t| t.to_owned())
}

/// `"a-b"` → `(a, b)`.
fn parse_range(text: &str) -> Option<(u32, u32)> {
    let (a, b) = text.split_once('-')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn parse_color(text: &str) -> Option<Rgb> {
    let h = text.trim().strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

// ------------------------------------------------------------- reading --

/// Decodes a compressed-DNA packet (type 1).
fn decode_compressed(data: &[u8]) -> Result<Vec<u8>> {
    let u32_at = |i: usize| -> Result<u32> {
        data.get(i..i + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| Error::Malformed("snapgene: truncated compressed DNA".into()))
    };
    if data.len() < 22 {
        return Err(Error::Malformed(
            "snapgene: compressed DNA packet too short".into(),
        ));
    }
    let total = u32_at(4)? as usize;
    let chunks = u32_at(9)? as usize;
    let lowercase = u32_at(13)? as usize;
    let mut bases = Vec::with_capacity(total);
    let mut pos = 17;
    for _ in 0..chunks {
        let marker = *data
            .get(pos)
            .ok_or_else(|| Error::Malformed("snapgene: truncated compressed section".into()))?;
        let count = u32_at(pos + 1)? as usize;
        pos += 5;
        match marker {
            0x01 => {
                let nb = count.div_ceil(4);
                let raw = data
                    .get(pos..pos + nb)
                    .ok_or_else(|| Error::Malformed("snapgene: truncated 2-bit section".into()))?;
                for i in 0..count {
                    let byte = raw[i / 4];
                    let shift = 6 - 2 * (i % 4);
                    bases.push(b"GATC"[((byte >> shift) & 3) as usize]);
                }
                pos += nb;
            }
            0x02 => {
                const CODES: [u8; 16] = *b"NNNNNBDHKMRSVWYN";
                let nb = count.div_ceil(2);
                let raw = data
                    .get(pos..pos + nb)
                    .ok_or_else(|| Error::Malformed("snapgene: truncated IUPAC section".into()))?;
                for i in 0..count {
                    let nib = if i % 2 == 0 {
                        raw[i / 2] >> 4
                    } else {
                        raw[i / 2] & 0x0f
                    };
                    bases.push(CODES[nib as usize]);
                }
                pos += nb;
            }
            0x03 => bases.extend(std::iter::repeat_n(b'N', count)),
            m => {
                return Err(Error::Malformed(format!(
                    "snapgene: unknown compressed section marker {m:#x}"
                )));
            }
        }
    }
    for _ in 0..lowercase {
        let (a, b) = (u32_at(pos)? as usize, u32_at(pos + 4)? as usize);
        pos += 8;
        for base in bases.iter_mut().take(b + 1).skip(a) {
            base.make_ascii_lowercase();
        }
    }
    Ok(bases)
}

/// The features of a type-10 packet.
fn read_features(data: &[u8], len: u32) -> Result<Vec<Feature>> {
    let text = parse_xml(data, "features")?;
    let doc = Document::parse(&text)
        .map_err(|e| Error::Malformed(format!("snapgene: features XML: {e}")))?;
    let mut out = Vec::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("Feature")) {
        let name = node.attribute("name").unwrap_or("").to_owned();
        let kind = node
            .attribute("type")
            .filter(|t| !t.is_empty())
            .unwrap_or("misc_feature")
            .to_owned();
        let directionality: u8 = node
            .attribute("directionality")
            .and_then(|d| d.parse().ok())
            .unwrap_or(0);
        let segments: Vec<Node> = node
            .children()
            .filter(|c| c.has_tag_name("Segment") && c.attribute("type") != Some("gap"))
            .collect();
        let ranges: Vec<(u32, u32)> = segments
            .iter()
            .filter_map(|s| s.attribute("range"))
            .filter_map(parse_range)
            .collect();
        if ranges.is_empty() {
            continue;
        }
        let color = segments
            .iter()
            .find_map(|s| s.attribute("color"))
            .and_then(parse_color)
            .unwrap_or(DEFAULT_COLOR);
        // Span on the top strand: (a, b) with a > b meaning "wraps the origin".
        let (a, b) = if ranges.len() == 1 {
            ranges[0]
        } else if let Some(w) = ranges.iter().find(|(a, b)| a > b) {
            (
                w.0,
                ranges
                    .iter()
                    .filter(|(a, b)| a <= b)
                    .map(|r| r.1)
                    .max()
                    .unwrap_or(w.1)
                    .max(w.1),
            )
        } else {
            (
                ranges.iter().map(|r| r.0).min().unwrap(),
                ranges.iter().map(|r| r.1).max().unwrap(),
            )
        };
        let (a, b) = (a.clamp(1, len.max(1)), b.clamp(1, len.max(1)));
        let strand = if directionality == 2 {
            Strand::Reverse
        } else {
            Strand::Forward
        };
        let (start, end) = match strand {
            Strand::Forward => (a, b),
            Strand::Reverse => (b, a),
        };
        let mut lines: Vec<String> = Vec::new();
        for q in node.children().filter(|c| c.has_tag_name("Q")) {
            let Some(qname) = q.attribute("name") else {
                continue;
            };
            for v in q.children().filter(|c| c.has_tag_name("V")) {
                let value = v
                    .attribute("text")
                    .map(strip_html)
                    .or_else(|| v.attribute("int").map(str::to_owned))
                    .or_else(|| v.attribute("predef").map(str::to_owned));
                if let Some(value) = value {
                    if qname == "label" && value == name {
                        continue;
                    }
                    lines.push(format!("{qname}={value}"));
                }
            }
        }
        if ranges.len() > 1 {
            lines.push(format!(
                "segments={}",
                ranges
                    .iter()
                    .map(|(a, b)| format!("{a}-{b}"))
                    .collect::<Vec<_>>()
                    .join(";")
            ));
        }
        out.push(Feature {
            name,
            description: lines.join("\r"),
            kind,
            start,
            end,
            strand,
            show: true,
            protect: false,
            arrow: matches!(directionality, 1 | 2),
            color,
        });
    }
    Ok(out)
}

/// The primers of a type-5 packet, as `primer_bind` features (one per
/// binding site that SnapGene would show, following Biopython's reading of
/// the hybridization parameters).
fn read_primers(data: &[u8], len: u32) -> Result<Vec<Feature>> {
    let text = parse_xml(data, "primers")?;
    let doc = Document::parse(&text)
        .map_err(|e| Error::Malformed(format!("snapgene: primers XML: {e}")))?;
    let params = doc
        .descendants()
        .find(|n| n.has_tag_name("HybridizationParams"));
    let min_len: usize = params
        .and_then(|p| p.attribute("minContinuousMatchLen"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let min_tm: i64 = params
        .and_then(|p| p.attribute("minMeltingTemperature"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut out = Vec::new();
    for primer in doc.descendants().filter(|n| n.has_tag_name("Primer")) {
        let name = primer.attribute("name").unwrap_or("primer").to_owned();
        let sequence = primer.attribute("sequence").unwrap_or("").to_owned();
        let mut seen: Vec<(u32, u32, bool)> = Vec::new();
        for site in primer.children().filter(|c| c.has_tag_name("BindingSite")) {
            let Some((a, b)) = site.attribute("location").and_then(parse_range) else {
                continue;
            };
            let reverse = site.attribute("boundStrand") == Some("1");
            if site.attribute("simplified") == Some("1") && seen.contains(&(a, b, reverse)) {
                continue;
            }
            if let Some(annealed) = site.attribute("annealedBases")
                && annealed.len() < min_len
            {
                continue;
            }
            if let Some(tm) = site
                .attribute("meltingTemperature")
                .and_then(|t| t.parse::<i64>().ok())
                && tm < min_tm
            {
                continue;
            }
            seen.push((a, b, reverse));
            // Primer locations are 0-based (Biopython, #5053).
            let (lo, hi) = ((a + 1).clamp(1, len.max(1)), (b + 1).clamp(1, len.max(1)));
            let (start, end) = if reverse { (hi, lo) } else { (lo, hi) };
            out.push(Feature {
                name: name.clone(),
                description: format!("{PRIMER_MARK}{sequence}"),
                kind: "primer_bind".into(),
                start,
                end,
                strand: if reverse {
                    Strand::Reverse
                } else {
                    Strand::Forward
                },
                show: true,
                protect: false,
                arrow: true,
                color: Rgb(0, 139, 139),
            });
        }
    }
    Ok(out)
}

/// What a notes packet contributes: the comment and, when set, the name.
fn read_notes(data: &[u8]) -> Result<(String, Option<String>)> {
    let text = parse_xml(data, "notes")?;
    let doc = Document::parse(&text)
        .map_err(|e| Error::Malformed(format!("snapgene: notes XML: {e}")))?;
    let root = doc.root_element();
    let description = child_text(root, "Description")
        .map(|t| strip_html(&t))
        .unwrap_or_default();
    let comments = child_text(root, "Comments")
        .map(|t| strip_html(&t))
        .unwrap_or_default();
    let comment = if comments.is_empty() || comments == description {
        description
    } else if description.is_empty() {
        comments
    } else {
        format!("{description}\r{comments}")
    };
    let label = child_text(root, "CustomMapLabel")
        .map(|t| strip_html(&t))
        .filter(|t| !t.is_empty());
    Ok((comment, label))
}

/// Features and primers of a packet list, in file order (features first).
fn features_of(packets: &[(u8, Vec<u8>)], len: u32) -> Result<Vec<Feature>> {
    let mut out = Vec::new();
    for (t, data) in packets {
        if *t == FEATURES {
            out.extend(read_features(data, len)?);
        }
    }
    for (t, data) in packets {
        if *t == PRIMERS {
            out.extend(read_primers(data, len)?);
        }
    }
    Ok(out)
}

fn comment_of(packets: &[(u8, Vec<u8>)]) -> Result<(String, Option<String>)> {
    for (t, data) in packets {
        if *t == NOTES {
            return read_notes(data);
        }
    }
    Ok((String::new(), None))
}

/// Parses a `.dna` file. `name` is the file stem, used unless the notes
/// carry a custom map label.
pub fn read(bytes: &[u8], name: &str) -> Result<Sequence> {
    if !looks_like(bytes) {
        return Err(Error::Malformed(
            "snapgene: missing the SnapGene cookie".into(),
        ));
    }
    let packets = packets(bytes)?;
    let header = &packets[0].1;
    let molecule = u16::from_be_bytes([header[8], header[9]]);
    let (flags, bases): (u8, Vec<u8>) = match packets
        .iter()
        .find(|(t, _)| matches!(*t, DNA | PROTEIN | RNA))
    {
        Some((_, data)) if !data.is_empty() => (data[0], data[1..].to_vec()),
        _ => match packets.iter().find(|(t, _)| *t == COMPRESSED) {
            Some((_, data)) => (0, decode_compressed(data)?),
            None => return Err(Error::Malformed("snapgene: no sequence packet".into())),
        },
    };
    let len = bases.len() as u32;
    let seq_type = match molecule {
        2 => SeqType::Protein,
        7 => SeqType::Rna,
        _ if bases.iter().all(|&b| is_acgt(b)) => SeqType::Dna,
        _ => SeqType::DegenerateDna,
    };
    let features = features_of(&packets, len)?;
    let (comment, label) = comment_of(&packets)?;
    Ok(Sequence {
        name: label.unwrap_or_else(|| name.to_owned()),
        bases,
        seq_type,
        topology: if flags & 1 == 1 {
            Topology::Circular
        } else {
            Topology::Linear
        },
        comment,
        features,
        ends: Ends::default(),
        source: Source::SnapGene,
        strider_header: None,
        snapgene_extras: Some(packets),
    })
}

// ------------------------------------------------------------- writing --

/// Today as `YYYY.M.D` (SnapGene's notes date form).
fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y}.{m}.{d}")
}

/// A stable pseudo-UUID for new notes packets (SnapGene only needs it unique).
fn pseudo_uuid(seq: &Sequence) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seq
        .name
        .bytes()
        .chain(seq.bases.iter().copied())
        .chain(today().bytes())
    {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let lo = h;
    let hi = h.rotate_left(29) ^ 0x9e37_79b9_7f4a_7c15;
    format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        (lo >> 32) as u32,
        (lo >> 16) as u16,
        (lo & 0xfff) as u16,
        (hi & 0xfff) as u16,
        hi >> 16 & 0xffff_ffff_ffff
    )
}

fn sequence_packet(seq: &Sequence, previous_flags: Option<u8>) -> (u8, Vec<u8>) {
    let t = match seq.seq_type {
        SeqType::Protein => PROTEIN,
        SeqType::Rna => RNA,
        _ => DNA,
    };
    let mut flags = previous_flags.unwrap_or(if t == DNA { 0x02 } else { 0 }) & !1;
    if seq.topology.is_circular() {
        flags |= 1;
    }
    let mut data = Vec::with_capacity(seq.bases.len() + 1);
    data.push(flags);
    data.extend_from_slice(&seq.bases);
    (t, data)
}

/// The features packet for `seq` (primer-derived features are left to the
/// primers packet).
fn features_packet(seq: &Sequence) -> Vec<u8> {
    let len = seq.len() as u32;
    let mut xml = String::from("<?xml version=\"1.0\"?>");
    let features: Vec<&Feature> = seq
        .features
        .iter()
        .filter(|f| !f.description.contains(PRIMER_MARK))
        .collect();
    xml.push_str(&format!("<Features nextValidID=\"{}\">", features.len()));
    for (i, f) in features.iter().enumerate() {
        let (lo, hi, wraps) = f.lo_hi();
        let (a, b) = if wraps { (hi, lo) } else { (lo, hi) };
        let (a, b) = (a.clamp(1, len.max(1)), b.clamp(1, len.max(1)));
        let directionality = match (f.strand, f.arrow) {
            (Strand::Forward, true) => 1,
            (Strand::Reverse, true) => 2,
            (Strand::Reverse, false) => 2,
            (Strand::Forward, false) => 0,
        };
        xml.push_str(&format!(
            "<Feature recentID=\"{i}\" name=\"{}\" directionality=\"{directionality}\" type=\"{}\" allowSegmentOverlaps=\"0\" consecutiveTranslationNumbering=\"1\">",
            xml_escape(&f.name),
            xml_escape(&f.kind)
        ));
        xml.push_str(&format!(
            "<Segment range=\"{a}-{b}\" color=\"#{:02x}{:02x}{:02x}\" type=\"standard\"/>",
            f.color.0, f.color.1, f.color.2
        ));
        for line in f
            .description
            .split(['\r', '\n'])
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("segments="))
        {
            let (k, v) = match line.split_once('=') {
                Some((k, v))
                    if !k.is_empty()
                        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') =>
                {
                    (k, v)
                }
                _ => ("note", line),
            };
            if v.chars().all(|c| c.is_ascii_digit())
                && !v.is_empty()
                && matches!(k, "codon_start" | "transl_table")
            {
                xml.push_str(&format!(
                    "<Q name=\"{}\"><V int=\"{v}\"/></Q>",
                    xml_escape(k)
                ));
            } else {
                xml.push_str(&format!(
                    "<Q name=\"{}\"><V text=\"{}\"/></Q>",
                    xml_escape(k),
                    xml_escape(&as_html(v))
                ));
            }
        }
        xml.push_str("</Feature>");
    }
    xml.push_str("</Features>");
    xml.into_bytes()
}

fn notes_packet(seq: &Sequence) -> Vec<u8> {
    let date = today();
    let mut xml = String::from("<Notes>\n");
    xml.push_str(&format!("<UUID>{}</UUID>\n", pseudo_uuid(seq)));
    xml.push_str("<Type>Synthetic</Type>\n");
    if !seq.comment.trim().is_empty() {
        xml.push_str(&format!(
            "<Description>{}</Description>\n",
            xml_escape(&as_html(seq.comment.trim()))
        ));
    }
    xml.push_str(&format!("<Created UTC=\"0:0:0\">{date}</Created>\n<LastModified UTC=\"0:0:0\">{date}</LastModified>\n"));
    xml.push_str("<CreatedBy>SerialClon.rs</CreatedBy>\n<SequenceClass>UNA</SequenceClass>\n<TransformedInto>unspecified</TransformedInto>\n</Notes>\n");
    xml.into_bytes()
}

/// Replaces (or adds) the `Description` of an existing notes packet.
fn patch_notes(original: &[u8], comment: &str) -> Vec<u8> {
    let mut text = String::from_utf8_lossy(original).into_owned();
    for tag in ["Description", "Comments"] {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        while let Some(i) = text.find(&open) {
            let mut end = match text[i..].find(&close) {
                Some(j) => i + j + close.len(),
                None => i + open.len(),
            };
            if text[end..].starts_with('\n') {
                end += 1;
            }
            text.replace_range(i..end, "");
        }
    }
    if !comment.trim().is_empty() {
        let element = format!(
            "<Description>{}</Description>\n",
            xml_escape(&as_html(comment.trim()))
        );
        match text.find("</Notes>") {
            Some(i) => text.insert_str(i, &element),
            None => text.push_str(&element),
        }
    }
    text.into_bytes()
}

const DEFAULT_PROPERTIES: &str = "<AdditionalSequenceProperties><UpstreamStickiness>0</UpstreamStickiness><DownstreamStickiness>0</DownstreamStickiness><UpstreamModification>Unmodified</UpstreamModification><DownstreamModification>Unmodified</DownstreamModification></AdditionalSequenceProperties>";
const EMPTY_PRIMERS: &str = "<?xml version=\"1.0\"?><Primers nextValidID=\"0\"><HybridizationParams minContinuousMatchLen=\"10\" allowMismatch=\"1\" minMeltingTemperature=\"40\" showAdditionalFivePrimeMatches=\"1\" minimumFivePrimeAnnealing=\"15\"/></Primers>\n";

/// Serialises a sequence as a `.dna` file.
///
/// With [`Sequence::snapgene_extras`] present (the sequence came from a
/// SnapGene file) every packet is written back in its original order;
/// the sequence packet is regenerated (identical if the bases and topology
/// did not change), the features packet only if the features changed, the
/// notes packet only if the comment changed. Without extras a minimal file
/// is written.
pub fn write(seq: &Sequence) -> Vec<u8> {
    let mut out = Vec::new();
    let Some(extras) = &seq.snapgene_extras else {
        let molecule: u16 = match seq.seq_type {
            SeqType::Protein => 2,
            SeqType::Rna => 7,
            _ => 1,
        };
        let mut header = COOKIE.to_vec();
        header.extend_from_slice(&molecule.to_be_bytes());
        header.extend_from_slice(&EXPORT_VERSION.to_be_bytes());
        header.extend_from_slice(&IMPORT_VERSION.to_be_bytes());
        packet(&mut out, HEADER, &header);
        let (t, data) = sequence_packet(seq, None);
        packet(&mut out, t, &data);
        packet(&mut out, PROPERTIES, DEFAULT_PROPERTIES.as_bytes());
        packet(&mut out, FEATURES, &features_packet(seq));
        packet(&mut out, PRIMERS, EMPTY_PRIMERS.as_bytes());
        packet(&mut out, NOTES, &notes_packet(seq));
        return out;
    };
    let len = seq.len() as u32;
    let features_unchanged = features_of(extras, len)
        .map(|f| f == seq.features)
        .unwrap_or(false);
    let comment_unchanged = comment_of(extras)
        .map(|(c, _)| c == seq.comment)
        .unwrap_or(false);
    let has_features_packet = extras.iter().any(|(t, _)| *t == FEATURES);
    let has_sequence_packet = extras
        .iter()
        .any(|(t, _)| matches!(*t, DNA | PROTEIN | RNA));
    // Features that belong in a features packet (primer-derived ones live
    // in the primers packet, which is kept as it was).
    let own_features = seq
        .features
        .iter()
        .any(|f| !f.description.contains(PRIMER_MARK));
    let mut features_written = false;
    for (t, data) in extras {
        match *t {
            DNA | PROTEIN | RNA => {
                let (nt, ndata) = sequence_packet(seq, data.first().copied());
                packet(&mut out, nt, &ndata);
            }
            COMPRESSED if !has_sequence_packet => {
                // The only sequence was compressed: replace it by a plain one
                // so SnapGene reads what we have.
                let (nt, ndata) = sequence_packet(seq, None);
                packet(&mut out, nt, &ndata);
            }
            FEATURES => {
                if features_unchanged {
                    packet(&mut out, FEATURES, data);
                } else {
                    packet(&mut out, FEATURES, &features_packet(seq));
                }
                features_written = true;
            }
            PRIMERS | NOTES if !has_features_packet && !features_written && own_features => {
                packet(&mut out, FEATURES, &features_packet(seq));
                features_written = true;
                if *t == NOTES && !comment_unchanged {
                    packet(&mut out, NOTES, &patch_notes(data, &seq.comment));
                } else {
                    packet(&mut out, *t, data);
                }
            }
            NOTES if !comment_unchanged => {
                packet(&mut out, NOTES, &patch_notes(data, &seq.comment))
            }
            _ => packet(&mut out, *t, data),
        }
    }
    if !features_written && own_features {
        packet(&mut out, FEATURES, &features_packet(seq));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    const FIXTURES: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/sequences/snapgene"
    );

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(Path::new(FIXTURES).join(name)).unwrap()
    }

    fn fixtures() -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(FIXTURES)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "dna"))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn every_fixture_round_trips_byte_for_byte() {
        let files = fixtures();
        assert!(files.len() >= 20, "{} fixtures", files.len());
        for path in files {
            let bytes = std::fs::read(&path).unwrap();
            assert!(looks_like(&bytes), "{}", path.display());
            let seq = read(&bytes, "x").unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(!seq.bases.is_empty(), "{}", path.display());
            assert_eq!(seq.source, Source::SnapGene);
            assert_eq!(write(&seq), bytes, "{} does not round-trip", path.display());
        }
    }

    #[test]
    fn pfa_kanmx4_features_match_snapgene() {
        let seq = read(&fixture("biopython-pFA-KanMX4.dna"), "biopython-pFA-KanMX4").unwrap();
        assert_eq!(seq.name, "pFA6-kanMX4"); // CustomMapLabel
        assert_eq!(seq.len(), 3941);
        assert_eq!(seq.topology, Topology::Circular);
        assert_eq!(seq.seq_type, SeqType::Dna);
        assert!(
            seq.comment
                .starts_with("Plasmid carrying the kanMX selector module")
        );
        assert!(!seq.comment.contains("<html>"));
        assert_eq!(seq.features.len(), 9);
        let sp6 = seq
            .features
            .iter()
            .find(|f| f.name == "SP6 promoter")
            .unwrap();
        assert_eq!(
            (sp6.start, sp6.end, sp6.strand, sp6.arrow),
            (3925, 2, Strand::Forward, true)
        );
        assert_eq!(sp6.lo_hi(), (2, 3925, true));
        assert_eq!(
            sp6.description,
            "note=promoter for bacteriophage SP6 RNA polymerase"
        );
        let t7 = seq
            .features
            .iter()
            .find(|f| f.name == "T7 promoter")
            .unwrap();
        assert_eq!((t7.start, t7.end, t7.strand), (1597, 1579, Strand::Reverse));
        assert_eq!(t7.lo_hi(), (1579, 1597, false));
        let amp = seq.features.iter().find(|f| f.name == "AmpR").unwrap();
        assert_eq!(amp.kind, "CDS");
        assert_eq!(amp.color, Rgb(0xcc, 0xff, 0xcc));
        assert!(amp.description.contains("segments="), "{}", amp.description);
        let tef = seq
            .features
            .iter()
            .find(|f| f.name == "TEF terminator")
            .unwrap();
        assert!(!tef.arrow); // no directionality
    }

    #[test]
    fn origin_spanning_features_on_a_44_bp_circle() {
        let seq = read(&fixture("sgffp-origin_spanning_features.dna"), "x").unwrap();
        assert_eq!(seq.len(), 44);
        let by = |n: &str| seq.features.iter().find(|f| f.name == n).unwrap().clone();
        let none = by("origin_spanning_no_dir");
        assert_eq!(
            (none.start, none.end, none.strand, none.arrow),
            (38, 7, Strand::Forward, false)
        );
        assert_eq!(none.lo_hi(), (7, 38, true));
        assert_eq!(none.len(44), 14);
        let right = by("origin_spanning_right");
        assert_eq!(
            (right.start, right.end, right.strand, right.arrow),
            (38, 7, Strand::Forward, true)
        );
        assert_eq!(right.description, "codon_start=1\rtransl_table=1");
        let left = by("origin_spanning_left");
        assert_eq!(
            (left.start, left.end, left.strand),
            (7, 38, Strand::Reverse)
        );
        assert_eq!(left.lo_hi(), (7, 38, true));
        // Two primers → primer_bind features at 1-based coordinates.
        assert_eq!(
            seq.features
                .iter()
                .filter(|f| f.kind == "primer_bind")
                .count(),
            2
        );
    }

    #[test]
    fn primers_follow_the_hybridization_parameters() {
        let seq = read(&fixture("biopython-sample-hybridization-params.dna"), "x").unwrap();
        let primers: Vec<&Feature> = seq
            .features
            .iter()
            .filter(|f| f.kind == "primer_bind")
            .collect();
        // XhoI-hht2(US)-Fwd binds at 0-based 497-515 → 498..516 forward; its
        // 18 °C off-target site is below the 40 °C minimum and is dropped.
        let fwd = primers
            .iter()
            .find(|p| p.name == "XhoI-hht2(US)-Fwd")
            .unwrap();
        assert_eq!(
            (fwd.start, fwd.end, fwd.strand),
            (498, 516, Strand::Forward)
        );
        assert!(fwd.description.starts_with("snapgene_primer=AAAAAActcgag"));
        assert_eq!(
            primers
                .iter()
                .filter(|p| p.name == "XhoI-hht2(US)-Fwd")
                .count(),
            1
        );
        assert_eq!(primers.len(), 2);
    }

    #[test]
    fn protein_rna_and_degenerate_files() {
        let p = read(&fixture("sgffp-test.prot.dna"), "x").unwrap();
        assert_eq!(p.seq_type, SeqType::Protein);
        assert_eq!(p.len(), 51);
        assert!(p.bases.starts_with(b"KKRREREFLW"));
        let r = read(&fixture("sgffp-test.rna.dna"), "x").unwrap();
        assert_eq!(r.seq_type, SeqType::Rna);
        let d = read(&fixture("sgffp-degenerate-1.dna"), "x").unwrap();
        assert_eq!(d.seq_type, SeqType::DegenerateDna);
        let plain = read(&fixture("biopython-sample-d.dna"), "x").unwrap();
        assert_eq!(plain.comment, "Sample Sequence D");
        assert!(plain.bases.iter().any(|b| b.is_ascii_lowercase()));
        assert_eq!(plain.topology, Topology::Linear);
    }

    #[test]
    fn gibson_assembly_matches_its_fasta() {
        let seq = read(&fixture("sgffp-gibson_assembly.dna"), "x").unwrap();
        // The FASTA the same project ships holds the product, the linearised
        // vector and the fragment; the .dna is the product.
        let fasta = crate::fasta::read(&fixture("sgffp-gibson_assembly.fasta"));
        assert_eq!(fasta.len(), 3);
        let product = fasta.iter().find(|s| s.name == "product").unwrap();
        assert_eq!(seq.len(), 10558);
        assert!(
            seq.bases.eq_ignore_ascii_case(&product.bases),
            "sequence differs from the FASTA the same project ships"
        );
        // 29 features in the features packet (14 of them primer_bind sites
        // SnapGene stores as features) plus the 4 primers of the primers packet.
        assert_eq!(
            seq.features
                .iter()
                .filter(|f| !f.description.contains(PRIMER_MARK))
                .count(),
            29
        );
        assert_eq!(
            seq.features
                .iter()
                .filter(|f| f.description.contains(PRIMER_MARK))
                .count(),
            4
        );
        let ase1 = seq
            .features
            .iter()
            .find(|f| f.name == "ase1" && f.kind == "CDS")
            .unwrap();
        assert_eq!((ase1.start, ase1.end), (1686, 3928)); // two exons, gap skipped
        assert!(ase1.description.contains("segments=1686-2109;2157-3928"));
    }

    #[test]
    fn compressed_dna_decodes() {
        // One 2-bit section "GATC" ×2, one IUPAC "NR", one N-run of 3, lower-case 0..=1.
        let mut d = Vec::new();
        d.extend_from_slice(&0u32.to_be_bytes()); // cl (unused)
        d.extend_from_slice(&13u32.to_be_bytes()); // ul
        d.push(7); // stamp
        d.extend_from_slice(&3u32.to_be_bytes()); // chunks
        d.extend_from_slice(&1u32.to_be_bytes()); // lowercases
        d.push(0x01);
        d.extend_from_slice(&8u32.to_be_bytes());
        d.extend_from_slice(&[0b0001_1011, 0b0001_1011]);
        d.push(0x02);
        d.extend_from_slice(&2u32.to_be_bytes());
        d.push(0x4a);
        d.push(0x03);
        d.extend_from_slice(&3u32.to_be_bytes());
        d.extend_from_slice(&0u32.to_be_bytes());
        d.extend_from_slice(&1u32.to_be_bytes());
        assert_eq!(decode_compressed(&d).unwrap(), b"gaTCGATCNRNNN");
    }

    #[test]
    fn a_new_sequence_writes_a_minimal_file_biopython_would_read() {
        let mut seq = crate::xdna::read(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../reference/sequences/pBR322-annotated.xdna"
            ))
            .unwrap(),
            "pBR322",
        )
        .unwrap();
        seq.comment = "Cloning vector pBR322 <test> & \"quotes\"".into();
        let bytes = write(&seq);
        assert!(looks_like(&bytes));
        let back = read(&bytes, "pBR322").unwrap();
        assert_eq!(back.bases, seq.bases);
        assert_eq!(back.topology, Topology::Circular);
        assert_eq!(back.seq_type, SeqType::Dna);
        assert_eq!(back.comment, seq.comment);
        assert_eq!(back.features.len(), 3);
        for (a, b) in back.features.iter().zip(&seq.features) {
            assert_eq!(
                (
                    a.name.as_str(),
                    a.kind.as_str(),
                    a.start,
                    a.end,
                    a.strand,
                    a.color,
                    a.arrow
                ),
                (
                    b.name.as_str(),
                    b.kind.as_str(),
                    b.start,
                    b.end,
                    b.strand,
                    b.color,
                    b.arrow
                )
            );
        }
        // Writing the re-read sequence gives the same bytes again.
        assert_eq!(write(&back), bytes);
    }

    #[test]
    fn edits_keep_the_other_packets() {
        let original = fixture("biopython-sample-hybridization-params.dna");
        let mut seq = read(&original, "x").unwrap();
        let types_before: Vec<u8> = packets(&original)
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        seq.bases[0] = b'T';
        seq.topology = seq.topology.toggled();
        seq.comment = "edited".into();
        seq.features.push(Feature {
            name: "new".into(),
            description: "note=added here".into(),
            kind: "misc_feature".into(),
            start: 10,
            end: 20,
            strand: Strand::Forward,
            show: true,
            protect: true,
            arrow: true,
            color: Rgb(1, 2, 3),
        });
        let bytes = write(&seq);
        let types_after: Vec<u8> = packets(&bytes)
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(types_after, types_before, "packet order and kinds are kept");
        let back = read(&bytes, "x").unwrap();
        assert_eq!(back.bases, seq.bases);
        assert_eq!(back.topology, seq.topology);
        assert_eq!(back.comment, "edited");
        assert!(
            back.features
                .iter()
                .any(|f| f.name == "new" && f.start == 10 && f.end == 20)
        );
        // Primers came back from the untouched primers packet, not duplicated.
        assert_eq!(
            back.features
                .iter()
                .filter(|f| f.kind == "primer_bind")
                .count(),
            2
        );
        assert_eq!(back.features.len(), seq.features.len());
    }

    #[test]
    fn truncated_files_are_rejected() {
        let bytes = fixture("sgffp-test.dna");
        assert!(read(&bytes[..bytes.len() - 3], "x").is_err());
        assert!(read(&bytes[..10], "x").is_err());
        assert!(!looks_like(b"LOCUS"));
    }
}
