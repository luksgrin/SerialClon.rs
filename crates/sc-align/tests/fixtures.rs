//! Alignment against the shipped sample files.

use sc_align::{Kind, align, consensus, local};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");

fn xdna(name: &str) -> sc_seq::Sequence {
    sc_format::xdna::read(&std::fs::read(format!("{FIXTURES}/{name}")).unwrap(), name).unwrap()
}

fn genbank(name: &str) -> sc_seq::Sequence {
    sc_format::genbank::read(&std::fs::read(format!("{FIXTURES}/{name}")).unwrap())
        .unwrap()
        .remove(0)
}

/// pBluescript II KS(+) and SK(+) differ by the orientation of the
/// polylinker relative to the backbone. In the two GenBank records (X52327,
/// X52328) the backbone is what appears inverted — KS(+) carries AmpR forward
/// at 327..986, SK(+) as complement(1976..2635) — while KpnI…SacI reads the
/// same way in both. So: the whole records align best on the reverse strand
/// over the backbone, and the polylinker windows align forward — the
/// polylinker is the inverted segment.
#[test]
fn ks_and_sk_polylinker_is_inverted_relative_to_the_backbone() {
    let ks = xdna("pBluescript-II-KS+-annotated.xdna");
    let sk = genbank("pBluescript-II-SK+.gb");
    let whole = align(&ks.bases, &sk.bases, Kind::Dna).unwrap();
    assert!(
        whole.b_reverse,
        "the backbones should align on the reverse strand"
    );
    assert!(
        whole.length >= 2500,
        "backbone alignment covers only {} columns",
        whole.length
    );
    assert!(whole.identity >= 95.0, "identity {}", whole.identity);
    // Polylinker: between T3 (2172..2191) and T7 (2317..2336) in KS(+), between
    // T7 (626..645) and T3 (771..790) in SK(+).
    let ks_pl = &ks.bases[2191..2316];
    let sk_pl = &sk.bases[645..770];
    let forward = local(ks_pl, sk_pl, Kind::Dna).unwrap();
    assert!(
        forward.length >= 100 && forward.identity >= 95.0,
        "polylinker forward: {} cols, {}%",
        forward.length,
        forward.identity
    );
    let best = align(ks_pl, sk_pl, Kind::Dna).unwrap();
    assert!(
        !best.b_reverse,
        "the polylinker reads the same way in both records"
    );
    let cons = consensus(&forward);
    assert!(
        cons.iter().all(|b| b"ACGTacgt".contains(b)),
        "{}",
        String::from_utf8_lossy(&cons)
    );
}

/// AmpR is identical in both plasmids; KS(+) carries it forward, SK(+)
/// reverse.
#[test]
fn ampr_aligns_on_the_reverse_strand_between_the_records() {
    let ks = xdna("pBluescript-II-KS+-annotated.xdna");
    let sk = genbank("pBluescript-II-SK+.gb");
    let a = &ks.bases[326..986];
    let amp = sk.features.iter().find(|f| f.name == "AmpR").unwrap();
    let (lo, hi, _) = amp.lo_hi();
    let b = &sk.bases[lo as usize - 1..hi as usize];
    let aln = align(a, b, Kind::Dna).unwrap();
    assert!(aln.b_reverse);
    assert!(aln.identity > 99.0, "{}", aln.identity);
    assert_eq!(aln.length, 660);
    assert_eq!(aln.matches, 660);
}
