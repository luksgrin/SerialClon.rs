//! The Sequence Map: the text restriction map with multi-frame translation
//! (User Manual §IV.D), laid out exactly as the original printed it — the
//! block structure, label stacking, `>`/`<` orientation arrows, coordinate
//! line and Features legend follow `reference/docs/pBluescript-II-SK+-unique-sites-map.txt`.
//!
//! ```text
//!                                    BsaAI
//!                     >BtgZI   DraIII       >F1 ori
//!                     |        | |          |
//!   CCGTCTATCAGGGCGATGGCCCACTACGTGAACCATCACCC…   < 300
//!    R  L  S  G  R  W  P  T  T  *  T  I  T  L  I  K  F  F  G …      (frame 1)
//!     V  Y  Q  G  D  G  P  L  R  E  P  S  P  *  S  S  F  L  G …     (frame 2)
//!   P  S  I  R  A  M  A  H  Y  V  N  H  H  P  N  Q  V  F  W  G …    (frame 3)
//!   GGCAGATAGTCCCGCTACCGGGTGATGCACTTGGTAGTGGGATTAG…
//!            210       220       230       240       250 …
//! ```

use sc_seq::translate::{GeneticCode, STANDARD};
use sc_seq::{Sequence, Strand};

use crate::{Library, find_sites};

/// Which enzymes to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteFilter {
    /// "Do not show RE sites".
    None,
    /// "Show all sites".
    All,
    /// "Show RE cutting maximum n times".
    MaxCuts(usize),
    /// A chosen list of enzyme names.
    Particular(Vec<String>),
}

/// Which part to translate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Translate {
    /// No translation lines.
    None,
    /// "Translate all".
    All,
    /// "Translate From … To …", 1-based inclusive.
    Range(usize, usize),
}

/// The Sequence Map's four control panels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMapOptions {
    /// Bases per line (Display panel, default 100).
    pub per_line: usize,
    /// Display ▸ Show Reverse strand.
    pub show_reverse: bool,
    /// Display ▸ Coord.
    pub show_coords: bool,
    /// Translation panel.
    pub translate: Translate,
    /// Frames 1, 2, 3.
    pub frames: [bool; 3],
    /// Codon mode (one frame): bases grouped in codons.
    pub codons: bool,
    /// Only translate upper-case bases (2.6 option).
    pub uppercase_only: bool,
    /// Put the amino acid under the middle base of its codon instead of the
    /// first (Preferences, changelog 2.5).
    pub aa_middle: bool,
    /// Restriction sites panel.
    pub sites: SiteFilter,
    /// Features checkbox.
    pub features: bool,
}

impl Default for TextMapOptions {
    fn default() -> Self {
        TextMapOptions {
            per_line: 100,
            show_reverse: true,
            show_coords: true,
            translate: Translate::All,
            frames: [true, true, true],
            codons: false,
            uppercase_only: false,
            aa_middle: false,
            sites: SiteFilter::MaxCuts(1),
            features: true,
        }
    }
}

/// The three header lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    /// e.g. `SerialClon 0.1.0`.
    pub app: String,
    /// Date and time as text.
    pub date: String,
    /// Library name, e.g. `RELibrary`.
    pub library: String,
}

/// A label to stack above a block: column (0-based within the block), text.
struct Mark {
    col: usize,
    text: String,
}

const INDENT: usize = 2;

/// Places labels in rows above the sequence: each goes to the lowest row
/// where it fits with one blank column on each side. Returns the rows from
/// top to bottom.
fn stack(marks: &[Mark], width: usize) -> Vec<String> {
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let mut order: Vec<&Mark> = marks.iter().collect();
    order.sort_by(|a, b| {
        a.col.cmp(&b.col).then_with(|| {
            a.text
                .to_ascii_lowercase()
                .cmp(&b.text.to_ascii_lowercase())
        })
    });
    for m in order {
        let lo = m.col.saturating_sub(1);
        let hi = m.col + m.text.len() + 1;
        let row = rows
            .iter()
            .position(|r| {
                r[lo.min(r.len())..hi.min(r.len())]
                    .iter()
                    .all(|&c| c == b' ')
            })
            .unwrap_or_else(|| {
                rows.push(vec![b' '; width.max(hi) + 1]);
                rows.len() - 1
            });
        let r = &mut rows[row];
        if r.len() < hi + 1 {
            r.resize(hi + 1, b' ');
        }
        r[m.col..m.col + m.text.len()].copy_from_slice(m.text.as_bytes());
    }
    rows.iter()
        .rev()
        .map(|r| String::from_utf8_lossy(r).trim_end().to_owned())
        .collect()
}

