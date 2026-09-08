//! Codon usage tables (changelog 2.5: reverse translation with a chosen
//! table, Manage Codon Usage, Import Codon Usage from the web; 2.6: use a
//! table's codon → amino-acid assignments as the genetic code).
//!
//! Tables are stored as JSON (`resources/codon-usage/*.json`, one per
//! species, with the source URL and retrieval date). Imports accept the two
//! text layouts of the Kazusa Codon Usage Database and the GCG
//! `CodonFrequency` layout.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::translate::{GeneticCode, STANDARD, codon_index};

/// One codon's usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodonUsage {
    /// `ATG`-style, DNA alphabet.
    pub codon: String,
    /// The amino acid it encodes (standard code when the source has none).
    #[serde(default)]
    pub amino_acid: String,
    /// Fraction among the synonymous codons of that amino acid.
    #[serde(default)]
    pub fraction: f64,
    /// Occurrences per thousand codons.
    pub per_thousand: f64,
    /// Raw count in the source.
    #[serde(default)]
    pub count: u64,
}

/// A codon usage table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodonTable {
    /// Species / label.
    pub name: String,
    /// NCBI taxid when known.
    #[serde(default)]
    pub taxid: u32,
    /// File-name-safe id.
    #[serde(default)]
    pub slug: String,
    /// Where it came from.
    #[serde(default)]
    pub source: String,
    /// Retrieval date.
    #[serde(default)]
    pub retrieved: String,
    /// Number of coding sequences behind the counts.
    #[serde(default)]
    pub cds: Option<u64>,
    /// Total codons behind the counts.
    #[serde(default)]
    pub codons_total: Option<u64>,
    /// Shipped with the application.
    #[serde(default)]
    pub builtin: bool,
    /// The 64 entries.
    pub codons: Vec<CodonUsage>,
}

/// Errors from parsing a table.
#[derive(Debug, thiserror::Error)]
pub enum CodonError {
    /// Nothing that looks like a table.
    #[error("no codon usage table found in the text ({0} codons recognised, 64 expected)")]
    Incomplete(usize),
    /// Bad JSON.
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

fn is_codon(t: &str) -> bool {
    t.len() == 3 && t.chars().all(|c| "ACGTU".contains(c.to_ascii_uppercase()))
}

fn number(t: &str) -> Option<f64> {
    t.trim_matches(|c| c == '(' || c == ')').parse::<f64>().ok()
}

impl CodonTable {
    /// Parses the JSON form.
    pub fn from_json(text: &str) -> Result<CodonTable, CodonError> {
        Ok(serde_json::from_str(text)?)
    }

    /// The JSON form.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Parses Kazusa text (`UUU 22.4( 22308)` or `UUU F 0.57 19.7 ( 101)`,
    /// HTML tags tolerated) or GCG `CodonFrequency` text
    /// (`Gly GGG 1743.00 24.35 0.19`). Amino acids and fractions are filled
    /// in from the standard code when the layout lacks them.
    pub fn parse_text(text: &str, name: &str) -> Result<CodonTable, CodonError> {
        let plain = strip_tags(text);
        let tokens: Vec<&str> = plain.split_whitespace().collect();
        let mut entries: HashMap<String, CodonUsage> = HashMap::new();
        let mut i = 0;
        while i < tokens.len() {
            if !is_codon(tokens[i]) {
                i += 1;
                continue;
            }
            let codon = tokens[i].to_ascii_uppercase().replace('U', "T");
            let mut nums: Vec<f64> = Vec::new();
            let mut aa = String::new();
            let mut j = i + 1;
            while j < tokens.len() && j <= i + 5 && !is_codon(tokens[j]) {
                let tk = tokens[j];
                if tk.len() == 1
                    && (tk.as_bytes()[0].is_ascii_uppercase() || tk == "*")
                    && nums.is_empty()
                {
                    aa = tk.to_owned();
                } else if let Some(n) = number(tk) {
                    nums.push(n);
                } else if tk != "(" && !tk.starts_with('(') {
                    break;
                }
                j += 1;
            }
            if !nums.is_empty() {
                let (fraction, per_thousand, count) = match nums.len() {
                    // Kazusa "style N": fraction, per thousand, (count)
                    3 if nums[0] <= 1.0 => (nums[0], nums[1], nums[2] as u64),
                    // GCG: number, /1000, fraction
                    3 => (nums[2], nums[1], nums[0] as u64),
                    // Kazusa compact: per thousand (count)
                    2 => (0.0, nums[0], nums[1] as u64),
                    _ => (0.0, nums[0], 0),
                };
                entries.insert(
                    codon.clone(),
                    CodonUsage {
                        codon,
                        amino_acid: aa,
                        fraction,
                        per_thousand,
                        count,
                    },
                );
            }
            i = j.max(i + 1);
        }
        if entries.len() < 64 {
            return Err(CodonError::Incomplete(entries.len()));
        }
        let mut codons: Vec<CodonUsage> = entries.into_values().collect();
        codons.sort_by_key(|c| codon_index(c.codon.as_bytes()));
        let mut table = CodonTable {
            name: name.to_owned(),
            taxid: 0,
            slug: slug(name),
            source: String::new(),
            retrieved: String::new(),
            cds: None,
            codons_total: None,
            builtin: false,
            codons,
        };
        table.fill_in();
        Ok(table)
    }

