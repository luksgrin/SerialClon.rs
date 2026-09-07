//! GenBank / EMBL / DDBJ flat files, including Serial Cloner's own dialect.
//!
//! The reader is tolerant: NCBI records with `join()` locations, EMBL
//! two-letter line codes, CR / LF / CRLF line ends, and Serial Cloner's
//! 13-column keyword field all parse. Feature colours, visibility, protection
//! and arrows come from the `SerialCloner_*` qualifiers when present and from
//! [`Rgb::default_for_kind`] otherwise ("Colorize annotation when importing
//! from non-serial cloner format").
//!
//! The writer produces what "Export as GenBank" / "Copy as GenBank" did
//! (Features ReadMe §VII.A), in standard column positions:
//!
//! ```text
//! LOCUS       pBluescript_II_SK(+)        2961 bp    DNA     circular SYN 02-MAY-2010
//! DEFINITION  .
//! …
//! COMMENT     Serial Cloner Genbank Format
//! COMMENT     SerialCloner_Type=DNA
//! COMMENT     SerialCloner_Comments=gi|58063|emb|X52328|ARBL2SKP pBluescript II SK(+) vector DNA
//!             , phagemid excised from lambda ZAPII
//! COMMENT     SerialCloner_Ends=0,0,,0,
//! FEATURES             Location/Qualifiers
//!      primer_bind     complement(771..790)
//!                      /label=T3
//!                      /SerialCloner_Color=&h008B8B
//!                      /SerialCloner_Show=True
//!                      /SerialCloner_Protect=False
//!                      /SerialCloner_Arrow=True
//! ORIGIN
//!         1 ctaaattgta agcgttaata …
//! //
//! ```
//!
//! `SerialCloner_Ends` is only attested as `0,0,,0,` (blunt); the cohesive
//! form written here — `left,right,left-bases,0,right-bases` — is a guess
//! that reproduces the attested case and is documented as such.

use sc_seq::alphabet::{is_acgt, is_iupac};
use sc_seq::{Ends, Feature, Overhang, Rgb, SeqType, Sequence, Source, Strand, Topology};

use crate::{Error, Result, lines};

/// A feature as it appears in the file, before interpretation.
#[derive(Debug, Default)]
struct RawFeature {
    key: String,
    location: String,
    qualifiers: Vec<(String, String)>,
}

#[derive(Debug, Default)]
struct Record {
    locus_line: String,
    name: String,
    circular: bool,
    seq_type: SeqType,
    definition_lines: Vec<String>,
    /// `SerialCloner_Comments=` value with its continuation lines.
    sc_comment: Option<Vec<String>>,
    sc_type: Option<String>,
    sc_ends: Option<String>,
    features: Vec<RawFeature>,
    origin: String,
}

#[derive(PartialEq)]
enum Section {
    Header,
    Features,
    Origin,
}

/// Reads every record in a GenBank or EMBL file.
pub fn read(bytes: &[u8]) -> Result<Vec<Sequence>> {
    let text = String::from_utf8_lossy(bytes);
    let mut records = Vec::new();
    let mut rec = Record::default();
    let mut section = Section::Header;
    let mut keyword = String::new();
    let mut started = false;

    for raw in lines(&text) {
        let line = normalise_embl(raw);
        let Some(line) = line else { continue };
        if line.trim_end() == "//" {
            if started {
                records.push(std::mem::take(&mut rec));
            }
            section = Section::Header;
            started = false;
            continue;
        }
        // Blank lines are skipped, except an indented one continuing a
        // Serial Cloner comment (an empty comment line).
        let continuing_comment = section == Section::Header
            && keyword == "COMMENT"
            && rec.sc_comment.is_some()
            && rec.sc_ends.is_none();
        if line.trim().is_empty() && !(continuing_comment && line.starts_with(' ')) {
            continue;
        }
        started = true;
        let starts_keyword = !line.starts_with(' ');
        if starts_keyword {
            let (kw, value) = split_keyword(&line);
            keyword = kw.to_owned();
            match kw {
                "LOCUS" | "ID" => parse_locus(&mut rec, &line, value),
                "FEATURES" | "FH" => section = Section::Features,
                "ORIGIN" | "SQ" => section = Section::Origin,
                "DEFINITION" => rec.definition_lines.push(line.to_owned()),
                "COMMENT" => parse_comment(&mut rec, value),
                _ => {}
            }
            continue;
        }
        match section {
            Section::Header => match keyword.as_str() {
                "DEFINITION" => rec.definition_lines.push(line.to_owned()),
                "COMMENT" => {
                    if let Some(lines) = rec.sc_comment.as_mut()
                        && rec.sc_ends.is_none()
                    {
                        lines.push(line.trim_start().to_owned());
                    }
                }
                _ => {}
            },
            Section::Features => parse_feature_line(&mut rec, &line),
            Section::Origin => rec.origin.push_str(&line),
        }
    }
    if started {
        records.push(rec);
    }
    records.into_iter().map(finish).collect()
}

