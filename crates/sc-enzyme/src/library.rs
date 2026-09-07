//! The `striderc` enzyme file and the [`Enzyme`] model.
//!
//! ```text
//!  REBASE version 609                                             striderc.609
//!  ...
//! AatII,gacgt/c,
//! AccI,gt/mkac,
//! #AarI,cacctgc,4,8,
//! #AciI,ccgc,-3,-1,
//! ```
//!
//! A plain entry gives the top-strand cut as `/` inside the site; the bottom
//! strand is cut symmetrically (`len − top`), which is right for the
//! palindromic sites this notation is used for. A `#` entry gives REBASE's
//! `(a/b)` offsets: the top strand is cut `a` bases after the 3' end of the
//! site and the bottom strand `b` bases after it, so negative offsets reach
//! back inside the site — that is how asymmetric type II sites such as AciI
//! `CCGC(-3/-1)` are written.

use std::collections::HashMap;

use sc_seq::alphabet::{is_iupac, is_palindromic};
use serde::{Deserialize, Serialize};

/// Summary of a `striderc` file's banner and size.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryInfo {
    /// The `NNN` of `REBASE version NNN`, when the banner is present.
    pub rebase_release: Option<u32>,
    /// Number of enzyme entry lines.
    pub enzyme_count: usize,
}

/// Reads the banner and counts the entries of a `striderc`-formatted text.
pub fn library_info(text: &str) -> LibraryInfo {
    let mut rebase_release = None;
    let mut enzyme_count = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        if rebase_release.is_none()
            && let Some(rest) = trimmed.strip_prefix("REBASE version")
        {
            rebase_release = rest.split_whitespace().next().and_then(|n| n.parse().ok());
            continue;
        }
        if parse_entry(trimmed).is_some() {
            enzyme_count += 1;
        }
    }
    LibraryInfo {
        rebase_release,
        enzyme_count,
    }
}

/// A restriction enzyme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enzyme {
    /// Name, e.g. `EcoRI`.
    pub name: String,
    /// Recognition site, lower-case IUPAC, no cut mark.
    pub site: Vec<u8>,
    /// Top-strand cut, as an offset from the first base of the site: the cut
    /// lies before `site[cut_top]` (may be negative or beyond the site for
    /// type IIs).
    pub cut_top: i32,
    /// Bottom-strand cut in the same top-strand coordinates.
    pub cut_bottom: i32,
    /// Written with `#` (offset notation) in the file.
    pub type_iis: bool,
}

impl Enzyme {
    /// Signed overhang length: positive = 5' overhang, negative = 3', 0 = blunt.
    pub fn overhang(&self) -> i32 {
        self.cut_bottom - self.cut_top
    }

    /// True if the cut leaves blunt ends.
    pub fn is_blunt(&self) -> bool {
        self.cut_top == self.cut_bottom
    }

    /// True if the site reads the same on both strands, so one search covers
    /// both.
    pub fn is_palindromic(&self) -> bool {
        is_palindromic(&self.site)
    }

    /// The site with the top-strand cut marked, e.g. `g/aattc`, or REBASE's
    /// `cacctgc(4/8)` when the cut falls outside the site.
    pub fn display_site(&self) -> String {
        let site = String::from_utf8_lossy(&self.site);
        let len = self.site.len() as i32;
        if (0..=len).contains(&self.cut_top)
            && self.cut_bottom == len - self.cut_top
            && !self.type_iis
        {
            let (a, b) = site.split_at(self.cut_top as usize);
            format!("{a}/{b}")
        } else {
            format!("{site}({}/{})", self.cut_top - len, self.cut_bottom - len)
        }
    }

    /// The `striderc` line for this enzyme.
    pub fn to_striderc(&self) -> String {
        let site = String::from_utf8_lossy(&self.site);
        let len = self.site.len() as i32;
        if self.type_iis {
            format!(
                "#{},{},{},{},",
                self.name,
                site,
                self.cut_top - len,
                self.cut_bottom - len
            )
        } else {
            let (a, b) = site.split_at(self.cut_top.clamp(0, len) as usize);
            format!("{},{}/{},", self.name, a, b)
        }
    }
}

