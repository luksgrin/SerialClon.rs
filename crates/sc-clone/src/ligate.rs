//! Blunting and ligation (Build a Construct).

use sc_seq::alphabet::reverse_complement;
use sc_seq::{Ends, Overhang, Sequence, Source, Topology};

use crate::{first_line, inherit_into};

/// Which end of a fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The 5' (left) end.
    Left,
    /// The 3' (right) end.
    Right,
}

/// How an end is made blunt — the original's two radio buttons next to
/// "Non-processed": *Blunt (Fill-In)* keeps the overhang bases (the extent
/// is kept and the enzyme label gets `-Blunt(filled)`), *Blunt (Remove)*
/// trims them off (`-Blunt(trimmed)`). The original offers both for either
/// kind of overhang and does not second-guess the choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluntMode {
    /// Fill in: the overhang bases stay.
    Fill,
    /// Remove: the overhang bases are trimmed.
    Remove,
}

/// Makes one end blunt the way a polymerase would: a 5' overhang is filled
/// in (the extent is kept), a 3' overhang is chewed back (its bases are
/// removed).
pub fn blunt(seq: &mut Sequence, side: Side) {
    let o = match side {
        Side::Left => &seq.ends.left,
        Side::Right => &seq.ends.right,
    };
    let mode = if o.length < 0 {
        BluntMode::Remove
    } else {
        BluntMode::Fill
    };
    blunt_with(seq, side, mode);
}

/// Makes one end blunt with an explicit [`BluntMode`]; the fragment's name
/// is not touched (the window labels the end `-Blunt(filled)` /
/// `-Blunt(trimmed)` as the original does).
pub fn blunt_with(seq: &mut Sequence, side: Side, mode: BluntMode) {
    let o = match side {
        Side::Left => &seq.ends.left,
        Side::Right => &seq.ends.right,
    };
    if o.is_blunt() {
        return;
    }
    label_blunt(&mut seq.comment, side, mode);
    let k = o.length.unsigned_abs() as usize;
    if mode == BluntMode::Remove {
        match side {
            Side::Left => {
                seq.bases.drain(..k.min(seq.bases.len()));
                for f in &mut seq.features {
                    f.start = f.start.saturating_sub(k as u32).max(1);
                    f.end = f.end.saturating_sub(k as u32).max(1);
                }
            }
            Side::Right => {
                let n = seq.bases.len();
                seq.bases.truncate(n.saturating_sub(k));
                let n = seq.bases.len() as u32;
                seq.features.retain(|f| f.start <= n && f.end <= n);
            }
        }
    }
    match side {
        Side::Left => seq.ends.left = Overhang::BLUNT,
        Side::Right => seq.ends.right = Overhang::BLUNT,
    }
}

/// True if the right end of `a` can ligate to the left end of `b`: both
/// blunt, or overhangs of the same kind and length whose protruding bases
/// are complementary.
pub fn compatible(a: &Overhang, b: &Overhang) -> bool {
    if a.is_blunt() && b.is_blunt() {
        return true;
    }
    if a.length != b.length {
        return false;
    }
    let ab: Vec<u8> = a.bases.iter().map(|x| x.to_ascii_uppercase()).collect();
    let bb: Vec<u8> = b.bases.iter().map(|x| x.to_ascii_uppercase()).collect();
    ab == reverse_complement(&bb)
}

/// Why a ligation failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LigationError {
    /// No fragment.
    #[error("nothing to ligate")]
    Empty,
    /// Two ends do not match; the message names them.
    #[error("{0}")]
    Incompatible(String),
}

fn incompatible(a: &Sequence, a_end: &Overhang, b: &Sequence, b_end: &Overhang) -> LigationError {
    LigationError::Incompatible(format!(
        "the right end of \"{}\" ({}) is not compatible with the left end of \"{}\" ({})",
        a.name,
        describe(a_end),
        b.name,
        describe(b_end)
    ))
}

