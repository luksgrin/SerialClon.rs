//! Import and export of feature collections (Features ReadMe §IX.A.b–c).
//!
//! - **SerialClon**: the JSON form of [`Collection`].
//! - **Generic table**: one feature per line, tab-separated:
//!   `name<TAB>sequence<TAB>type<TAB>colour<TAB>DNA|Peptide` (the last two
//!   optional; `#` lines are comments).
//! - **ApE** (`Default_Features.txt`): `name<TAB>sequence<TAB>type<TAB>
//!   forward colour<TAB>reverse colour<TAB>…`.

use crate::{Collection, FeatureDef, color_hex, parse_color};

/// A collection format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// SerialClon JSON.
    SerialClon,
    /// Generic tab-separated table.
    Table,
    /// ApE feature file.
    Ape,
}

/// Sniffs the format.
pub fn detect(text: &str) -> Format {
    let t = text.trim_start();
    if t.starts_with('{') {
        Format::SerialClon
    } else {
        let tabs = t
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.matches('\t').count())
            .unwrap_or(0);
        if tabs >= 4 {
            Format::Ape
        } else {
            Format::Table
        }
    }
}

/// Parses a collection in any format, naming it `name` when the format
/// carries no name.
pub fn read(text: &str, name: &str) -> Result<Collection, String> {
    match detect(text) {
        Format::SerialClon => Collection::from_json(text).map_err(|e| e.to_string()),
        Format::Table | Format::Ape => {
            let mut features = Vec::new();
            for line in text.lines() {
                let line = line.trim_end();
                if line.trim().is_empty() || line.starts_with('#') {
                    continue;
                }
                let cols: Vec<&str> = line.split('\t').map(str::trim).collect();
                if cols.len() < 2 || cols[1].is_empty() {
                    continue;
                }
                let sequence = cols[1];
                let peptide = cols.get(4).is_some_and(|c| {
                    c.eq_ignore_ascii_case("peptide") || c.eq_ignore_ascii_case("protein")
                }) || (!sequence
                    .bytes()
                    .all(|b| sc_seq::alphabet::is_iupac(b) || b.is_ascii_whitespace())
                    && sequence.bytes().any(|b| b.is_ascii_alphabetic()));
                let mut def = FeatureDef::simple(
                    cols[0],
                    cols.get(2)
                        .filter(|k| !k.is_empty())
                        .unwrap_or(&"misc_feature"),
                    "#84a4c0",
                    sequence,
                );
                def.peptide = peptide;
                if let Some(c) = cols.get(3).and_then(|c| parse_color(c)) {
                    def.color = color_hex(c);
                }
                features.push(def);
            }
            if features.is_empty() {
                return Err("no features found in this file".into());
            }
            Ok(Collection {
                name: name.to_owned(),
                scan: true,
                builtin: false,
                features,
            })
        }
    }
}

/// Writes a collection as a generic table.
pub fn write_table(c: &Collection) -> String {
    let mut out = String::from("# name\tsequence\ttype\tcolour\tDNA|Peptide\n");
    for f in &c.features {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            f.name,
            f.five,
            f.kind,
            f.color,
            if f.peptide { "Peptide" } else { "DNA" }
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_round_trip_and_ape() {
        let text = "# c\nAmpR\tATGAGT\tCDS\t#ff0000\tDNA\nHA\tYPYDVPDYA\tmisc_feature\t\tPeptide\n";
        let c = read(text, "mine").unwrap();
        assert_eq!(c.name, "mine");
        assert_eq!(c.features.len(), 2);
        assert_eq!(c.features[0].color, "#ff0000");
        assert!(c.features[1].peptide);
        let back = read(&write_table(&c), "again").unwrap();
        assert_eq!(back.features, c.features);
        let ape = "T7\tTAATACGACTCACTATAGGG\tpromoter\t#00ffff\t#ff00ff\t1\n";
        assert_eq!(detect(ape), Format::Ape);
        let c = read(ape, "ape").unwrap();
        assert_eq!(c.features[0].kind, "promoter");
        assert_eq!(c.features[0].color, "#00ffff");
        assert!(!c.features[0].peptide);
        let json = c.to_json();
        assert_eq!(detect(&json), Format::SerialClon);
        assert_eq!(read(&json, "x").unwrap(), c);
    }
}
