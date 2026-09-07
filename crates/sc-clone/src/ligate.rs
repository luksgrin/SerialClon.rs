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

/// Makes one end blunt the way a polymerase would: a 5' overhang is filled
/// in (the extent is kept), a 3' overhang is chewed back (its bases are
/// removed).
pub fn blunt(seq: &mut Sequence, side: Side) {
    let o = match side {
        Side::Left => &seq.ends.left,
        Side::Right => &seq.ends.right,
    };
    if o.is_blunt() {
        return;
    }
    let k = o.length.unsigned_abs() as usize;
    if o.length < 0 {
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

/// Ligates fragments in order. With `circularize` the last right end is
/// joined to the first left end and the product is circular (a plasmid);
/// otherwise the product keeps the outer ends. Features inside every
/// fragment are inherited; the comment records the assembly.
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
    };
    inherit_into(&mut product.features, first, 0, first.len(), 0);
    let mut parts = vec![format!("{} [{} bp]", first.name, first.len())];
    for b in &fragments[1..] {
        let (bases, offset) = join(&product, b)?;
        product.bases = bases;
        product.ends.right = b.ends.right.clone();
        inherit_into(&mut product.features, b, 0, b.len(), offset);
        parts.push(format!("{} [{} bp]", b.name, b.len()));
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
        product.features.retain(|f| f.start <= n && f.end <= n);
        product.ends = Ends::default();
        product.topology = Topology::Circular;
    }
    product.comment = format!(
        "{} ligation of {}: {}\r{}",
        if circularize { "Circular" } else { "Linear" },
        fragments.len(),
        parts.join(" + "),
        fragments
            .iter()
            .map(|f| first_line(&f.comment))
            .filter(|c| !c.is_empty())
            .collect::<Vec<_>>()
            .join(" | ")
    );
    Ok(product)
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
        assert_eq!(p.features.len(), 2);
        assert_eq!((p.features[1].start, p.features[1].end), (9, 12));
        assert!(
            p.comment
                .starts_with("Linear ligation of 2: vec [8 bp] + ins [8 bp]")
        );
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