/// Describes an end for messages.
pub fn describe(o: &Overhang) -> String {
    if o.is_blunt() {
        "blunt".into()
    } else {
        format!(
            "{}' {}",
            if o.is_five_prime() { 5 } else { 3 },
            String::from_utf8_lossy(&o.bases)
        )
    }
}

/// Joins `a` (left) and `b` (right), whose ends must be compatible. The
/// overhang duplex is counted once. Returns the bases and the offset of
/// `b`'s first base in the product.
fn join(a: &Sequence, b: &Sequence) -> Result<(Vec<u8>, usize), LigationError> {
    if !compatible(&a.ends.right, &b.ends.left) {
        return Err(incompatible(a, &a.ends.right, b, &b.ends.left));
    }
    let k = a.ends.right.length.unsigned_abs() as usize;
    let mut bases = a.bases.clone();
    bases.extend_from_slice(&b.bases[k.min(b.bases.len())..]);
    Ok((bases, a.bases.len() - k))
}

/// True if the construct closes on itself: the last fragment's right end
/// can ligate to the first fragment's left end. The original decides the
/// product's topology from this alone ("If the 5' end of the ligate
/// fragment is compatible with the 3' end, then a circular plasmid is
/// produced. Otherwise, a linear fragment is generated").
pub fn closes(fragments: &[Sequence]) -> bool {
    match (fragments.first(), fragments.last()) {
        (Some(a), Some(b)) => compatible(&b.ends.right, &a.ends.left),
        _ => false,
    }
}

/// Ligates fragments in order. With `circularize` the last right end is
/// joined to the first left end and the product is circular (a plasmid);
/// otherwise the product keeps the outer ends. Features inside every
/// fragment are inherited, and every fragment after the first gets an
/// `insert from <name>` feature spanning it, as the original's
/// `LigUpdateAnnot` adds; the comment records the assembly.
pub fn ligate(
    fragments: &[Sequence],
    circularize: bool,
    name: &str,
) -> Result<Sequence, LigationError> {
    let Some(first) = fragments.first() else {
        return Err(LigationError::Empty);
    };
    let mut product = Sequence {
        name: name.to_owned(),
        bases: first.bases.clone(),
        seq_type: first.seq_type,
        topology: Topology::Linear,
        comment: String::new(),
        features: Vec::new(),
        ends: first.ends.clone(),
        source: Source::New,
        strider_header: None,
        snapgene_extras: None,
    };
    inherit_into(&mut product.features, first, 0, first.len(), 0);
    let mut parts = vec![label(first)];
    for b in &fragments[1..] {
        let (bases, offset) = join(&product, b)?;
        product.bases = bases;
        product.ends.right = b.ends.right.clone();
        inherit_into(&mut product.features, b, 0, b.len(), offset);
        if !b.is_empty() {
            product.features.push(sc_seq::Feature {
                name: format!("insert from {}", b.name),
                description: first_line(&b.comment).to_owned(),
                kind: "misc_feature".into(),
                start: offset as u32 + 1,
                end: (offset + b.len()) as u32,
                strand: sc_seq::Strand::Forward,
                show: true,
                protect: false,
                arrow: false,
                color: sc_seq::Rgb(0x84, 0xa4, 0xc0),
            });
        }
        parts.push(label(b));
    }
    if circularize {
        if !compatible(&product.ends.right, &product.ends.left) {
            let last = fragments.last().unwrap();
            return Err(incompatible(
                last,
                &product.ends.right,
                first,
                &product.ends.left,
            ));
        }
        let k = product.ends.right.length.unsigned_abs() as usize;
        let n = product.bases.len();
        product.bases.truncate(n - k);
        let n = product.bases.len() as u32;
        // The last insert's feature reaches into the duplex now folded onto
        // the origin: trim it to the last base rather than lose it.
        for f in &mut product.features {
            if f.name.starts_with("insert from ") && f.start <= n && f.end > n {
                f.end = n;
            }
        }
        product.features.retain(|f| f.start <= n && f.end <= n);
        product.ends = Ends::default();
        product.topology = Topology::Circular;
    }
    // The original's comment (`CircGraphMap.GoGenerateFragment`): `Ligation
    // of : <fragment 1> to <fragment 2> and to <fragment 3>`, each fragment
    // as `<source> [<size>  nt] : (<enzyme 5'>[<pos>] / <enzyme 3'>[<pos>])`.
    let mut comment = format!("Ligation of : {}", parts[0]);
    for (i, p) in parts.iter().enumerate().skip(1) {
        comment.push_str(if i == 1 { " to " } else { " and to " });
        comment.push_str(p);
    }
    product.comment = comment;
    Ok(product)
}