/// Maps EMBL line codes onto GenBank keywords; returns `None` for lines to skip.
fn normalise_embl(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    if bytes.len() >= 2
        && bytes[0].is_ascii_uppercase()
        && bytes[1].is_ascii_uppercase()
        && (bytes.len() == 2 || bytes[2] == b' ')
    {
        let code = &line[..2];
        let rest = line.get(5..).unwrap_or("");
        return match code {
            "XX" => None,
            "ID" => Some(format!("LOCUS       {rest}")),
            "DE" => Some(format!("DEFINITION  {rest}")),
            "AC" => Some(format!("ACCESSION   {rest}")),
            "CC" => Some(format!("COMMENT     {rest}")),
            "FH" => Some("FEATURES             Location/Qualifiers".to_owned()),
            "FT" => Some(format!("     {rest}")),
            "SQ" => Some("ORIGIN".to_owned()),
            _ if line.len() > 12
                && line.as_bytes()[..12]
                    .iter()
                    .all(|b| b.is_ascii_uppercase() || *b == b' ') =>
            {
                Some(line.to_owned())
            }
            _ => Some(line.to_owned()),
        };
    }
    Some(line.to_owned())
}

fn split_keyword(line: &str) -> (&str, &str) {
    match line.split_once(char::is_whitespace) {
        Some((kw, rest)) => (kw, rest.trim()),
        None => (line.trim(), ""),
    }
}

fn parse_locus(rec: &mut Record, line: &str, value: &str) {
    rec.locus_line = line.to_owned();
    let tokens: Vec<&str> = value
        .split(|c: char| c.is_whitespace() || c == ';')
        .filter(|t| !t.is_empty())
        .collect();
    rec.name = tokens.first().map(|t| (*t).to_owned()).unwrap_or_default();
    let lower = value.to_ascii_lowercase();
    rec.circular = lower.contains("circular");
    rec.seq_type = if lower.contains("rna") {
        SeqType::Rna
    } else if lower.contains(" aa") || lower.contains("prt") || lower.contains("protein") {
        SeqType::Protein
    } else {
        SeqType::Dna
    };
}

fn parse_comment(rec: &mut Record, value: &str) {
    if let Some(v) = value.strip_prefix("SerialCloner_Comments=") {
        rec.sc_comment = Some(vec![v.to_owned()]);
    } else if let Some(v) = value.strip_prefix("SerialCloner_Type=") {
        rec.sc_type = Some(v.trim().to_owned());
    } else if let Some(v) = value.strip_prefix("SerialCloner_Ends=") {
        rec.sc_ends = Some(v.trim().to_owned());
    }
}

fn parse_feature_line(rec: &mut Record, line: &str) {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if let Some(q) = trimmed.strip_prefix('/')
        && indent >= 6
    {
        let (key, value) = match q.split_once('=') {
            Some((k, v)) => (k.trim().to_owned(), v.trim().to_owned()),
            None => (q.trim().to_owned(), String::new()),
        };
        if let Some(f) = rec.features.last_mut() {
            f.qualifiers.push((key, value));
        }
        return;
    }
    // A feature key line: "     CDS             complement(3293..4153)"
    if indent == 5 || (indent > 0 && indent < 10) {
        if let Some((key, location)) = trimmed.split_once(char::is_whitespace) {
            rec.features.push(RawFeature {
                key: key.to_owned(),
                location: location.trim().to_owned(),
                qualifiers: Vec::new(),
            });
            return;
        }
    }
    // Continuation of the previous qualifier value (or of a long location).
    if let Some(f) = rec.features.last_mut() {
        if let Some((key, value)) = f.qualifiers.last_mut() {
            if key != "translation" && !value.is_empty() {
                value.push(' ');
            }
            value.push_str(trimmed);
        } else {
            f.location.push_str(trimmed);
        }
    }
}

