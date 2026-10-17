//! The recovered definitions against public GenBank records of the vectors
//! they come from (`reference/sequences/genbank/`), plus our own additions.
//!
//! The original's definitions are what they are (some carry odd letters,
//! some are fragments of a particular vector), so these tests only assert
//! that the expected elements are found; the coordinates are checked
//! against the original's own output in `fixtures.rs`.

use sc_features::{builtin_collections, scan};
use sc_seq::{Feature, Sequence, Strand};

const RECORDS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../reference/sequences/genbank"
);

fn record(file: &str) -> Sequence {
    let bytes = std::fs::read(format!("{RECORDS}/{file}")).unwrap();
    sc_format::genbank::read(&bytes).unwrap().remove(0)
}

fn hits(seq: &Sequence) -> Vec<Feature> {
    scan(seq, &builtin_collections())
}

/// Scan with our additions enabled too.
fn hits_with_additions(seq: &Sequence) -> Vec<Feature> {
    let mut cols = builtin_collections();
    for c in &mut cols {
        c.scan = true;
    }
    scan(seq, &cols)
}

fn names(list: &[Feature]) -> Vec<&str> {
    list.iter().map(|f| f.name.as_str()).collect()
}

fn assert_found(list: &[Feature], name: &str) {
    assert!(
        list.iter().any(|f| f.name == name),
        "{name} not found; names: {:?}",
        names(list)
    );
}

fn assert_at(list: &[Feature], name: &str, start: u32, end: u32, strand: Strand) {
    assert!(
        list.iter()
            .any(|f| f.name == name && f.start == start && f.end == end && f.strand == strand),
        "{name} expected at {start}..{end} {strand:?}; got {:?}",
        list.iter()
            .filter(|f| f.name == name)
            .map(|f| (f.start, f.end, f.strand))
            .collect::<Vec<_>>()
    );
}

#[test]
fn pgex_4t_1_generic() {
    let h = hits(&record("U13853-pGEX-4T-1.gb"));
    // GST is annotated 258..911 in U13853 and the original's 654-bp
    // consensus is exactly that span.
    assert_at(&h, "GST", 258, 911, Strand::Forward);
    assert_at(&h, "Thrombin site", 918, 935, Strand::Forward);
    assert_at(&h, "tac", 184, 212, Strand::Forward);
    for n in [
        "lacI",
        "LacO",
        "AmpR",
        "Amp prom",
        "ColE1 origin",
        "LacZ alpha",
    ] {
        assert_found(&h, n);
    }
}

#[test]
fn plko_1_lentiviral_elements() {
    let h = hits(&record("PX675714-pLKO.1-shGJA1.gb"));
    assert_at(&h, "delta U3", 1260, 1312, Strand::Forward);
    assert_at(&h, "HIV-1 5 LTR", 4969, 5149, Strand::Forward);
    assert_at(&h, "RRE", 5814, 6047, Strand::Forward);
    for n in [
        "HIV-1 psi pack",
        "SV40 ORI",
        "AmpR",
        "F1 ori",
        "ColE1 origin",
    ] {
        assert_found(&h, n);
    }
    // The vector's PuroR and U6 promoter differ from the original's
    // consensuses (pLKO.1 carries pac of a different origin and a U6
    // fragment that is 16 bp short), so the original does not find them.
    assert!(
        !h.iter()
            .any(|f| f.name == "PuroR" || f.name == "U6 promoter")
    );
}

#[test]
fn pbi121_and_pan7_1_plant_and_fungal_elements() {
    let h = hits(&record("AF485783-pBI121.gb"));
    assert_at(&h, "NOS-P", 2530, 2802, Strand::Forward);
    let h = hits_with_additions(&record("AF485783-pBI121.gb"));
    assert_found(&h, "NOS terminator");
    let h = hits(&record("Z32698-pAN7-1.gb"));
    for n in ["AmpR", "ColE1 origin", "lac", "LacO"] {
        assert_found(&h, n);
    }
}

#[test]
fn ptrc99a_and_pbad24_prokaryotic_elements() {
    let h = hits(&record("U13872-pTrc99A.gb"));
    assert_at(&h, "rrnB", 409, 566, Strand::Forward);
    assert_at(&h, "rrnB T2", 707, 734, Strand::Forward);
    assert_at(&h, "lacI", 3046, 4137, Strand::Forward);
    let h = hits_with_additions(&record("U13872-pTrc99A.gb"));
    assert_at(&h, "trc promoter", 193, 222, Strand::Forward);
    let h = hits(&record("X81837-pBAD24.gb"));
    assert_at(&h, "PBAD", 1250, 1277, Strand::Forward);
    for n in ["rrnB", "rrnB T2", "AmpR", "Amp prom", "ColE1 origin"] {
        assert_found(&h, n);
    }
}

#[test]
fn pires1neo_carries_kan_neo() {
    let h = hits(&record("U89673-pIRES1neo.gb"));
    assert_at(&h, "Kan/neoR", 1922, 2713, Strand::Forward);
    assert_found(&h, "T7");
}

#[test]
fn prs_vectors_yeast_markers_and_cen_ars() {
    let h = hits(&record("U03442-pRS316.gb"));
    assert_at(&h, "CEN-ARS pRS", 4305, 4815, Strand::Forward);
    assert_found(&h, "F1 ori");
    let h = hits(&record("U03435-pRS303.gb"));
    assert_at(&h, "HIS3", 191, 1363, Strand::Forward);
    let h = hits(&record("U03436-pRS304.gb"));
    assert_at(&h, "TRP1", 364, 1186, Strand::Forward);
    let h = hits(&record("U03437-pRS305.gb"));
    assert!(
        h.iter()
            .any(|f| f.name == "LEU2" && f.strand == Strand::Reverse),
        "LEU2 on the reverse strand of pRS305"
    );
    let h = hits(&record("U03453-pRS424.gb"));
    for n in ["2 micron origin", "2u ori", "TRP1", "FRT"] {
        assert_found(&h, n);
    }
    let h = hits(&record("X70276-YCp50.gb"));
    assert_at(&h, "CEN-ARS YCp50", 3251, 5656, Strand::Forward);
    for n in ["TetA", "TetR", "AmpR"] {
        assert_found(&h, n);
    }
}

/// `#` in a consensus is the original's gap wildcard (`.*` in its regex).
/// "M13 origin" has one, and matches the pRS vectors' f1 origin region
/// with a few bases bridged.
#[test]
fn gap_wildcard_definitions_match() {
    let h = hits(&record("U03442-pRS316.gb"));
    let m13 = h
        .iter()
        .find(|f| f.name == "M13 origin")
        .expect("M13 origin");
    assert_eq!(
        (m13.start, m13.end, m13.strand),
        (1798, 1343, Strand::Reverse)
    );
}

/// Our additions are shipped in a collection that is not scanned unless
/// the user enables it.
#[test]
fn additions_are_off_by_default() {
    let seq = record("U13872-pTrc99A.gb");
    assert!(!hits(&seq).iter().any(|f| f.name == "trc promoter"));
    assert!(
        hits_with_additions(&seq)
            .iter()
            .any(|f| f.name == "trc promoter")
    );
}