    /// Fills missing amino acids (standard code) and fractions.
    pub fn fill_in(&mut self) {
        for c in &mut self.codons {
            if c.amino_acid.is_empty() {
                c.amino_acid = (STANDARD.amino_acid(c.codon.as_bytes()) as char).to_string();
            }
        }
        let mut totals: HashMap<String, f64> = HashMap::new();
        for c in &self.codons {
            *totals.entry(c.amino_acid.clone()).or_default() += c.per_thousand;
        }
        for c in &mut self.codons {
            if c.fraction == 0.0 {
                let t = totals.get(&c.amino_acid).copied().unwrap_or(0.0);
                c.fraction = if t > 0.0 {
                    (c.per_thousand / t * 100.0).round() / 100.0
                } else {
                    0.0
                };
            }
        }
    }

    /// The most used codon of each amino acid (what reverse translation
    /// picks; changelog 2.6 fixed "the selection of the most used AA").
    pub fn preferred_codons(&self) -> HashMap<u8, String> {
        let mut best: HashMap<u8, (f64, String)> = HashMap::new();
        for c in &self.codons {
            let Some(&aa) = c.amino_acid.as_bytes().first() else {
                continue;
            };
            let entry = best
                .entry(aa.to_ascii_uppercase())
                .or_insert((-1.0, String::new()));
            if c.per_thousand > entry.0 {
                *entry = (c.per_thousand, c.codon.clone());
            }
        }
        best.into_iter()
            .map(|(aa, (_, codon))| (aa, codon))
            .collect()
    }

    /// Reverse-translates a peptide with the most used codon of each amino
    /// acid; `*` becomes the most used stop, unknown letters `NNN`.
    pub fn reverse_translate(&self, peptide: &[u8]) -> Vec<u8> {
        let preferred = self.preferred_codons();
        let mut out = Vec::with_capacity(peptide.len() * 3);
        for &aa in peptide {
            if !aa.is_ascii_alphabetic() && aa != b'*' {
                continue;
            }
            match preferred.get(&aa.to_ascii_uppercase()) {
                Some(c) => out.extend_from_slice(c.as_bytes()),
                None => out.extend_from_slice(b"NNN"),
            }
        }
        out
    }

    /// The table's own codon → amino-acid assignments as a genetic code
    /// (2.6: "use the Codon code from Codon Usage tables as a Genetic code").
    pub fn genetic_code(&self) -> GeneticCode {
        let mut code = GeneticCode {
            id: 0,
            name: "Codon usage table",
            amino_acids: STANDARD.amino_acids,
            starts: STANDARD.starts,
        };
        for c in &self.codons {
            if let (Some(i), Some(&aa)) = (
                codon_index(c.codon.as_bytes()),
                c.amino_acid.as_bytes().first(),
            ) {
                code.amino_acids[i] = aa.to_ascii_uppercase();
            }
        }
        code
    }
}

fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
}

/// A file-name-safe id from a name.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_matches('-').to_owned();
    if s.is_empty() { "table".into() } else { s }
}