/// Parses one entry line; `None` for banner, prose or blank lines.
pub fn parse_entry(line: &str) -> Option<Enzyme> {
    let line = line.trim();
    let (type_iis, body) = match line.strip_prefix('#') {
        Some(rest) => (true, rest),
        None => (false, line),
    };
    let fields: Vec<&str> = body.split(',').map(str::trim).collect();
    let name = *fields.first()?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') {
        return None;
    }
    let raw_site = *fields.get(1)?;
    if raw_site.is_empty() {
        return None;
    }
    if type_iis {
        let site: Vec<u8> = raw_site.bytes().map(|b| b.to_ascii_lowercase()).collect();
        if !site.iter().all(|&b| is_iupac(b)) {
            return None;
        }
        let a: i32 = fields.get(2)?.parse().ok()?;
        let b: i32 = fields.get(3)?.parse().ok()?;
        let len = site.len() as i32;
        Some(Enzyme {
            name: name.to_owned(),
            site,
            cut_top: len + a,
            cut_bottom: len + b,
            type_iis: true,
        })
    } else {
        let cut = raw_site.find('/')?;
        let site: Vec<u8> = raw_site
            .bytes()
            .filter(|&b| b != b'/')
            .map(|b| b.to_ascii_lowercase())
            .collect();
        if site.is_empty() || !site.iter().all(|&b| is_iupac(b)) {
            return None;
        }
        let len = site.len() as i32;
        Some(Enzyme {
            name: name.to_owned(),
            site,
            cut_top: cut as i32,
            cut_bottom: len - cut as i32,
            type_iis: false,
        })
    }
}

/// An enzyme library: the parsed file, in file order, with a name index.
#[derive(Debug, Clone, Default)]
pub struct Library {
    /// Banner and count.
    pub info: LibraryInfo,
    /// Enzymes in file order.
    pub enzymes: Vec<Enzyme>,
    index: HashMap<String, usize>,
}

impl Library {
    /// Parses a `striderc` file.
    pub fn parse(text: &str) -> Library {
        let enzymes: Vec<Enzyme> = text.lines().filter_map(parse_entry).collect();
        let index = enzymes
            .iter()
            .enumerate()
            .map(|(i, e)| (e.name.to_ascii_lowercase(), i))
            .collect();
        Library {
            info: library_info(text),
            enzymes,
            index,
        }
    }

    /// Looks an enzyme up by name, case-insensitively.
    pub fn get(&self, name: &str) -> Option<&Enzyme> {
        self.index
            .get(&name.to_ascii_lowercase())
            .map(|&i| &self.enzymes[i])
    }

    /// Number of enzymes.
    pub fn len(&self) -> usize {
        self.enzymes.len()
    }

    /// True if the library has no enzymes.
    pub fn is_empty(&self) -> bool {
        self.enzymes.is_empty()
    }

    /// Enzymes whose recognition site is identical (Enzyme Library ▸ find
    /// isoschizomers), excluding the enzyme itself.
    pub fn isoschizomers(&self, name: &str) -> Vec<&Enzyme> {
        let Some(e) = self.get(name) else {
            return Vec::new();
        };
        self.enzymes
            .iter()
            .filter(|o| o.site == e.site && o.name != e.name)
            .collect()
    }

    /// Enzymes leaving ends that ligate to this one's: same overhang length
    /// and sense, and — for cuts inside the site — the same protruding
    /// bases. Type IIs overhangs depend on the flanking sequence and are only
    /// matched on length here.
    pub fn compatible_ends(&self, name: &str) -> Vec<&Enzyme> {
        let Some(e) = self.get(name) else {
            return Vec::new();
        };
        let ov = e.overhang();
        self.enzymes
            .iter()
            .filter(|o| {
                o.name != e.name
                    && o.overhang() == ov
                    && (ov == 0 || protruding(o) == protruding(e))
            })
            .collect()
    }

    /// Serialises back to `striderc` (banner + one line per enzyme).
    pub fn to_striderc(&self) -> String {
        let mut out = String::new();
        if let Some(v) = self.info.rebase_release {
            out.push_str(&format!(" REBASE version {v}   striderc.{v}\n\n"));
        }
        for e in &self.enzymes {
            out.push_str(&e.to_striderc());
            out.push('\n');
        }
        out
    }
}