/// The fragment as the original's Build a Construct window labels it: the
/// provenance of a fragment extracted between two sites, without its
/// `Fragment extracted from ` prefix; otherwise the name, the size label and
/// the two ends.
fn label(f: &Sequence) -> String {
    let c = first_line(&f.comment);
    match c.strip_prefix("Fragment extracted from ") {
        Some(rest) if rest.contains(" : (") => rest.to_owned(),
        _ => format!(
            "{} [{}  nt] : ({}[1] / {}[{}])",
            f.name,
            f.len(),
            describe(&f.ends.left),
            describe(&f.ends.right),
            f.len()
        ),
    }
}

/// Suffixes the extremity label of a `Fragment extracted from … : (<5'>[p] /
/// <3'>[p])` comment with `-Blunt(filled)` / `-Blunt(trimmed)`, as the
/// original's window relabels a processed end.
fn label_blunt(comment: &mut String, side: Side, mode: BluntMode) {
    let suffix = match mode {
        BluntMode::Fill => "-Blunt(filled)",
        BluntMode::Remove => "-Blunt(trimmed)",
    };
    let Some(open) = comment.find(" : (") else {
        return;
    };
    // The labels themselves may hold parentheses (`-Blunt(filled)`): the
    // closing one is the last of the line.
    let Some(close) = comment.rfind(')') else {
        return;
    };
    if close < open + 4 {
        return;
    }
    let inner = &comment[open + 4..close];
    let Some((left, right)) = inner.split_once(" / ") else {
        return;
    };
    let relabel = |s: &str| match s.find('[') {
        Some(b) if !s[..b].ends_with(suffix) => format!("{}{}{}", &s[..b], suffix, &s[b..]),
        _ => s.to_owned(),
    };
    let (left, right) = match side {
        Side::Left => (relabel(left), right.to_owned()),
        Side::Right => (left.to_owned(), relabel(right)),
    };
    comment.replace_range(open + 4..close, &format!("{left} / {right}"));
}

