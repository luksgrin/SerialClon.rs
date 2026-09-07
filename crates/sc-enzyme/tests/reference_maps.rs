//! Digestion checked against Serial Cloner's own printed restriction map of
//! pBluescript II SK(+) (Features ReadMe v2.1 §IV, "Showing restriction
//! enzymes cutting maximum 1 time"), extracted to
//! `reference/docs/pBluescript-II-SK+-unique-sites-map.txt`, and against the
//! textbook pBR322 map.

use sc_enzyme::{Library, find_sites};
use sc_seq::{Strand, Topology};

const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{REPO}/{path}")).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The enzyme database Serial Cloner 2.6.1 shipped — the closest we have to
/// the one the map was printed with (2010).
fn library_211() -> Library {
    Library::parse(&read("reference/RELibrary-striderc.211"))
}

fn sk_plus() -> sc_seq::Sequence {
    sc_format::genbank::read(read("reference/sequences/pBluescript-II-SK+.gb").as_bytes())
        .unwrap()
        .remove(0)
}

/// A site label as printed on the map: name, orientation arrow if any, and
/// the 1-based column position its `|` points at.
#[derive(Debug)]
struct Printed {
    name: String,
    arrow: Option<char>,
    position: usize,
}

/// Parses the printed map. Each block is: label lines, a line of `|` marks,
/// the top-strand line ending in `< N`, three translation lines, the bottom
/// strand and a coordinate line. A label's first character sits in the
/// column of its `|`; the column maps to a nucleotide through the top-strand
/// line. Feature names (from the "Features :" legend) are ignored.
fn parse_map(text: &str, features: &[&str]) -> Vec<Printed> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut labels: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let is_pipe_line = !line.trim().is_empty() && line.chars().all(|c| c == ' ' || c == '|');
        if is_pipe_line {
            let seq_line = lines[i + 1];
            let (seq_part, end) = seq_line
                .rsplit_once('<')
                .expect("top strand line ends with < N");
            let end: usize = end.trim().parse().unwrap();
            let seq_col = seq_part.find(|c: char| "ACGT".contains(c)).unwrap();
            let seq_len = seq_part.trim().len();
            let start = end + 1 - seq_len;
            let pipe_cols: Vec<usize> = line
                .char_indices()
                .filter(|(_, c)| *c == '|')
                .map(|(k, _)| k)
                .collect();
            for label_line in &labels {
                // Tokens separated by two or more spaces; a token may hold one space ("F1 ori").
                let mut col = 0;
                for chunk in label_line.split("  ") {
                    if !chunk.is_empty() {
                        let token = chunk.trim();
                        let token_col = col + (chunk.len() - chunk.trim_start().len());
                        // Two site names one space apart ("SciI HincII") are two labels.
                        let pieces: Vec<(usize, &str)> =
                            if features.contains(&token.trim_start_matches(['>', '<'])) {
                                vec![(token_col, token)]
                            } else {
                                let mut v = Vec::new();
                                let mut c = token_col;
                                for piece in token.split(' ') {
                                    v.push((c, piece));
                                    c += piece.len() + 1;
                                }
                                v
                            };
                        for (token_col, token) in pieces {
                            let (arrow, name) = match token.chars().next() {
                                Some(c @ ('>' | '<')) => (Some(c), &token[1..]),
                                _ => (None, token),
                            };
                            if !features.contains(&name) && !name.is_empty() {
                                // Snap to the nearest pipe (pdftotext can drift by a column).
                                let pipe = pipe_cols
                                    .iter()
                                    .min_by_key(|&&p| p.abs_diff(token_col))
                                    .copied()
                                    .unwrap_or(token_col);
                                if pipe.abs_diff(token_col) <= 1 && pipe >= seq_col {
                                    out.push(Printed {
                                        name: name.to_owned(),
                                        arrow,
                                        position: start + (pipe - seq_col),
                                    });
                                }
                            }
                        }
                    }
                    col += chunk.len() + 2;
                }
            }
            labels.clear();
            i += 7;
            continue;
        }
        if !line.trim().is_empty()
            && !line.starts_with("Restriction map")
            && !line.starts_with('#')
            && !line.starts_with('<')
        {
            labels.push(line);
        } else {
            labels.clear();
        }
        i += 1;
    }
    out
}

const FEATURES: &[&str] = &[
    "T3",
    "M13-fwd",
    "M13-rev",
    "T7",
    "ColE1 origin",
    "F1 ori",
    "LacZ alpha",
    "LacO",
    "AmpR",
];