/// Column mapping: in codon mode the chosen frame's codons are separated by
/// a space, so display columns run ahead of base offsets.
struct Columns {
    codon_frame: Option<usize>,
    block_start: usize,
}

impl Columns {
    fn col(&self, global: usize) -> usize {
        let offset = global - self.block_start;
        match self.codon_frame {
            None => offset,
            Some(f) => {
                // One space before every codon start of frame f after the block start.
                let first = self.block_start;
                let boundaries = (first + 1..=global).filter(|&g| g % 3 == f).count();
                offset + boundaries
            }
        }
    }
}

/// Renders the map.
pub fn render(
    seq: &Sequence,
    library: &Library,
    opts: &TextMapOptions,
    header: Option<&Header>,
) -> String {
    render_with_code(seq, library, opts, header, &STANDARD)
}

/// Renders the map with a specific genetic code.
pub fn render_with_code(
    seq: &Sequence,
    library: &Library,
    opts: &TextMapOptions,
    header: Option<&Header>,
    code: &GeneticCode,
) -> String {
    let n = seq.len();
    let per_line = opts.per_line.clamp(10, 1000);
    let mut out = String::new();

    if let Some(h) = header {
        out.push_str(&format!("<{}> -- <{}>\n", h.app, h.date));
        out.push_str(&format!("Restriction map of {}\n", seq.name));
        let what = match &opts.sites {
            SiteFilter::None => "Not showing restriction sites".to_owned(),
            SiteFilter::All => "Showing all restriction sites".to_owned(),
            SiteFilter::MaxCuts(k) => format!(
                "Showing restriction enzymes cutting maximum {k} time{}",
                if *k == 1 { "" } else { "s" }
            ),
            SiteFilter::Particular(names) => {
                format!("Showing particular sites: {}", names.join(", "))
            }
        };
        out.push_str(&format!(
            "{what} [using {} as a Restriction Enzyme Library]\n###\n\n",
            h.library
        ));
    }

    // Every site to show, as (position, label).
    let mut site_marks: Vec<(usize, String)> = Vec::new();
    match &opts.sites {
        SiteFilter::None => {}
        filter => {
            for enzyme in &library.enzymes {
                if let SiteFilter::Particular(names) = filter
                    && !names.iter().any(|x| x.eq_ignore_ascii_case(&enzyme.name))
                {
                    continue;
                }
                let sites = find_sites(&seq.bases, seq.topology, enzyme);
                if let SiteFilter::MaxCuts(k) = filter
                    && (sites.is_empty() || sites.len() > *k)
                {
                    continue;
                }
                let oriented = enzyme.is_oriented();
                for s in sites {
                    let arrow = if !oriented {
                        ""
                    } else if s.strand == Strand::Forward {
                        ">"
                    } else {
                        "<"
                    };
                    site_marks.push((s.position, format!("{arrow}{}", enzyme.name)));
                }
            }
        }
    }
    // Features are labelled at their midpoint, with their direction.
    let mut feature_marks: Vec<(usize, String)> = Vec::new();
    if opts.features {
        for f in &seq.features {
            if !f.show || f.start == 0 || f.start as usize > n || f.end == 0 || f.end as usize > n {
                continue;
            }
            let (lo, _, wraps) = f.lo_hi();
            let len = f.len(n as u32) as usize;
            let mid = if wraps {
                (lo as usize - 1 + len / 2) % n
            } else {
                lo as usize - 1 + len / 2
            };
            let arrow = if f.strand == Strand::Forward {
                ">"
            } else {
                "<"
            };
            feature_marks.push((mid, format!("{arrow}{}", f.name)));
        }
    }

    let single_frame = opts.frames.iter().filter(|&&f| f).count() == 1;
    let codon_frame = if opts.codons && single_frame {
        opts.frames.iter().position(|&f| f)
    } else {
        None
    };
    let (tr_from, tr_to) = match opts.translate {
        Translate::None => (0, 0),
        Translate::All => (0, n),
        Translate::Range(a, b) => (
            a.saturating_sub(1).min(n),
            b.min(n).max(a.saturating_sub(1).min(n)),
        ),
    };

    let mut start = 0;
    while start < n {
        let end = (start + per_line).min(n);
        let cols = Columns {
            codon_frame,
            block_start: start,
        };
        let width = cols.col(end - 1) + 1;

        // Labels.
        let marks: Vec<Mark> = site_marks
            .iter()
            .chain(feature_marks.iter())
            .filter(|(p, _)| *p >= start && *p < end)
            .map(|(p, t)| Mark {
                col: INDENT + cols.col(*p),
                text: t.clone(),
            })
            .collect();
        if !marks.is_empty() {
            for row in stack(&marks, INDENT + width) {
                out.push_str(&row);
                out.push('\n');
            }
            let mut pipes = vec![b' '; INDENT + width];
            for m in &marks {
                pipes[m.col] = b'|';
            }
            out.push_str(String::from_utf8_lossy(&pipes).trim_end());
            out.push('\n');
        }

        // Top strand.
        let mut line = vec![b' '; INDENT + width];
        for g in start..end {
            line[INDENT + cols.col(g)] = seq.bases[g];
        }
        out.push_str(&String::from_utf8_lossy(&line));
        out.push_str(&format!("   < {end}\n"));

        // Translation, one line per frame, letters under the codon's first base.
        if opts.translate != Translate::None {
            for (frame, &on) in opts.frames.iter().enumerate() {
                if !on {
                    continue;
                }
                let mut tl = vec![b' '; INDENT + width];
                let mut any = false;
                let mut g = start.saturating_sub(2);
                // First codon of this frame at or after block start.
                while g % 3 != frame {
                    g += 1;
                }
                while g < end {
                    if g >= start
                        && g + 3 <= n
                        && g >= tr_from
                        && g + 3 <= tr_to
                        && (!opts.aa_middle || g + 1 < end)
                    {
                        let codon = &seq.bases[g..g + 3];
                        let skip =
                            opts.uppercase_only && codon.iter().any(|b| b.is_ascii_lowercase());
                        if !skip {
                            let at = if opts.aa_middle { g + 1 } else { g };
                            tl[INDENT + cols.col(at)] = code.amino_acid(codon);
                            any = true;
                        }
                    }
                    g += 3;
                }
                if any || matches!(opts.translate, Translate::All) {
                    out.push_str(String::from_utf8_lossy(&tl).trim_end());
                    out.push('\n');
                }
            }
        }

        // Reverse strand.
        if opts.show_reverse {
            let mut rl = vec![b' '; INDENT + width];
            for g in start..end {
                rl[INDENT + cols.col(g)] = sc_seq::alphabet::complement(seq.bases[g]);
            }
            out.push_str(&String::from_utf8_lossy(&rl));
            out.push('\n');
        }

        // Coordinates: the number starts under its base, every 10 bases; the
        // block's last base is already named by "< end".
        if opts.show_coords {
            let mut cl = vec![b' '; INDENT + width + 12];
            let mut p = (start / 10 + 1) * 10;
            while p < end {
                let text = p.to_string();
                let c = INDENT + cols.col(p - 1);
                cl[c..c + text.len()].copy_from_slice(text.as_bytes());
                p += 10;
            }
            out.push_str(String::from_utf8_lossy(&cl).trim_end());
            out.push('\n');
        }
        out.push('\n');
        start = end;
    }

    if opts.features && !seq.features.is_empty() {
        out.push_str("\nFeatures :\n");
        for f in &seq.features {
            let dir = if f.strand == Strand::Forward {
                "CW"
            } else {
                "CCW"
            };
            out.push_str(&format!(
                "{:<14} : [{} : {} - {}]\n",
                f.name, f.start, f.end, dir
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_seq::{Feature, Rgb, Topology};

    fn lib() -> Library {
        Library::parse("EcoRI,g/aattc,\nBamHI,g/gatcc,\n#BsaI,ggtctc,1,5,\nSmaI,ccc/ggg,\n")
    }

    #[test]
    fn block_layout_matches_the_original() {
        let mut seq = Sequence::new("t", b"ATGGAATTCAAAGGATCCTAG".to_vec());
        seq.features.push(Feature {
            name: "orf".into(),
            description: String::new(),
            kind: "CDS".into(),
            start: 1,
            end: 21,
            strand: Strand::Forward,
            show: true,
            protect: false,
            arrow: true,
            color: Rgb(0, 0, 0),
        });
        let opts = TextMapOptions {
            per_line: 100,
            sites: SiteFilter::All,
            ..Default::default()
        };
        let text = render(&seq, &lib(), &opts, None);
        let lines: Vec<&str> = text.lines().collect();
        // Labels: EcoRI at base 4 and BamHI at base 13 share the lowest row;
        // the feature is labelled at its midpoint (base 11) and, colliding
        // with both, stacks above.
        assert_eq!(lines[0], "              BamHI");
        assert_eq!(lines[1], "     EcoRI  >orf");
        assert_eq!(lines[2], "     |      | |");
        assert_eq!(lines[3], "  ATGGAATTCAAAGGATCCTAG   < 21");
        // Frame 1: M E F K G S *; frame 2 and 3 offset by one column each.
        assert_eq!(lines[4], "  M  E  F  K  G  S  *");
        assert!(lines[5].starts_with("   "));
        assert!(lines[6].starts_with("    "));
        assert_eq!(lines[7], "  TACCTTAAGTTTCCTAGGATC");
        assert_eq!(lines[8], "           10        20");
        assert!(text.contains("Features :\norf            : [1 : 21 - CW]"));
    }

    #[test]
    fn non_palindromic_sites_get_arrows_and_blocks_wrap() {
        let seq = Sequence::new("t", b"ccGGTCTCaaaaaaaaaaaaGAGACCaa".to_vec());
        let opts = TextMapOptions {
            per_line: 20,
            translate: Translate::None,
            show_reverse: false,
            show_coords: false,
            sites: SiteFilter::All,
            features: false,
            ..Default::default()
        };
        let text = render(&seq, &lib(), &opts, None);
        assert!(text.contains("    >BsaI\n    |\n  ccGGTCTCaaaaaaaaaaaa   < 20\n"));
        assert!(text.contains("  <BsaI\n  |\n  GAGACCaa   < 28\n"));
    }

    #[test]
    fn max_cuts_filter_and_header() {
        let seq = Sequence::new("p", b"GAATTCggGAATTCggGGATCC".to_vec());
        let opts = TextMapOptions {
            sites: SiteFilter::MaxCuts(1),
            translate: Translate::None,
            ..Default::default()
        };
        let h = Header {
            app: "SerialClon 0.1.0".into(),
            date: "07-SEP-2026".into(),
            library: "RELibrary".into(),
        };
        let text = render(&seq, &lib(), &opts, Some(&h));
        assert!(text.starts_with("<SerialClon 0.1.0> -- <07-SEP-2026>\nRestriction map of p\nShowing restriction enzymes cutting maximum 1 time [using RELibrary as a Restriction Enzyme Library]\n###\n\n"));
        assert!(text.contains("BamHI"));
        assert!(!text.contains("EcoRI"));
    }

    #[test]
    fn codon_mode_groups_the_chosen_frame() {
        let seq = Sequence::new("t", b"ATGGAATTC".to_vec());
        let opts = TextMapOptions {
            frames: [true, false, false],
            codons: true,
            show_reverse: false,
            show_coords: false,
            sites: SiteFilter::None,
            features: false,
            ..Default::default()
        };
        let text = render(&seq, &lib(), &opts, None);
        assert_eq!(text, "  ATG GAA TTC   < 9\n  M   E   F\n\n");
    }

    #[test]
    fn amino_acid_under_the_middle_base() {
        let seq = Sequence::new("t", b"ATGGAATTC".to_vec());
        let opts = TextMapOptions {
            frames: [true, false, false],
            aa_middle: true,
            show_reverse: false,
            show_coords: false,
            sites: SiteFilter::None,
            features: false,
            ..Default::default()
        };
        assert_eq!(
            render(&seq, &lib(), &opts, None),
            "  ATGGAATTC   < 9\n   M  E  F\n\n"
        );
    }

    #[test]
    fn translation_range_and_uppercase_only() {
        let seq = Sequence::new("t", b"atgGAATTCtag".to_vec());
        let base = TextMapOptions {
            show_reverse: false,
            show_coords: false,
            sites: SiteFilter::None,
            features: false,
            frames: [true, false, false],
            ..Default::default()
        };
        let text = render(
            &seq,
            &lib(),
            &TextMapOptions {
                translate: Translate::Range(4, 9),
                ..base.clone()
            },
            None,
        );
        assert_eq!(text, "  atgGAATTCtag   < 12\n     E  F\n\n");
        let text = render(
            &seq,
            &lib(),
            &TextMapOptions {
                uppercase_only: true,
                ..base
            },
            None,
        );
        assert_eq!(text, "  atgGAATTCtag   < 12\n     E  F\n\n");
        let circular = Sequence {
            topology: Topology::Circular,
            ..seq
        };
        assert!(render(&circular, &lib(), &TextMapOptions::default(), None).contains("< 12"));
    }
}