/// The 0-based positions after which a junction follows in the product of
/// [`ligate`] or of a Gateway reaction: both ends of every `insert from`
/// feature, plus the closing junction of a circular product (the base before
/// the origin). The original rescans the product for features around every
/// junction (`SetAnnotations(seq, from, to, false)`).
pub fn junctions(product: &Sequence) -> Vec<usize> {
    let mut out: Vec<usize> = product
        .features
        .iter()
        .filter(|f| f.name.starts_with("insert from ") || f.name.starts_with("Insert from "))
        .flat_map(|f| [f.start as usize - 1, f.end as usize])
        .filter(|&j| j > 0 && j < product.len())
        .collect();
    if product.topology.is_circular() && !product.is_empty() {
        out.push(product.len());
    }
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_seq::{Feature, Rgb, Strand};

    fn frag(name: &str, bases: &str, left: Overhang, right: Overhang) -> Sequence {
        let mut s = Sequence::new(name, bases.as_bytes().to_vec());
        s.ends = Ends { left, right };
        s
    }

    fn feature(name: &str, start: u32, end: u32) -> Feature {
        Feature {
            name: name.into(),
            description: String::new(),
            kind: "misc_feature".into(),
            start,
            end,
            strand: Strand::Forward,
            show: true,
            protect: false,
            arrow: true,
            color: Rgb(0, 0, 0),
        }
    }

    #[test]
    fn compatibility_rules() {
        assert!(compatible(&Overhang::BLUNT, &Overhang::BLUNT));
        assert!(compatible(
            &Overhang::five_prime(b"GATC"),
            &Overhang::five_prime(b"GATC")
        )); // BamHI / BglII
        assert!(compatible(
            &Overhang::five_prime(b"AGGT"),
            &Overhang::five_prime(b"ACCT")
        ));
        assert!(!compatible(
            &Overhang::five_prime(b"GATC"),
            &Overhang::five_prime(b"AATT")
        ));
        assert!(!compatible(
            &Overhang::five_prime(b"GATC"),
            &Overhang::three_prime(b"GATC")
        ));
        assert!(!compatible(
            &Overhang::BLUNT,
            &Overhang::five_prime(b"GATC")
        ));
    }

    #[test]
    fn blunting() {
        let mut s = frag(
            "f",
            "AATTCaaaG",
            Overhang::five_prime(b"AATT"),
            Overhang::three_prime(b"G"),
        );
        blunt(&mut s, Side::Left);
        assert_eq!(s.bases, b"AATTCaaaG");
        assert!(s.ends.left.is_blunt());
        blunt(&mut s, Side::Right);
        assert_eq!(s.bases, b"AATTCaaa");
        assert!(s.ends.is_blunt());
    }

    #[test]
    fn cohesive_ligation_counts_the_duplex_once_and_inherits_features() {
        let mut a = frag(
            "vec",
            "ccccGATC",
            Overhang::BLUNT,
            Overhang::five_prime(b"GATC"),
        );
        a.features.push(feature("v", 1, 4));
        let mut b = frag(
            "ins",
            "GATCtttt",
            Overhang::five_prime(b"GATC"),
            Overhang::BLUNT,
        );
        b.features.push(feature("i", 5, 8));
        let p = ligate(&[a, b], false, "p").unwrap();
        assert_eq!(p.bases, b"ccccGATCtttt");
        assert!(p.ends.is_blunt());
        // The inherited features plus the original's "insert from" feature
        // over the insert (from the junction's first duplicated base).
        assert_eq!(p.features.len(), 3);
        assert_eq!((p.features[1].start, p.features[1].end), (9, 12));
        assert_eq!(p.features[2].name, "insert from ins");
        assert_eq!((p.features[2].start, p.features[2].end), (5, 12));
        // The original's comment: fragments without an extraction provenance
        // are labelled with their name, size and ends.
        assert_eq!(
            p.comment,
            "Ligation of : vec [8  nt] : (blunt[1] / 5' GATC[8]) to ins [8  nt] : (5' GATC[1] / blunt[8])"
        );
        assert_eq!(junctions(&p), vec![4]);
    }

    #[test]
    fn comment_follows_the_original_templates_and_blunting_relabels_the_ends() {
        // Fragments as the Graphic Map extracts them carry the original's
        // `Fragment extracted from …` line, which the ligation reuses.
        let mut a = frag(
            "DNA Fragment #1",
            "AATTCggggGATC",
            Overhang::five_prime(b"AATT"),
            Overhang::five_prime(b"GATC"),
        );
        a.comment = "Fragment extracted from pVec [13  nt] : (EcoRI[396] / BamHI[1402])".into();
        let mut b = frag(
            "DNA Fragment #2",
            "GATCttttGATC",
            Overhang::five_prime(b"GATC"),
            Overhang::five_prime(b"GATC"),
        );
        b.comment = "Fragment extracted from pIns [12  nt] : (BglII[10] / BamHI[220])".into();
        let mut c = frag(
            "DNA Fragment #3",
            "GATCaaa",
            Overhang::five_prime(b"GATC"),
            Overhang::BLUNT,
        );
        c.comment = "Fragment extracted from pX [7  nt] : (BamHI[5] / SmaI[11])".into();
        let p = ligate(&[a.clone(), b.clone(), c.clone()], false, "Ligation #1").unwrap();
        assert_eq!(
            p.comment,
            "Ligation of : pVec [13  nt] : (EcoRI[396] / BamHI[1402]) to pIns [12  nt] : (BglII[10] / BamHI[220]) and to pX [7  nt] : (BamHI[5] / SmaI[11])"
        );
        // The insert feature's note is the fragment's provenance line.
        let ins = p
            .features
            .iter()
            .find(|f| f.name == "insert from DNA Fragment #2")
            .unwrap();
        assert_eq!(ins.description, b.comment);
        assert_eq!(junctions(&p), vec![9, 17, 21]);

        // Blunting relabels the processed extremity, once.
        blunt_with(&mut a, Side::Left, BluntMode::Fill);
        blunt_with(&mut a, Side::Left, BluntMode::Fill);
        assert_eq!(
            a.comment,
            "Fragment extracted from pVec [13  nt] : (EcoRI-Blunt(filled)[396] / BamHI[1402])"
        );
        blunt_with(&mut a, Side::Right, BluntMode::Remove);
        assert_eq!(
            a.comment,
            "Fragment extracted from pVec [13  nt] : (EcoRI-Blunt(filled)[396] / BamHI-Blunt(trimmed)[1402])"
        );
        // A circular product has a closing junction at the origin.
        let v = frag(
            "v",
            "GATCccccGATC",
            Overhang::five_prime(b"GATC"),
            Overhang::five_prime(b"GATC"),
        );
        let i = frag(
            "i",
            "GATCttttGATC",
            Overhang::five_prime(b"GATC"),
            Overhang::five_prime(b"GATC"),
        );
        let circ = ligate(&[v, i], true, "c").unwrap();
        assert_eq!(junctions(&circ), vec![8, 16]);
    }

    #[test]
    fn blunt_modes_fill_or_trim_either_overhang_and_closes_detects_a_plasmid() {
        let mut fill = frag(
            "f",
            "AATTCggg",
            Overhang::five_prime(b"AATT"),
            Overhang::BLUNT,
        );
        blunt_with(&mut fill, Side::Left, BluntMode::Fill);
        assert_eq!(fill.bases, b"AATTCggg");
        assert!(fill.ends.left.is_blunt());
        let mut trim = frag(
            "t",
            "AATTCggg",
            Overhang::five_prime(b"AATT"),
            Overhang::BLUNT,
        );
        blunt_with(&mut trim, Side::Left, BluntMode::Remove);
        assert_eq!(trim.bases, b"Cggg");
        // A 3' overhang may be "filled" too: the original does not refuse.
        let mut keep = frag(
            "k",
            "gggGTAC",
            Overhang::BLUNT,
            Overhang::three_prime(b"GTAC"),
        );
        blunt_with(&mut keep, Side::Right, BluntMode::Fill);
        assert_eq!(keep.bases, b"gggGTAC");
        assert!(keep.ends.right.is_blunt());

        let a = frag(
            "a",
            "GATCcc",
            Overhang::five_prime(b"GATC"),
            Overhang::BLUNT,
        );
        let b = frag(
            "b",
            "ttGATC",
            Overhang::BLUNT,
            Overhang::five_prime(b"GATC"),
        );
        assert!(closes(&[a.clone(), b.clone()]));
        assert!(!closes(&[a.clone(), a.clone()]));
        assert!(!closes(std::slice::from_ref(&b)));
        assert!(!closes(&[]));
    }

    #[test]
    fn circular_construct() {
        let a = frag(
            "vec",
            "AATTCggggG",
            Overhang::five_prime(b"AATT"),
            Overhang::three_prime(b"G"),
        );
        // b's left 3' overhang is carried by the bottom strand, read 5'→3': C.
        let b = frag(
            "ins",
            "GttttGAATT",
            Overhang::three_prime(b"C"),
            Overhang::five_prime(b"AATT"),
        );
        let p = ligate(&[a, b], true, "construct").unwrap();
        assert_eq!(p.topology, Topology::Circular);
        assert_eq!(p.bases, b"AATTCggggGttttG");
        assert!(p.ends.is_blunt());
        let bad = frag("x", "CCCC", Overhang::five_prime(b"GATC"), Overhang::BLUNT);
        assert!(matches!(
            ligate(&[bad], true, "c"),
            Err(LigationError::Incompatible(..))
        ));
        assert!(matches!(ligate(&[], true, "c"), Err(LigationError::Empty)));
    }
}