/// Parses a location into (first, last, strand, wraps), `first`/`last` being
/// the ends in top-strand reading order: `lo..hi` normally, or `a` and `b`
/// for `join(a..len,1..b)`.
fn parse_location(loc: &str, seq_len: u32) -> Option<(u32, u32, Strand, bool)> {
    let strand = if loc.contains("complement") {
        Strand::Reverse
    } else {
        Strand::Forward
    };
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let cleaned: String = loc
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ' '))
        .collect();
    for part in cleaned.split(',') {
        let inner = part.trim_matches(|c: char| !c.is_ascii_digit());
        let nums: Vec<u32> = inner
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        match nums[..] {
            [a] => ranges.push((a, a)),
            [a, b, ..] => ranges.push((a.min(b), a.max(b))),
            [] => {}
        }
    }
    if ranges.is_empty() {
        return None;
    }
    // join(a..len,1..b) is a feature crossing the origin.
    if let [(a, hi_end), (1, b)] = ranges[..]
        && hi_end == seq_len
        && seq_len > 0
    {
        return Some((a, b, strand, true));
    }
    let lo = ranges.iter().map(|r| r.0).min()?;
    let hi = ranges.iter().map(|r| r.1).max()?;
    Some((lo, hi, strand, false))
}

fn parse_color(value: &str) -> Option<Rgb> {
    let v = value.trim().trim_matches('"');
    let hex = v
        .strip_prefix("&h")
        .or_else(|| v.strip_prefix("&H"))
        .or_else(|| v.strip_prefix('#'));
    if let Some(h) = hex
        && h.len() == 6
        && let Ok(n) = u32::from_str_radix(h, 16)
    {
        return Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8));
    }
    let parts: Vec<u8> = v
        .split(',')
        .filter(|p| !p.trim().is_empty())
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    match parts[..] {
        [r, g, b] => Some(Rgb(r, g, b)),
        _ => None,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().trim_matches('"').to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_owned()
}

fn convert_feature(raw: RawFeature, seq_len: u32) -> Option<Feature> {
    if raw.key == "source" {
        return None;
    }
    let (first, last, strand, _wraps) = parse_location(&raw.location, seq_len)?;
    // A reverse feature starts (5' end) where the top strand's span ends.
    let (start, end) = match strand {
        Strand::Forward => (first, last),
        Strand::Reverse => (last, first),
    };
    let get = |k: &str| {
        raw.qualifiers
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.as_str())
    };
    let name_key = [
        "label",
        "gene",
        "product",
        "standard_name",
        "note",
        "bound_moiety",
    ]
    .into_iter()
    .find(|k| get(k).is_some_and(|v| !unquote(v).is_empty()));
    let name = name_key
        .map(|k| unquote(get(k).unwrap()))
        .unwrap_or_else(|| raw.key.clone());
    let description = raw
        .qualifiers
        .iter()
        .filter(|(k, _)| {
            Some(k.as_str()) != name_key && !k.starts_with("SerialCloner_") && k != "translation"
        })
        .map(|(k, v)| {
            if v.is_empty() {
                k.clone()
            } else {
                format!("{k}={}", unquote(v))
            }
        })
        .collect::<Vec<_>>()
        .join("\r");
    let color = get("SerialCloner_Color")
        .and_then(parse_color)
        .unwrap_or_else(|| Rgb::default_for_kind(&raw.key));
    Some(Feature {
        name,
        description,
        kind: raw.key,
        start,
        end,
        strand,
        show: get("SerialCloner_Show")
            .and_then(parse_bool)
            .unwrap_or(true),
        protect: get("SerialCloner_Protect")
            .and_then(parse_bool)
            .unwrap_or(false),
        arrow: get("SerialCloner_Arrow")
            .and_then(parse_bool)
            .unwrap_or(true),
        color,
    })
}

fn parse_ends(value: &str) -> Ends {
    let fields: Vec<&str> = value.split(',').collect();
    let int = |i: usize| {
        fields
            .get(i)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(0)
    };
    let seq = |i: usize| {
        fields
            .get(i)
            .map(|s| s.trim().as_bytes().to_vec())
            .unwrap_or_default()
    };
    let (left, right) = (int(0), int(1));
    Ends {
        left: Overhang {
            length: left,
            bases: if left == 0 { Vec::new() } else { seq(2) },
        },
        right: Overhang {
            length: right,
            bases: if right == 0 { Vec::new() } else { seq(4) },
        },
    }
}

