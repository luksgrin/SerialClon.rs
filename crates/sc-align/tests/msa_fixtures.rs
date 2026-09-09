//! Multiple alignment of the shipped plasmids: the two pBluescripts share
//! their backbone, pBR322 shares the ColE1 origin and bla.

use sc_align::multiple::{Input, Mode, Options, align_multiple};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");

fn read(name: &str) -> Input {
    let seq = sc_format::read_file(std::path::Path::new(FIXTURES).join(name).as_path())
        .unwrap()
        .remove(0);
    Input {
        name: seq.name.clone(),
        bases: seq.bases.clone(),
        seq_type: seq.seq_type,
    }
}

#[test]
fn plasmids_align_and_the_bluescripts_share_their_backbone() {
    let inputs = [
        read("pBluescript-II-KS+.xdna"),
        read("pBluescript-II-SK+.gb"),
        read("pBR322.xdna"),
    ];
    let msa = align_multiple(
        &inputs,
        &Options {
            mode: Mode::Auto,
            adjust_direction: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(msa.mode, Mode::FftNs2, "auto picks FFT-NS-2 above 2 kb");
    assert_eq!(msa.rows.len(), 3);
    assert!(msa.length >= 4361);
    for (r, i) in msa.rows.iter().zip(&inputs) {
        assert_eq!(r.name, i.name);
        assert_eq!(r.length, i.bases.len());
        assert_eq!(r.aligned.replace('-', "").len(), i.bases.len());
    }
    // KS(+) and SK(+) differ only by the polylinker orientation: their rows
    // are identical at the vast majority of columns where both have a residue.
    let (a, b) = (
        msa.rows[0].aligned.as_bytes(),
        msa.rows[1].aligned.as_bytes(),
    );
    let both: Vec<(u8, u8)> = a
        .iter()
        .zip(b)
        .filter(|(x, y)| **x != b'-' && **y != b'-')
        .map(|(x, y)| (x.to_ascii_uppercase(), y.to_ascii_uppercase()))
        .collect();
    let same = both.iter().filter(|(x, y)| x == y).count();
    assert!(
        same as f64 / both.len() as f64 > 0.9,
        "{same}/{}",
        both.len()
    );
    // pBR322 aligns its bla / ColE1 region: well over a thousand columns match KS(+).
    let (a, c) = (
        msa.rows[0].aligned.as_bytes(),
        msa.rows[2].aligned.as_bytes(),
    );
    let shared = a
        .iter()
        .zip(c)
        .filter(|(x, y)| **x != b'-' && **y != b'-' && x.eq_ignore_ascii_case(y))
        .count();
    assert!(shared > 1200, "{shared} shared columns");
    assert_eq!(msa.marks.len(), msa.length);
}