/// The overhang bases when the cut lies inside the site; `None` otherwise.
fn protruding(e: &Enzyme) -> Option<Vec<u8>> {
    let len = e.site.len() as i32;
    let (a, b) = (e.cut_top.min(e.cut_bottom), e.cut_top.max(e.cut_bottom));
    if a >= 0 && b <= len {
        Some(e.site[a as usize..b as usize].to_vec())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

    fn shipped() -> Library {
        Library::parse(&std::fs::read_to_string(format!("{REPO}/resources/RELibrary")).unwrap())
    }

    #[test]
    fn parses_type_ii_entries() {
        let e = parse_entry("EcoRI,g/aattc,").unwrap();
        assert_eq!(
            (e.site.as_slice(), e.cut_top, e.cut_bottom, e.type_iis),
            (&b"gaattc"[..], 1, 5, false)
        );
        assert_eq!(e.overhang(), 4);
        assert!(e.is_palindromic());
        assert_eq!(e.display_site(), "g/aattc");
        let blunt = parse_entry("SmaI,ccc/ggg,").unwrap();
        assert!(blunt.is_blunt());
        let three = parse_entry("PstI,ctgca/g,").unwrap();
        assert_eq!(three.overhang(), -4);
        let degenerate = parse_entry("AccI,gt/mkac,").unwrap();
        assert_eq!(degenerate.overhang(), 2);
        assert!(degenerate.is_palindromic());
    }

    #[test]
    fn parses_type_iis_entries_with_rebase_offsets() {
        let e = parse_entry("#AarI,cacctgc,4,8,").unwrap();
        assert_eq!((e.cut_top, e.cut_bottom, e.overhang()), (11, 15, 4));
        assert!(!e.is_palindromic());
        assert_eq!(e.display_site(), "cacctgc(4/8)");
        assert_eq!(e.to_striderc(), "#AarI,cacctgc,4,8,");
        // AciI C^CGC is asymmetric, so REBASE writes it with negative offsets.
        let aci = parse_entry("#AciI,ccgc,-3,-1,").unwrap();
        assert_eq!((aci.cut_top, aci.cut_bottom, aci.overhang()), (1, 3, 2));
        // BsmI GAATGC(1/-1): 3' overhang of 2.
        let bsm = parse_entry("#BsaMI,gaatgc,1,-1,").unwrap();
        assert_eq!((bsm.cut_top, bsm.cut_bottom, bsm.overhang()), (7, 5, -2));
    }

    #[test]
    fn ignores_banner_and_prose() {
        assert!(parse_entry(" REBASE version 609   striderc.609").is_none());
        assert!(parse_entry("Copyright (c) Dr. Richard J. Roberts, 2024.").is_none());
        assert!(parse_entry("").is_none());
        assert!(parse_entry("Name,").is_none());
        assert!(parse_entry("Name,nocut,").is_none());
    }

    #[test]
    fn shipped_library_parses_completely() {
        let lib = shipped();
        assert_eq!(lib.info.enzyme_count, lib.len());
        assert!(lib.len() >= 700, "only {} enzymes", lib.len());
        assert_eq!(lib.enzymes.iter().filter(|e| e.type_iis).count(), 202);
        let eco = lib.get("ecori").unwrap();
        assert_eq!(eco.name, "EcoRI");
        assert_eq!(eco.display_site(), "g/aattc");
        assert!(lib.get("nothing").is_none());
        // Round trip through our own writer.
        let again = Library::parse(&lib.to_striderc());
        assert_eq!(again.enzymes, lib.enzymes);
        assert_eq!(again.info.rebase_release, lib.info.rebase_release);
    }

    #[test]
    fn serial_cloner_2_6_1_shipped_rebase_211() {
        let text =
            std::fs::read_to_string(format!("{REPO}/reference/RELibrary-striderc.211")).unwrap();
        let lib = Library::parse(&text);
        assert_eq!(lib.info.rebase_release, Some(211));
        assert!(lib.len() >= 700);
        assert!(lib.get("XmnI").is_some());
    }

    #[test]
    fn isoschizomers_and_compatible_ends() {
        let lib = shipped();
        let iso: Vec<_> = lib
            .isoschizomers("SmaI")
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(iso.contains(&"XmaI"), "{iso:?}");
        let compat: Vec<_> = lib
            .compatible_ends("BamHI")
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(compat.contains(&"BglII"), "{compat:?}");
        assert!(compat.contains(&"BclI"), "{compat:?}");
        assert!(!compat.contains(&"EcoRI"));
    }
}
