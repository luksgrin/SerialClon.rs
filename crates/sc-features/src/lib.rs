//! Feature annotations for SerialClon.rs (Features ReadMe v2.1).
//!
//! [`Collection`]s of [`FeatureDef`]s are scanned against a sequence by
//! [`scan`]: a *simple* definition is one consensus (DNA with IUPAC codes, or
//! a peptide matched against the six-frame translation); a *complex* one has
//! 5' and 3' consensus ends with anything between, optional 5'/3' pieces that
//! are included when present (the EGFP start and stop codons), and upstream /
//! downstream context that must be present but is not part of the feature.
//! [`merge`] applies a scan the way the original did: unprotected hits are
//! replaced, user-edited (protected) ones are kept. [`formats`] reads and
//! writes collections (SerialClon JSON, generic table, ApE).

pub mod formats;
pub mod scan;

use sc_seq::{Feature, Rgb};
use serde::{Deserialize, Serialize};

pub use scan::{merge, scan, scan_one};

/// A feature definition to scan for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureDef {
    /// Name given to found features.
    pub name: String,
    /// GenBank key (`CDS`, `promoter`, `misc_feature`…).
    pub kind: String,
    /// Colour as `#rrggbb`.
    pub color: String,
    /// Found features get an arrow.
    #[serde(default = "yes")]
    pub arrow: bool,
    /// Only look on the sense strand.
    #[serde(default)]
    pub sense_only: bool,
    /// Consensus is a peptide (matched against the translation).
    #[serde(default)]
    pub peptide: bool,
    /// 5' consensus (or the whole consensus for a simple definition).
    pub five: String,
    /// 3' consensus (complex definition), empty for simple.
    #[serde(default)]
    pub three: String,
    /// Included if present immediately 5' of the match.
    #[serde(default)]
    pub potential_five: String,
    /// Included if present immediately 3' of the match.
    #[serde(default)]
    pub potential_three: String,
    /// Must precede the match; not part of the feature.
    #[serde(default)]
    pub upstream: String,
    /// Must follow the match; not part of the feature.
    #[serde(default)]
    pub downstream: String,
    /// Free text, copied into the feature description.
    #[serde(default)]
    pub comment: String,
    /// Use when scanning.
    #[serde(default = "yes")]
    pub scan: bool,
}

fn yes() -> bool {
    true
}

impl FeatureDef {
    /// A simple DNA definition.
    pub fn simple(name: &str, kind: &str, color: &str, consensus: &str) -> Self {
        FeatureDef {
            name: name.into(),
            kind: kind.into(),
            color: color.into(),
            arrow: true,
            sense_only: false,
            peptide: false,
            five: consensus.into(),
            three: String::new(),
            potential_five: String::new(),
            potential_three: String::new(),
            upstream: String::new(),
            downstream: String::new(),
            comment: String::new(),
            scan: true,
        }
    }

    /// True for a complex definition (a 3' end or any context is given).
    pub fn is_complex(&self) -> bool {
        !(self.three.is_empty()
            && self.potential_five.is_empty()
            && self.potential_three.is_empty()
            && self.upstream.is_empty()
            && self.downstream.is_empty())
    }

    /// The colour as RGB (grey when unparsable).
    pub fn rgb(&self) -> Rgb {
        parse_color(&self.color).unwrap_or(Rgb(132, 164, 192))
    }
}

/// Parses `#rrggbb` or `r,g,b`.
pub fn parse_color(text: &str) -> Option<Rgb> {
    let t = text.trim();
    if let Some(h) = t
        .strip_prefix('#')
        .or_else(|| t.strip_prefix("&h"))
        .or_else(|| t.strip_prefix("&H"))
        && h.len() == 6
        && let Ok(n) = u32::from_str_radix(h, 16)
    {
        return Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8));
    }
    let parts: Vec<u8> = t
        .split(',')
        .filter(|p| !p.trim().is_empty())
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    match parts[..] {
        [r, g, b] => Some(Rgb(r, g, b)),
        _ => None,
    }
}

/// Formats an RGB colour as `#rrggbb`.
pub fn color_hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// A named collection of definitions (Generic, Yeast, Other, Tags & Cut,
/// Gateway, or the user's own).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    /// Name.
    pub name: String,
    /// Use when scanning.
    #[serde(default = "yes")]
    pub scan: bool,
    /// Shipped with the application.
    #[serde(default)]
    pub builtin: bool,
    /// The definitions.
    pub features: Vec<FeatureDef>,
}

impl Collection {
    /// Parses the JSON form.
    pub fn from_json(text: &str) -> Result<Collection, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// The JSON form.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// The five collections compiled into the application.
pub fn builtin_collections() -> Vec<Collection> {
    [
        include_str!("../../../resources/features/generic.json"),
        include_str!("../../../resources/features/yeast.json"),
        include_str!("../../../resources/features/other.json"),
        include_str!("../../../resources/features/tags-and-cut.json"),
        include_str!("../../../resources/features/gateway.json"),
    ]
    .iter()
    .filter_map(|t| Collection::from_json(t).ok())
    .collect()
}

/// A feature made from a definition and a match.
pub fn feature_from(def: &FeatureDef, start: u32, end: u32, strand: sc_seq::Strand) -> Feature {
    Feature {
        name: def.name.clone(),
        description: def.comment.clone(),
        kind: def.kind.clone(),
        start,
        end,
        strand,
        show: true,
        protect: false,
        arrow: def.arrow,
        color: def.rgb(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_collections_load() {
        let c = builtin_collections();
        assert_eq!(c.len(), 5);
        let names: Vec<&str> = c.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            ["Generic", "Yeast", "Other", "Tags & Cut", "Gateway"]
        );
        assert!(c[0].features.iter().any(|f| f.name == "AmpR"));
        assert!(c[3].features.iter().all(|f| f.peptide));
        let egfp = c[0].features.iter().find(|f| f.name == "EGFP").unwrap();
        assert!(egfp.is_complex());
        assert_eq!(
            (egfp.potential_five.as_str(), egfp.potential_three.as_str()),
            ("ATG", "TAA")
        );
    }

    #[test]
    fn colours() {
        assert_eq!(parse_color("#4682b4"), Some(Rgb(70, 130, 180)));
        assert_eq!(parse_color("70,130,180,"), Some(Rgb(70, 130, 180)));
        assert_eq!(parse_color("&h4682B4"), Some(Rgb(70, 130, 180)));
        assert_eq!(color_hex(Rgb(70, 130, 180)), "#4682b4");
    }
}
