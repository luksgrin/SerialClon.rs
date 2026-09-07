//! The researched built-in definitions against the public GenBank records
//! they were taken from (`reference/sequences/genbank/`): a scan must find
//! each one at the record's own annotated coordinates.

use sc_features::{builtin_collections, scan};
use sc_seq::{Feature, Sequence, Strand};

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../reference/sequences/genbank"
);

fn record(file: &str) -> Sequence {
    let bytes = std::fs::read(format!("{DIR}/{file}")).unwrap_or_else(|e| panic!("{file}: {e}"));
    sc_format::genbank::read(&bytes).unwrap().remove(0)
}

fn hits(seq: &Sequence) -> Vec<Feature> {
    scan(seq, &builtin_collections())
}

fn assert_at(list: &[Feature], name: &str, lo: u32, hi: u32, strand: Strand) {
    let found = list
        .iter()
        .any(|f| f.name == name && f.lo_hi() == (lo, hi, false) && f.strand == strand);
    assert!(
        found,
        "{name} expected at {lo}..{hi} {strand:?}; got {:?}",
        list.iter()
            .filter(|f| f.name == name)
            .map(|f| f.lo_hi())
            .collect::<Vec<_>>()
    );
}

fn assert_found(list: &[Feature], name: &str) {
    assert!(
        list.iter().any(|f| f.name == name),
        "{name} not found; names: {:?}",
        list.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn pgex_4t_1_generic() {
    let seq = record("U13853-pGEX-4T-1.gb");
    let h = hits(&seq);
    assert_at(&h, "GST", 258, 911, Strand::Forward);
    assert_at(&h, "LacI", 3318, 4400, Strand::Forward);
    assert_at(&h, "tac promoter", 183, 216, Strand::Forward);
    assert_found(&h, "AmpR");
    assert_found(&h, "ColE1 origin");
}

#[test]
fn plko_1_lentiviral_elements() {
    let seq = record("PX675714-pLKO.1-shGJA1.gb");
    let h = hits(&seq);
    assert_at(&h, "hPGK promoter", 1, 511, Strand::Forward);
    assert_at(&h, "PuroR", 533, 1132, Strand::Forward);
    assert_at(&h, "Delta U3", 1260, 1493, Strand::Forward);
    assert_at(&h, "Amp Promoter", 2549, 2653, Strand::Forward);
    assert_at(&h, "HIV-1 5LTR", 4969, 5149, Strand::Forward);
    assert_at(&h, "HIV-1 psi", 5196, 5321, Strand::Forward);
    assert_at(&h, "HIV RRE", 5814, 6047, Strand::Forward);
    assert_at(&h, "U6 promoter", 6574, 6820, Strand::Forward);
    assert_found(&h, "AmpR");
    assert_found(&h, "SV40 late polyA");
}

#[test]
fn pbi121_plant_elements() {
    let seq = record("AF485783-pBI121.gb");
    let h = hits(&seq);
    assert_at(&h, "NOS-P", 2519, 2825, Strand::Forward);
    assert_at(&h, "NOS terminator", 4022, 4277, Strand::Forward);
    assert_at(&h, "CaMV 35S", 4974, 5808, Strand::Forward);
}

#[test]
fn ptrc99a_and_pbad24_prokaryotic_elements() {
    let trc = record("U13872-pTrc99A.gb");
    let h = hits(&trc);
    assert_at(&h, "trc promoter", 193, 222, Strand::Forward);
    assert_at(&h, "rrnB T1", 532, 575, Strand::Forward);
    assert_at(&h, "rrnBT2", 707, 734, Strand::Forward);
    assert_at(&h, "rrnB", 409, 734, Strand::Forward);
    assert_at(&h, "LacI", 3055, 4137, Strand::Forward);
    let bad = record("X81837-pBAD24.gb");
    let h = hits(&bad);
    assert_at(&h, "PBAD", 1213, 1277, Strand::Forward);
    assert_at(&h, "M13 origin", 2784, 3242, Strand::Forward);
    assert_found(&h, "rrnB T1");
    assert_found(&h, "rrnBT2");
}

#[test]
fn pires1neo_and_pan7_1() {
    let h = hits(&record("U89673-pIRES1neo.gb"));
    assert_at(&h, "IRES [1]", 1299, 1884, Strand::Forward);
    let h = hits(&record("Z32698-pAN7-1.gb"));
    assert_at(&h, "TrpC", 3339, 4108, Strand::Forward);
}

#[test]
fn prs_vectors_yeast_markers_and_cen_ars() {
    let h = hits(&record("U03442-pRS316.gb"));
    assert_at(&h, "URA3", 411, 1214, Strand::Forward);
    assert_found(&h, "CEN-ARS pRS");
    assert_found(&h, "F1 ori");
    let h = hits(&record("U03435-pRS303.gb"));
    assert_at(&h, "HIS3", 503, 1162, Strand::Forward);
    let h = hits(&record("U03436-pRS304.gb"));
    assert_at(&h, "TRP1", 466, 1140, Strand::Forward);
    let h = hits(&record("U03437-pRS305.gb"));
    assert!(
        h.iter()
            .any(|f| f.name == "LEU2" && f.strand == Strand::Reverse && f.len(5504) == 1095),
        "LEU2 on the reverse strand of pRS305"
    );
    let h = hits(&record("U03453-pRS424.gb"));
    assert_found(&h, "2 micron origin");
    assert_found(&h, "2u ori");
    assert_found(&h, "TRP1");
    let h = hits(&record("X70276-YCp50.gb"));
    assert_at(&h, "CEN-ARS YCp50", 3248, 5656, Strand::Forward);
    assert_found(&h, "CEN-ARS YCp-lac");
    assert_found(&h, "URA3");
}

/// Peptide definitions are matched against the translation: a back-translated
/// Fc region and GAL4 domain must be found in any frame and strand.
#[test]
fn peptide_definitions_match_back_translated_dna() {
    let collections = builtin_collections();
    let defs: Vec<_> = collections
        .iter()
        .flat_map(|c| c.features.iter())
        .filter(|d| d.peptide)
        .collect();
    let codon = |aa: u8| -> &'static str {
        match aa {
            b'A' => "GCT",
            b'R' => "CGT",
            b'N' => "AAT",
            b'D' => "GAT",
            b'C' => "TGT",
            b'E' => "GAA",
            b'Q' => "CAA",
            b'G' => "GGT",
            b'H' => "CAT",
            b'I' => "ATT",
            b'L' => "CTG",
            b'K' => "AAA",
            b'M' => "ATG",
            b'F' => "TTT",
            b'P' => "CCT",
            b'S' => "TCT",
            b'T' => "ACT",
            b'W' => "TGG",
            b'Y' => "TAT",
            b'V' => "GTT",
            _ => "NNN",
        }
    };
    for name in [
        "human IgG1 Fc",
        "mouse IgG2A Fc",
        "rat IgG2B Fc",
        "Gal4 DNA-binding domain",
        "Gal4 activation domain",
        "LexA DNA-binding domain",
    ] {
        let def = defs
            .iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("{name} missing"));
        let dna: String = format!("gg{}gg", def.five.bytes().map(codon).collect::<String>());
        let seq = Sequence::new("t", dna.into_bytes());
        let h = hits(&seq);
        let f = h
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} not found in its back-translation"));
        assert_eq!(
            (f.start, f.end),
            (3, 2 + 3 * def.five.len() as u32),
            "{name}"
        );
        let rc = Sequence::new("r", sc_seq::alphabet::reverse_complement(&seq.bases));
        assert!(
            hits(&rc)
                .iter()
                .any(|f| f.name == name && f.strand == Strand::Reverse),
            "{name} on the reverse strand"
        );
    }
}