fn finish(rec: Record) -> Result<Sequence> {
    if rec.locus_line.is_empty() && rec.origin.is_empty() {
        return Err(Error::Malformed("genbank: no LOCUS line".into()));
    }
    let protein = rec.seq_type == SeqType::Protein || rec.sc_type.as_deref() == Some("Protein");
    let bases: Vec<u8> = rec
        .origin
        .bytes()
        .filter(|b| b.is_ascii_alphabetic() && (protein || is_iupac(*b)))
        .collect();
    let seq_type = if protein {
        SeqType::Protein
    } else if rec.seq_type == SeqType::Rna {
        SeqType::Rna
    } else if bases.iter().all(|&b| is_acgt(b)) {
        SeqType::Dna
    } else {
        SeqType::DegenerateDna
    };
    let len = bases.len() as u32;
    let comment = match &rec.sc_comment {
        Some(lines) => lines.join("\r"),
        None => {
            let mut c = String::new();
            for line in std::iter::once(&rec.locus_line).chain(&rec.definition_lines) {
                if !line.is_empty() {
                    c.push_str(line.trim_end());
                    c.push('\r');
                }
            }
            c
        }
    };
    let features = rec
        .features
        .into_iter()
        .filter_map(|f| convert_feature(f, len))
        .collect();
    Ok(Sequence {
        name: rec.name,
        bases,
        seq_type,
        topology: if rec.circular {
            Topology::Circular
        } else {
            Topology::Linear
        },
        comment,
        features,
        ends: rec.sc_ends.as_deref().map(parse_ends).unwrap_or_default(),
        source: Source::GenBank,
        strider_header: None,
    })
}

fn location_of(f: &Feature, len: u32) -> String {
    let (lo, hi, wraps) = f.lo_hi();
    // A wrapping feature covers hi..len and 1..lo.
    let inner = if wraps {
        format!("join({hi}..{len},1..{lo})")
    } else {
        format!("{lo}..{hi}")
    };
    match f.strand {
        Strand::Forward => inner,
        Strand::Reverse => format!("complement({inner})"),
    }
}

fn qualifier(out: &mut String, key: &str, value: &str) {
    out.push_str("                     /");
    out.push_str(key);
    if !value.is_empty() {
        out.push('=');
        out.push_str(value);
    }
    out.push('\n');
}

fn quoted(value: &str) -> String {
    if value.parse::<f64>().is_ok() || value.starts_with('"') {
        value.to_owned()
    } else {
        format!("\"{}\"", value.replace('"', "'"))
    }
}