/// The tables compiled into the application (Kazusa, see each `source`).
pub fn builtin_tables() -> Vec<CodonTable> {
    [
        include_str!("../../../resources/codon-usage/ecoli-k12.json"),
        include_str!("../../../resources/codon-usage/yeast.json"),
        include_str!("../../../resources/codon-usage/human.json"),
        include_str!("../../../resources/codon-usage/mouse.json"),
        include_str!("../../../resources/codon-usage/drosophila.json"),
    ]
    .iter()
    .filter_map(|t| CodonTable::from_json(t).ok())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tables_load_and_round_trip_a_peptide() {
        let tables = builtin_tables();
        assert_eq!(tables.len(), 5);
        for t in &tables {
            assert_eq!(t.codons.len(), 64, "{}", t.name);
            assert!(t.source.contains("kazusa"), "{}", t.name);
            assert!(!t.retrieved.is_empty());
            let total: f64 = t.codons.iter().map(|c| c.per_thousand).sum();
            assert!((total - 1000.0).abs() < 2.0, "{}: {total}", t.name);
            let peptide = b"MSTFKVLLCGAVLSRIDAGQEQLGRRIHYSQNDLVEYSPVTEKHLTDGMTVRELCSAAITMSDNTAANLLLTTIGGPKELTAFLHNMGDHVTRLDRWEPELNEAIPNDERDTTMPVAMATTLRKLLTGELLTLASRQQLIDWMEADKVAGPLLRSALPAGWFIADKSGAGERGSRGIIAALGPDGKPSRIVVIYTTGSQATMDERNRQIAEIGASLIKHW*";
            let dna = t.reverse_translate(peptide);
            assert_eq!(dna.len(), peptide.len() * 3);
            assert_eq!(
                STANDARD.translate(&dna, true).as_bytes(),
                peptide,
                "{}",
                t.name
            );
        }
        // E. coli prefers CTG for leucine and CG* for arginine.
        let pref = tables[0].preferred_codons();
        assert_eq!(pref[&b'L'], "CTG");
        assert!(pref[&b'R'].starts_with("CG"));
    }

    #[test]
    fn parses_kazusa_and_gcg_layouts() {
        let ecoli = &builtin_tables()[0];
        let ctg_pt = ecoli
            .codons
            .iter()
            .find(|c| c.codon == "CTG")
            .unwrap()
            .per_thousand;
        let full = ecoli
            .codons
            .iter()
            .map(|c| {
                format!(
                    "{} {} {:.2} {:.1} ( {})",
                    c.codon.replace('T', "U"),
                    c.amino_acid,
                    c.fraction,
                    c.per_thousand,
                    c.count
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let t = CodonTable::parse_text(&format!("<PRE>{full}</PRE>"), "test").unwrap();
        assert_eq!(t.codons.len(), 64);
        assert_eq!(
            t.codons
                .iter()
                .find(|c| c.codon == "CTG")
                .unwrap()
                .amino_acid,
            "L"
        );
        assert!(matches!(
            CodonTable::parse_text(
                "UUU 22.4( 22308)  UCU  8.5(  8489)  UAU 16.3( 16188)  UGU  5.2(  5148)\n",
                "x"
            ),
            Err(CodonError::Incomplete(4))
        ));
        let compact = ecoli
            .codons
            .iter()
            .map(|c| {
                format!(
                    "{} {:.1}( {})",
                    c.codon.replace('T', "U"),
                    c.per_thousand,
                    c.count
                )
            })
            .collect::<Vec<_>>()
            .join("  ");
        let t = CodonTable::parse_text(&compact, "compact").unwrap();
        assert_eq!(
            t.codons
                .iter()
                .find(|c| c.codon == "ATG")
                .unwrap()
                .amino_acid,
            "M"
        );
        assert!(
            t.codons
                .iter()
                .all(|c| c.fraction > 0.0 || c.per_thousand == 0.0)
        );
        let gcg = ecoli
            .codons
            .iter()
            .map(|c| {
                format!(
                    "Xxx {} {:.2} {:.2} {:.2}",
                    c.codon, c.count as f64, c.per_thousand, c.fraction
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let t = CodonTable::parse_text(&gcg, "gcg").unwrap();
        let ctg = t.codons.iter().find(|c| c.codon == "CTG").unwrap();
        assert!((ctg.per_thousand - ctg_pt).abs() < 0.01);
    }

    #[test]
    fn genetic_code_from_table_and_json_round_trip() {
        let t = &builtin_tables()[2];
        let code = t.genetic_code();
        assert_eq!(code.amino_acid(b"ATG"), b'M');
        assert_eq!(code.amino_acid(b"TAA"), b'*');
        let back = CodonTable::from_json(&t.to_json()).unwrap();
        assert_eq!(&back, t);
        assert_eq!(
            slug("Escherichia coli K-12 (W3110)"),
            "escherichia-coli-k-12-w3110"
        );
    }
}
