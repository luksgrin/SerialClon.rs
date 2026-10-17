//! A real subcloning, end to end: EGFP out of pEGFP-N1 with BamHI and NotI,
//! into pBluescript II KS(+) opened at the same sites, exactly as a user
//! would do it with Extract Fragment and Build a Construct.

use sc_clone::ligate::{Side, blunt, ligate};
use sc_clone::{SiteRef, extract_between};
use sc_enzyme::{Library, find_sites};
use sc_features::{builtin_collections, merge, scan};
use sc_seq::{Overhang, Sequence, Topology};

const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(name: &str) -> Sequence {
    let path = format!("{REPO}/reference/sequences/{name}");
    sc_format::xdna::read(&std::fs::read(&path).unwrap(), name).unwrap()
}

fn site(seq: &Sequence, lib: &Library, enzyme: &str) -> SiteRef {
    let e = lib.get(enzyme).unwrap();
    let sites = find_sites(&seq.bases, seq.topology, e);
    assert_eq!(sites.len(), 1, "{enzyme} must be unique in {}", seq.name);
    let s = &sites[0];
    SiteRef {
        enzyme: e.name.clone(),
        position: s.position + 1,
        cut_top: s.cut_top,
        cut_bottom: s.cut_bottom,
    }
}

#[test]
fn egfp_from_pegfp_n1_into_pbluescript_ks_plus() {
    let lib =
        Library::parse(&std::fs::read_to_string(format!("{REPO}/resources/RELibrary")).unwrap());
    let collections = builtin_collections();

    // The donor, annotated by a scan (as the user would do first).
    let mut donor = read("pEGFP-N1.xdna");
    let found = scan(&donor, &collections);
    merge(&mut donor.features, found);
    assert!(donor.features.iter().any(|f| f.name == "EGFP"));

    // Extract the BamHI–NotI fragment carrying EGFP.
    let bam = site(&donor, &lib, "BamHI");
    let not = site(&donor, &lib, "NotI");
    let insert = extract_between(&donor, &bam, &not, true).unwrap();
    assert_eq!(insert.ends.left, Overhang::five_prime(b"GATC"));
    assert_eq!(insert.ends.right, Overhang::five_prime(b"GGCC"));
    let egfp = insert
        .features
        .iter()
        .find(|f| f.name == "EGFP")
        .expect("EGFP travels with the fragment");
    // The original's EGFP consensus is the 714-bp body without start/stop.
    assert_eq!(egfp.len(insert.len() as u32), 714);

    // Open the vector at the same sites: the backbone runs from NotI round to BamHI.
    let vector_src = read("pBluescript-II-KS+-annotated.xdna");
    let v_bam = site(&vector_src, &lib, "BamHI");
    let v_not = site(&vector_src, &lib, "NotI");
    let vector = extract_between(&vector_src, &v_not, &v_bam, true).unwrap();
    // KS(+) is stored in lower case there; ends are compared case-insensitively.
    assert_eq!(vector.ends.left.bases.to_ascii_uppercase(), b"GGCC");
    assert_eq!(vector.ends.right.bases.to_ascii_uppercase(), b"GATC");
    assert!(vector.ends.left.is_five_prime() && vector.ends.right.is_five_prime());
    assert!(vector.features.iter().any(|f| f.name == "AmpR"));
    assert!(vector.features.iter().any(|f| f.name == "ColE1 origin"));

    // Ligate: vector right (BamHI) to insert left (BamHI), insert right (NotI) back to vector left (NotI).
    let product = ligate(&[vector.clone(), insert.clone()], true, "pBS-EGFP").unwrap();
    assert_eq!(product.topology, Topology::Circular);
    assert!(product.ends.is_blunt());
    // Two 4-nt duplexes counted once each.
    assert_eq!(product.len(), vector.len() + insert.len() - 8);
    assert!(
        product
            .bases
            .iter()
            .all(|b| matches!(b.to_ascii_uppercase(), b'A' | b'C' | b'G' | b'T')),
        "no degenerate DNA from a ligation"
    );
    // The EGFP ORF is intact and in the product, with its features and the vector's.
    let egfp_seq = String::from_utf8_lossy(&donor.bases[678..1398]).to_ascii_uppercase();
    assert!(
        String::from_utf8_lossy(&product.bases)
            .to_ascii_uppercase()
            .contains(&egfp_seq)
    );
    let names: Vec<&str> = product.features.iter().map(|f| f.name.as_str()).collect();
    for expected in ["EGFP", "AmpR", "ColE1 origin", "F1 ori", "T7"] {
        assert!(
            names.contains(&expected),
            "{expected} missing from {names:?}"
        );
    }
    let n_egfp = product.features.iter().filter(|f| f.name == "EGFP").count();
    assert_eq!(n_egfp, 1, "no duplicated features");
    // The BamHI and NotI sites are regenerated at the junctions.
    assert_eq!(
        find_sites(&product.bases, product.topology, lib.get("BamHI").unwrap()).len(),
        1
    );
    assert_eq!(
        find_sites(&product.bases, product.topology, lib.get("NotI").unwrap()).len(),
        1
    );
    // The original's comment: `Ligation of : <vector label> to <insert label>`,
    // each label being the fragment's own provenance line.
    assert!(product.comment.starts_with("Ligation of : pBluescript-II-KS+-annotated.xdna [2946  nt] : (NotI[2286] / BamHI[2268]) to pEGFP-N1.xdna [745  nt] : (BamHI[660] / NotI[1400])"));
    assert!(product.comment.ends_with(")"));

    // Mismatched ends are refused: the pEGFP backbone (NotI…BamHI) presents
    // a NotI end to the vector's BamHI end.
    let mut flipped = extract_between(&donor, &not, &bam, true).unwrap();
    assert!(ligate(&[vector.clone(), flipped.clone()], true, "x").is_err());
    // Blunting both makes it possible (with the sites destroyed).
    blunt(&mut flipped, Side::Left);
    blunt(&mut flipped, Side::Right);
    let mut vector_blunt = vector.clone();
    blunt(&mut vector_blunt, Side::Left);
    blunt(&mut vector_blunt, Side::Right);
    let blunt_product = ligate(&[vector_blunt, flipped], true, "blunt").unwrap();
    assert_eq!(blunt_product.topology, Topology::Circular);
}
