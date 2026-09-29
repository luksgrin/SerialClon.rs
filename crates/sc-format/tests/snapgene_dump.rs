//! Writes, for every SnapGene fixture, what our reader saw (JSON) and two
//! files our writer produces — the byte-exact round trip and a "fresh" file
//! written from the model alone — under the target directory, so that an
//! independent parser (Biopython, `scripts/check-snapgene.py`) can compare
//! them.

use std::path::Path;

use sc_seq::Strand;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../reference/sequences/snapgene"
);

#[test]
fn dump_for_external_check() {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("snapgene-check");
    std::fs::create_dir_all(&out).unwrap();
    let mut paths: Vec<_> = std::fs::read_dir(FIXTURES)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "dna"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 20);
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let seq = sc_format::snapgene::read(&bytes, &stem).unwrap();
        let features: Vec<serde_json::Value> = seq
            .features
            .iter()
            .map(|f| {
                let (lo, hi, wraps) = f.lo_hi();
                serde_json::json!({
                    "name": f.name, "kind": f.kind, "lo": lo, "hi": hi, "wraps": wraps,
                    "strand": if f.strand == Strand::Forward { 1 } else { -1 },
                    "primer": f.description.contains("snapgene_primer="),
                })
            })
            .collect();
        let report = serde_json::json!({
            "length": seq.len(),
            "circular": seq.topology.is_circular(),
            "seq_type": seq.seq_type.label(),
            "sequence": String::from_utf8_lossy(&seq.bases),
            "comment": seq.comment,
            "features": features,
        });
        std::fs::write(
            out.join(format!("{stem}.json")),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();
        std::fs::write(
            out.join(format!("{stem}.roundtrip.dna")),
            sc_format::snapgene::write(&seq),
        )
        .unwrap();
        let mut fresh = seq.clone();
        fresh.snapgene_extras = None;
        std::fs::write(
            out.join(format!("{stem}.fresh.dna")),
            sc_format::snapgene::write(&fresh),
        )
        .unwrap();
    }
    eprintln!("wrote {}", out.display());
}