#[test]
fn sk_plus_printed_map_positions_are_reproduced() {
    let map = read("reference/docs/pBluescript-II-SK+-unique-sites-map.txt");
    let printed = parse_map(&map, FEATURES);
    assert!(printed.len() >= 50, "parsed only {} labels", printed.len());
    let seq = sk_plus();
    assert_eq!(seq.len(), 2961);
    let lib = library_211();

    let mut compared = 0;
    let mut exact = 0;
    let mut problems = Vec::new();
    for p in &printed {
        // UthSI is the only label the map has that REBASE 211 no longer lists.
        let Some(enzyme) = lib.get(&p.name) else {
            assert_eq!(
                p.name, "UthSI",
                "unexpected enzyme missing from the 2012 library"
            );
            continue;
        };
        let sites = find_sites(&seq.bases, Topology::Circular, enzyme);
        compared += 1;
        // The map shows enzymes cutting exactly once.
        if sites.len() != 1 {
            problems.push(format!("{} should cut once, found {}", p.name, sites.len()));
            continue;
        }
        let site = &sites[0];
        // Serial Cloner prints the first base of the recognition site. The
        // pdftotext extraction drifts by one column on a few labels, hence
        // the tolerance; the majority must be exact.
        let first_base = site.position + 1;
        if first_base == p.position {
            exact += 1;
        } else if first_base.abs_diff(p.position) > 1 {
            problems.push(format!(
                "{} printed at {}, ours at {}",
                p.name, p.position, first_base
            ));
        }
        // ">" / "<" mark the strand of non-palindromic sites.
        if let Some(arrow) = p.arrow {
            let expected = if arrow == '>' {
                Strand::Forward
            } else {
                Strand::Reverse
            };
            if site.strand != expected {
                problems.push(format!(
                    "{}{} strand mismatch: ours {:?}",
                    arrow, p.name, site.strand
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(compared >= 45, "compared only {compared}");
    assert!(
        exact * 4 >= compared * 3,
        "only {exact}/{compared} positions exact"
    );
}

/// The textbook pBR322 map, as first-base coordinates read off the sequence
/// (NEB's catalogue quotes ScaI by its cut, 3846; the site starts at 3844).
/// pBR322 is numbered so that 0 is the middle of the EcoRI site, hence EcoRI
/// at 4359, straddling the origin.
#[test]
fn pbr322_textbook_sites() {
    let seq = sc_format::xdna::read(
        &std::fs::read(format!("{REPO}/reference/sequences/pBR322.xdna")).unwrap(),
        "pBR322",
    )
    .unwrap();
    let lib = Library::parse(&read("resources/RELibrary"));
    let unique = [
        ("ClaI", 23),
        ("HindIII", 29),
        ("EcoRV", 185),
        ("BamHI", 375),
        ("SphI", 562),
        ("SalI", 651),
        ("NruI", 972),
        ("PvuII", 2064),
        ("NdeI", 2295),
        ("PstI", 3607),
        ("PvuI", 3733),
        ("ScaI", 3844),
        ("AatII", 4284),
        ("EcoRI", 4359),
    ];
    for (name, position) in unique {
        let sites = find_sites(&seq.bases, Topology::Circular, lib.get(name).unwrap());
        assert_eq!(sites.len(), 1, "{name} should be unique in pBR322");
        assert_eq!(sites[0].position + 1, position, "{name}");
    }
    for name in [
        "KpnI", "XhoI", "NotI", "SacI", "SmaI", "XbaI", "BglII", "NcoI", "ApaI", "SpeI",
    ] {
        assert!(
            find_sites(&seq.bases, Topology::Circular, lib.get(name).unwrap()).is_empty(),
            "{name} should be absent from pBR322"
        );
    }
    // The EcoRI site straddles the origin: a linear reading must not find it.
    assert!(find_sites(&seq.bases, Topology::Linear, lib.get("EcoRI").unwrap()).is_empty());

    let usage = sc_enzyme::site_usage(&seq.bases, Topology::Circular, &lib);
    let uniques: Vec<&str> = usage
        .iter()
        .filter(|u| u.is_unique())
        .map(|u| u.enzyme.name.as_str())
        .collect();
    for (name, _) in unique {
        assert!(
            uniques.contains(&name),
            "{name} missing from the unique list"
        );
    }
    assert!(
        usage
            .iter()
            .any(|u| u.is_absent() && u.enzyme.name == "NotI")
    );
}

/// pBluescript II KS(+) and SK(+): the polylinker enzymes are unique in both,
/// and the polylinker sits between the T3 and T7 promoters — after T3 in
/// KS(+), after T7 in SK(+) (that is what the two names mean).
#[test]
fn polylinker_is_unique_and_between_the_promoters() {
    let lib = Library::parse(&read("resources/RELibrary"));
    let order = [
        "KpnI", "ApaI", "XhoI", "SalI", "ClaI", "HindIII", "EcoRV", "EcoRI", "PstI", "SmaI",
        "BamHI", "SpeI", "XbaI", "NotI", "SacII", "SacI",
    ];
    let ks = sc_format::xdna::read(
        &std::fs::read(format!(
            "{REPO}/reference/sequences/pBluescript-II-KS+-annotated.xdna"
        ))
        .unwrap(),
        "KS",
    )
    .unwrap();
    let sk = sk_plus();
    for (plasmid, first, last) in [(&ks, "T3", "T7"), (&sk, "T7", "T3")] {
        let positions: Vec<u32> = order
            .iter()
            .map(|name| {
                let sites = find_sites(&plasmid.bases, Topology::Circular, lib.get(name).unwrap());
                assert_eq!(
                    sites.len(),
                    1,
                    "{name} should be unique in {}",
                    plasmid.name
                );
                sites[0].position as u32 + 1
            })
            .collect();
        assert!(
            positions.windows(2).all(|w| w[0] < w[1]),
            "{}: {positions:?}",
            plasmid.name
        );
        let span = |label: &str| {
            plasmid
                .features
                .iter()
                .find(|f| f.name == label)
                .unwrap()
                .lo_hi()
        };
        let (before, after) = (span(first), span(last));
        assert!(
            before.1 < positions[0],
            "{}: {first} should precede the polylinker",
            plasmid.name
        );
        assert!(
            positions[positions.len() - 1] < after.0,
            "{}: {last} should follow the polylinker",
            plasmid.name
        );
    }
}

/// The Sequence Map renderer against the original's printed map of SK(+):
/// same labels at the same columns (the 2012 library adds a few enzymes and
/// lost UthSI), and the same translation, reverse strand and coordinate
/// lines in the first block (pdftotext collapsed the spaces between amino
/// acids, so those are compared without spaces).
#[test]
fn sequence_map_reproduces_the_printed_sk_plus_map() {
    use sc_enzyme::textmap::{SiteFilter, TextMapOptions, render};
    let printed_text = read("reference/docs/pBluescript-II-SK+-unique-sites-map.txt");
    let printed = parse_map(&printed_text, FEATURES);
    let seq = sk_plus();
    let lib = library_211();
    let opts = TextMapOptions {
        sites: SiteFilter::MaxCuts(1),
        ..Default::default()
    };
    let ours_text = render(&seq, &lib, &opts, None);
    let ours = parse_map(&ours_text, FEATURES);
    assert!(
        ours.len() >= printed.len() - 1,
        "ours {} vs printed {}",
        ours.len(),
        printed.len()
    );
    for p in &printed {
        if p.name == "UthSI" {
            continue;
        }
        let found = ours.iter().any(|o| {
            o.name == p.name && o.arrow == p.arrow && o.position.abs_diff(p.position) <= 1
        });
        assert!(
            found,
            "{}{} @{} not in our map",
            p.arrow.map(String::from).unwrap_or_default(),
            p.name,
            p.position
        );
    }
    // First block, line by line.
    let strip = |s: &str| s.replace(' ', "");
    let printed_lines: Vec<&str> = printed_text.lines().collect();
    let ours_lines: Vec<&str> = ours_text.lines().collect();
    let p0 = printed_lines
        .iter()
        .position(|l| l.starts_with("  CTAAATTGTAAGCG"))
        .unwrap();
    let o0 = ours_lines
        .iter()
        .position(|l| l.starts_with("  CTAAATTGTAAGCG"))
        .unwrap();
    assert_eq!(ours_lines[o0], printed_lines[p0]); // top strand + "< 100"
    for k in 1..=3 {
        assert_eq!(
            strip(ours_lines[o0 + k]),
            strip(printed_lines[p0 + k]),
            "frame {k}"
        );
        assert_eq!(
            ours_lines[o0 + k].len() - ours_lines[o0 + k].trim_start().len(),
            1 + k,
            "frame {k} indent"
        );
    }
    assert_eq!(ours_lines[o0 + 4], printed_lines[p0 + 4]); // reverse strand
    assert_eq!(
        ours_lines[o0 + 5].trim_end(),
        printed_lines[p0 + 5].trim_end()
    ); // coordinates
    assert_eq!(ours_lines[o0 - 1], printed_lines[p0 - 1]); // the PsiI pipe
    // The 2012 library adds AanI, PsiI's isoschizomer, stacked at the same
    // column; the printed PsiI label must be one of the rows above the pipe.
    assert!(
        ours_lines[o0 - 3..o0 - 1].contains(&printed_lines[p0 - 2]),
        "{:?}",
        &ours_lines[o0 - 3..o0]
    );
    // Legend.
    assert!(ours_text.contains("Features :\nT3             : [790 : 771 - CCW]\n"));
}