/// Serialises a sequence in Serial Cloner's GenBank dialect. `date` is the
/// LOCUS date, e.g. `07-SEP-2026`.
pub fn write(seq: &Sequence, date: &str) -> String {
    let len = seq.len() as u32;
    let molecule = match seq.seq_type {
        SeqType::Rna => "RNA",
        SeqType::Protein => "PRT",
        _ => "DNA",
    };
    let unit = if seq.seq_type == SeqType::Protein {
        "aa"
    } else {
        "bp"
    };
    let topology = if seq.topology.is_circular() {
        "circular"
    } else {
        "linear"
    };
    let name = seq.name.replace(' ', "_");
    let mut out = format!(
        "LOCUS       {name:<16} {len:>11} {unit}    {molecule:<7} {topology:<8} SYN {date}\n"
    );
    out.push_str("DEFINITION  .\nACCESSION   .\nVERSION     .\nKEYWORDS    .\nSOURCE      .\n  ORGANISM  .\n");
    out.push_str("COMMENT     Serial Cloner Genbank Format\n");
    out.push_str(&format!(
        "COMMENT     SerialCloner_Type={}\n",
        match seq.seq_type {
            SeqType::Protein => "Protein",
            SeqType::Rna => "RNA",
            SeqType::DegenerateDna => "Degenerate DNA",
            _ => "DNA",
        }
    ));
    let mut comment_lines = seq.comment.split(['\r', '\n']);
    out.push_str(&format!(
        "COMMENT     SerialCloner_Comments={}\n",
        comment_lines.next().unwrap_or("")
    ));
    for line in comment_lines {
        out.push_str("            ");
        out.push_str(line);
        out.push('\n');
    }
    let (l, r) = (&seq.ends.left, &seq.ends.right);
    out.push_str(&format!(
        "COMMENT     SerialCloner_Ends={},{},{},0,{}\n",
        l.length,
        r.length,
        String::from_utf8_lossy(&l.bases),
        String::from_utf8_lossy(&r.bases)
    ));
    out.push_str("FEATURES             Location/Qualifiers\n");
    for f in &seq.features {
        out.push_str(&format!("     {:<15} {}\n", f.kind, location_of(f, len)));
        qualifier(&mut out, "label", &f.name);
        for line in f
            .description
            .split(['\r', '\n'])
            .filter(|l| !l.trim().is_empty())
        {
            match line.split_once('=') {
                Some((k, v))
                    if !k.is_empty()
                        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') =>
                {
                    qualifier(&mut out, k, &quoted(v));
                }
                _ => qualifier(&mut out, "note", &quoted(line)),
            }
        }
        qualifier(
            &mut out,
            "SerialCloner_Color",
            &format!("&h{:02X}{:02X}{:02X}", f.color.0, f.color.1, f.color.2),
        );
        qualifier(
            &mut out,
            "SerialCloner_Show",
            if f.show { "True" } else { "False" },
        );
        qualifier(
            &mut out,
            "SerialCloner_Protect",
            if f.protect { "True" } else { "False" },
        );
        qualifier(
            &mut out,
            "SerialCloner_Arrow",
            if f.arrow { "True" } else { "False" },
        );
    }
    out.push_str("ORIGIN\n");
    for (i, line) in seq.bases.chunks(60).enumerate() {
        out.push_str(&format!("{:>9}", i * 60 + 1));
        for group in line.chunks(10) {
            out.push(' ');
            out.push_str(std::str::from_utf8(group).unwrap_or("?"));
        }
        out.push('\n');
    }
    out.push_str("//\n");
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
    fn ncbi_pbr322_matches_its_xdna_twin() {
        let gb = &read(&fixture("pBR322.gb")).unwrap()[0];
        let x = crate::xdna::read(&fixture("pBR322.xdna"), "pBR322").unwrap();
        assert_eq!(gb.name, "SYNPBR322");
        assert_eq!(gb.len(), 4361);
        assert_eq!(gb.topology, Topology::Circular);
        assert_eq!(gb.seq_type, SeqType::Dna);
        // The xdna copy has the CDS spans upper-cased; the bases are the same.
        assert!(gb.bases.eq_ignore_ascii_case(&x.bases));
        // The original's import puts the LOCUS and DEFINITION lines in the comment.
        assert_eq!(gb.comment, x.comment);
        assert_eq!(gb.source, Source::GenBank);
        let bla = gb
            .features
            .iter()
            .find(|f| f.name == "bla" && f.kind == "CDS")
            .unwrap();
        assert_eq!(
            (bla.start, bla.end, bla.strand),
            (4153, 3293, Strand::Reverse)
        );
        assert!(bla.description.contains("product=beta-lactamase"));
        assert!(!bla.description.contains("translation"));
        assert_eq!(bla.color, Rgb::default_for_kind("CDS"));
        let tet = gb
            .features
            .iter()
            .find(|f| f.name == "tet" && f.kind == "CDS")
            .unwrap();
        assert_eq!(
            (tet.start, tet.end, tet.strand),
            (86, 1276, Strand::Forward)
        );
        assert!(gb.features.iter().all(|f| f.kind != "source"));
        let single = gb.features.iter().find(|f| f.kind == "rep_origin").unwrap();
        assert_eq!((single.start, single.end), (2535, 2535));
    }

    #[test]
    fn serial_cloner_export_of_sk_plus_is_read_with_colours() {
        let seq = &read(&fixture("pBluescript-II-SK+.gb")).unwrap()[0];
        assert_eq!(seq.name, "pBluescript_II_SK(+)");
        assert_eq!(seq.len(), 2961);
        assert_eq!(seq.topology, Topology::Circular);
        assert_eq!(
            seq.comment,
            "gi|58063|emb|X52328|ARBL2SKP pBluescript II SK(+) vector DNA\r, phagemid excised from lambda ZAPII"
        );
        assert!(seq.ends.is_blunt());
        assert_eq!(seq.features.len(), 9);
        let t3 = &seq.features[0];
        assert_eq!((t3.name.as_str(), t3.kind.as_str()), ("T3", "primer_bind"));
        assert_eq!((t3.start, t3.end, t3.strand), (790, 771, Strand::Reverse));
        assert_eq!(t3.color, Rgb(0x00, 0x8B, 0x8B));
        assert_eq!(t3.description, "");
        let f1 = seq.features.iter().find(|f| f.name == "F1 ori").unwrap();
        assert!(f1.protect);
        assert_eq!(f1.color, Rgb(0xFF, 0x80, 0x00));
        // Same plasmid family as the KS(+) fixture: same backbone features.
        let amp = seq.features.iter().find(|f| f.name == "AmpR").unwrap();
        assert_eq!((amp.start, amp.end), (2635, 1976));
    }

    #[test]
    fn write_then_read_preserves_everything_that_matters() {
        let mut seq = crate::xdna::read(&fixture("pBR322-annotated.xdna"), "pBR322").unwrap();
        seq.features.push(Feature {
            name: "wrap".into(),
            description: "note=crosses the origin\rfree text".into(),
            kind: "misc_feature".into(),
            start: 4300,
            end: 50,
            strand: Strand::Forward,
            show: false,
            protect: true,
            arrow: false,
            color: Rgb(1, 2, 3),
        });
        seq.ends.left = Overhang::five_prime(b"GATC");
        let text = write(&seq, "07-SEP-2026");
        assert!(text.starts_with(
            "LOCUS       pBR322                  4361 bp    DNA     circular SYN 07-SEP-2026\n"
        ));
        assert!(text.contains("     rep_origin      complement(2516..3144)\n                     /label=ColE1 origin\n"));
        assert!(text.contains("     misc_feature    join(4300..4361,1..50)\n"));
        assert!(text.contains("/SerialCloner_Color=&h4682B4\n"));
        assert!(text.contains("COMMENT     SerialCloner_Ends=4,0,GATC,0,\n"));
        assert!(text.contains(
            "\n        1 ttctcatgtt tgacagctta tcatcgataa gctttaatgc ggtagtttat cacagttaaa\n"
        ));
        let back = &read(text.as_bytes()).unwrap()[0];
        assert_eq!(back.bases, seq.bases);
        assert_eq!(back.comment, seq.comment);
        assert_eq!(back.topology, seq.topology);
        assert_eq!(back.ends, seq.ends);
        assert_eq!(back.features.len(), 4);
        for (a, b) in back.features.iter().zip(&seq.features) {
            assert_eq!(
                (a.name.as_str(), a.kind.as_str(), a.start, a.end, a.strand),
                (b.name.as_str(), b.kind.as_str(), b.start, b.end, b.strand)
            );
            assert_eq!(
                (a.show, a.protect, a.arrow, a.color),
                (b.show, b.protect, b.arrow, b.color)
            );
        }
        assert_eq!(
            back.features[3].description,
            "note=crosses the origin\rnote=free text"
        );
    }

    #[test]
    fn blunt_ends_are_written_exactly_as_the_original_did() {
        let seq = Sequence::new("x", b"ACGT".to_vec());
        assert!(write(&seq, "01-JAN-2000").contains("SerialCloner_Ends=0,0,,0,\n"));
    }

    #[test]
    fn embl_records_are_accepted() {
        let embl = "ID   X52327; SV 1; circular; genomic DNA; STD; SYN; 12 BP.\nXX\nDE   test record\nXX\nFH   Key             Location/Qualifiers\nFT   CDS             complement(2..7)\nFT                   /gene=\"g\"\nXX\nSQ   Sequence 12 BP;\n     acgtacgtac gt        12\n//\n";
        let seq = &read(embl.as_bytes()).unwrap()[0];
        assert_eq!(seq.name, "X52327");
        assert_eq!(seq.bases, b"acgtacgtacgt");
        assert_eq!(seq.topology, Topology::Circular);
        assert_eq!(seq.features[0].name, "g");
        assert_eq!((seq.features[0].start, seq.features[0].end), (7, 2));
    }

    #[test]
    fn locations() {
        assert_eq!(
            parse_location("complement(2516..3144)", 4361),
            Some((2516, 3144, Strand::Reverse, false))
        );
        assert_eq!(
            parse_location("join(4300..4361,1..50)", 4361),
            Some((4300, 50, Strand::Forward, true))
        );
        assert_eq!(
            parse_location("<1..>100", 4361),
            Some((1, 100, Strand::Forward, false))
        );
        assert_eq!(
            parse_location("join(10..20,30..40)", 4361),
            Some((10, 40, Strand::Forward, false))
        );
        assert_eq!(
            parse_location("2535", 4361),
            Some((2535, 2535, Strand::Forward, false))
        );
        assert_eq!(
            parse_location("141^142", 4361),
            Some((141, 142, Strand::Forward, false))
        );
        assert_eq!(parse_location("garbage", 4361), None);
    }
}
