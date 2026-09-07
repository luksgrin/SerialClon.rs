//! The built-in collections against the shipped, annotated sample files:
//! a scan must reproduce what the original's scan put in them.

use sc_features::{builtin_collections, merge, scan};
use sc_seq::{Feature, Strand};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");

fn read(name: &str) -> sc_seq::Sequence {
    let path = format!("{FIXTURES}/{name}");
    sc_format::xdna::read(&std::fs::read(&path).unwrap(), name).unwrap()
}

fn found(seq: &sc_seq::Sequence) -> Vec<Feature> {
    scan(seq, &builtin_collections())
}

fn has(list: &[Feature], name: &str, start: u32, end: u32, strand: Strand) -> bool {
    list.iter()
        .any(|f| f.name == name && f.start == start && f.end == end && f.strand == strand)
}

#[test]
fn ks_plus_annotations_are_reproduced() {
    let seq = read("pBluescript-II-KS+-annotated.xdna");
    let hits = found(&seq);
    for f in &seq.features {
        assert!(
            has(&hits, &f.name, f.start, f.end, f.strand),
            "{} [{}:{} {:?}] not found; hits: {:?}",
            f.name,
            f.start,
            f.end,
            f.strand,
            hits.iter()
                .map(|h| format!("{} {}:{}", h.name, h.start, h.end))
                .collect::<Vec<_>>()
        );
    }
    // Merging the scan into the stored annotations changes nothing but adds the new ones.
    let mut features = seq.features.clone();
    let before = features.len();
    let added = merge(&mut features, hits);
    assert!(features.len() >= before);
    assert!(
        added >= 1,
        "the built-in database finds at least the lac promoter too"
    );
}

#[test]
fn pbr322_annotations_are_reproduced() {
    let seq = read("pBR322-annotated.xdna");
    let hits = found(&seq);
    for f in &seq.features {
        assert!(
            has(&hits, &f.name, f.start, f.end, f.strand),
            "{} [{}:{}] not found",
            f.name,
            f.start,
            f.end
        );
    }
}

#[test]
fn pegfp_n1_is_annotated_at_clontechs_coordinates() {
    let seq = read("pEGFP-N1.xdna");
    let hits = found(&seq);
    // EGFP 679..1398 including the start and stop codons picked up as potential ends.
    assert!(
        has(&hits, "EGFP", 679, 1398, Strand::Forward),
        "{:?}",
        hits.iter()
            .map(|h| format!("{} {}:{}", h.name, h.start, h.end))
            .collect::<Vec<_>>()
    );
    assert!(has(&hits, "CMV promoter", 1, 589, Strand::Forward));
    assert!(has(&hits, "Kan/neoR", 2629, 3423, Strand::Forward));
    assert!(hits.iter().any(|h| h.name == "SV40 late polyA"));
    assert!(hits.iter().any(|h| h.name == "F1 ori"));
    assert!(hits.iter().any(|h| h.name == "ColE1 origin"));
}

#[test]
fn gateway_vectors_carry_their_att_sites_and_ccdb() {
    let donr = read("pDONR201.xdna");
    let hits = found(&donr);
    for name in ["AttP1", "AttP2", "ccdB", "CmR"] {
        assert!(
            hits.iter().any(|h| h.name == name),
            "{name} missing from pDONR201"
        );
    }
    let dest = read("pDEST14.xdna");
    let hits = found(&dest);
    for name in ["AttR1", "AttR2", "ccdB", "CmR", "AmpR", "T7"] {
        assert!(
            hits.iter().any(|h| h.name == name),
            "{name} missing from pDEST14"
        );
    }
}
