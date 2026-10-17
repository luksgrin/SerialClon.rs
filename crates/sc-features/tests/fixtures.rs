//! The built-in collections against what Serial Cloner 2.6.1 itself found.
//!
//! `reference/original/golden/scan/` holds the files the original wrote
//! after *Features ▸ Scan Sequence* on the shipped sample files (see
//! `reference/docs/original-behaviour.md`). A scan with the recovered
//! definitions must produce exactly the same feature set, name by name and
//! coordinate by coordinate.

use std::collections::BTreeSet;

use sc_features::{builtin_collections, merge, scan};
use sc_seq::{Feature, Strand};

const SAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");
const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../reference/original/golden/scan"
);

fn read(dir: &str, name: &str) -> sc_seq::Sequence {
    let path = format!("{dir}/{name}");
    sc_format::xdna::read(&std::fs::read(&path).unwrap(), name).unwrap()
}

fn key(f: &Feature) -> (String, u32, u32, bool) {
    (f.name.clone(), f.start, f.end, f.strand == Strand::Reverse)
}

/// Scans `sample` and merges the hits into its existing features, like the
/// original's Scan Sequence; the result must equal the golden file's
/// features (the original's pre-existing, user-made features are protected
/// and excluded on both sides).
fn check(sample: &str, golden: &str) {
    let mut seq = read(SAMPLES, sample);
    let expected: BTreeSet<_> = read(GOLDEN, golden)
        .features
        .iter()
        .filter(|f| !f.protect)
        .map(key)
        .collect();
    let hits = scan(&seq, &builtin_collections());
    merge(&mut seq.features, hits);
    let actual: BTreeSet<_> = seq
        .features
        .iter()
        .filter(|f| !f.protect)
        .map(key)
        .collect();
    assert_eq!(actual, expected, "{sample} vs {golden}");
}

#[test]
fn ks_plus_scan_matches_the_original() {
    check("pBluescript-II-KS+.xdna", "pBluescript-scanned.xdna");
}

#[test]
fn ks_plus_annotated_scan_matches_the_original() {
    // Nine features were already there (an older database put them in);
    // the 2.6.1 scan adds "Amp prom" and "lac" and changes nothing else.
    check(
        "pBluescript-II-KS+-annotated.xdna",
        "KS(+)-annotated-scanned.xdna",
    );
}

#[test]
fn pbr322_annotated_scan_matches_the_original() {
    // Adds TetA (a second, longer tet definition) and "Amp prom".
    check("pBR322-annotated.xdna", "pBR322-annotated-scanned.xdna");
}

#[test]
fn pegfp_n1_scan_matches_the_original() {
    // The original finds seven features here and, notably, not its own
    // "CMV promoter" (whose consensus carries an N that pEGFP-N1 does not
    // match) nor "SV40 late polyA".
    check("pEGFP-N1.xdna", "pEGFP-N1-scanned.xdna");
}

#[test]
fn gateway_vectors_scan_matches_the_original() {
    // pDONR201 carries KanR, whose consensus has the `#` gap wildcard.
    check("pDONR201.xdna", "pDONR201-scanned.xdna");
    check("pDEST14.xdna", "pDEST14-scanned.xdna");
}

#[test]
fn scan_is_idempotent_on_the_originals_output() {
    // Scanning a file the original has already scanned adds nothing.
    for golden in [
        "pBluescript-scanned.xdna",
        "pBR322-annotated-scanned.xdna",
        "pDONR201-scanned.xdna",
        "pDEST14-scanned.xdna",
    ] {
        let mut seq = read(GOLDEN, golden);
        let before = seq.features.len();
        let keys: BTreeSet<_> = seq.features.iter().map(key).collect();
        let hits = scan(&seq, &builtin_collections());
        merge(&mut seq.features, hits);
        assert_eq!(seq.features.len(), before, "{golden}");
        let after: BTreeSet<_> = seq.features.iter().map(key).collect();
        assert_eq!(after, keys, "{golden}");
    }
}
