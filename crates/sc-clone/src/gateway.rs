//! Gateway™ cloning (User Manual §III.E, changelog 1.02 / 1.2 / 2.5 / 2.6):
//! the BP and LR reactions, and yeast-style homologous recombination.
//!
//! Every att site of the four kinds carries the palindromic 8-mer
//! `TTGTACAA` at its centre; the strand exchange is modelled at that core,
//! which is what makes attB + attP give attL, and attL + attR give attB.
//! Sites are located with the definitions of the Gateway Collection
//! (`resources/features/gateway.json`), so the reaction works on any
//! sequence the scan recognises.

use sc_features::Collection;
use sc_seq::alphabet::{mask, reverse_complement};
use sc_seq::{Sequence, Source, Topology};

use crate::{antiparallel, first_line, inherit_into};

/// The reaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    /// attB × attP → attL (entry clone).
    Bp,
    /// attL × attR → attB (expression clone).
    Lr,
}

/// Why a reaction failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GatewayError {
    /// A required site is missing.
    #[error("no {0} site found in \"{1}\"")]
    MissingSite(String, String),
    /// A site appears more than once.
    #[error("{0} appears {2} times in \"{1}\"")]
    Ambiguous(String, String, usize),
    /// The sites are arranged in a way the reaction cannot use.
    #[error("{0}")]
    Invalid(String),
}

const CORE: &[u8] = b"TTGTACAA";

/// Which att definitions locate the sites: the original's "Use minimal att
/// consensus" checkbox (`GateWayWin.attDefCheckBox` → `InitGatewaySeq`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttConsensus {
    /// The full definitions of the Gateway Feature Collection.
    #[default]
    Full,
    /// The original's minimal consensus, the same for every kind of site:
    /// `TTTGTACAAAAAAG` for site 1 and `CTTTCTTGTACAAAGT` (antiparallel,
    /// i.e. `ACTTTGTACAAGAAAG` on the sense strand) for site 2.
    Minimal,
}

/// The original's minimal site-1 consensus.
const MINIMAL_SITE1: &[u8] = b"TTTGTACAAAAAAG";
/// The original's minimal site-2 consensus, read on the sense strand.
const MINIMAL_SITE2: &[u8] = b"ACTTTGTACAAGAAAG";

/// A located site: its span and the crossover point (0-based, before base).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Site {
    name: String,
    lo: usize,
    cross: usize,
    forward: bool,
}

/// Mismatches tolerated when matching a site definition of `len` bases:
/// none for the 25-bp attB cores, a few for the long attP/attL/attR arms,
/// which vary by a base or two between real vectors.
fn allowance(len: usize) -> usize {
    len / 30
}

/// Occurrences of a definition on one strand: windows containing the core
/// exactly and matching the definition within the allowance, the best of
/// each cluster of overlapping candidates, as (start, mismatches).
fn windows(hay: &[u8], def: &[u8]) -> Vec<(usize, usize)> {
    let l = def.len();
    if l == 0 || hay.len() < l {
        return Vec::new();
    }
    let allow = allowance(l);
    let mut best: Vec<(usize, usize)> = Vec::new();
    for i in 0..=hay.len() - l {
        let w = &hay[i..i + l];
        let mut mism = 0;
        for (&a, &d) in w.iter().zip(def) {
            if mask(a) == 0 || mask(a) & !mask(d) != 0 {
                mism += 1;
                if mism > allow {
                    break;
                }
            }
        }
        if mism > allow || !w.windows(CORE.len()).any(|c| c == CORE) {
            continue;
        }
        match best.last_mut() {
            Some(last) if i < last.0 + l => {
                if mism < last.1 {
                    *last = (i, mism);
                }
            }
            _ => best.push((i, mism)),
        }
    }
    best
}

fn find_site(
    seq: &Sequence,
    collection: &Collection,
    prefix: &str,
    number: u8,
    consensus: AttConsensus,
) -> Result<Option<Site>, GatewayError> {
    let wanted = format!("{prefix}{number}").to_ascii_lowercase();
    let defs: Vec<Vec<u8>> = match consensus {
        AttConsensus::Minimal => vec![if number == 1 {
            MINIMAL_SITE1.to_vec()
        } else {
            MINIMAL_SITE2.to_vec()
        }],
        AttConsensus::Full => collection
            .features
            .iter()
            .filter(|d| d.name.to_ascii_lowercase() == wanted && !d.peptide)
            .map(|d| d.five.bytes().map(|b| b.to_ascii_uppercase()).collect())
            .collect(),
    };
    if defs.is_empty() {
        return Err(GatewayError::Invalid(format!(
            "the Gateway Collection has no {prefix}{number} definition"
        )));
    }
    let upper: Vec<u8> = seq.bases.iter().map(|b| b.to_ascii_uppercase()).collect();
    let rc = reverse_complement(&upper);
    let n = upper.len();
    // Every candidate with its mismatch count; att1 and att2 sites share
    // their arms and differ only around the core, so the best window wins
    // unless another one scores within a mismatch of it.
    let mut hits: Vec<(Site, usize)> = Vec::new();
    for def in &defs {
        for (i, m) in windows(&upper, def) {
            let c = upper[i..i + def.len()]
                .windows(CORE.len())
                .position(|w| w == CORE)
                .unwrap_or(0);
            hits.push((
                Site {
                    name: format!("{prefix}{number}"),
                    lo: i,
                    cross: i + c + 4,
                    forward: true,
                },
                m,
            ));
        }
        for (i, m) in windows(&rc, def) {
            // rc index i..i+l ↔ forward n-i-l..n-i; the palindromic core maps
            // onto itself, so the crossover point maps as n - cross.
            let c = rc[i..i + def.len()]
                .windows(CORE.len())
                .position(|w| w == CORE)
                .unwrap_or(0);
            hits.push((
                Site {
                    name: format!("{prefix}{number}"),
                    lo: n - i - def.len(),
                    cross: n - (i + c + 4),
                    forward: false,
                },
                m,
            ));
        }
    }
    hits.sort_by_key(|(h, m)| (*m, h.lo));
    hits.dedup_by(|a, b| a.0.lo.abs_diff(b.0.lo) < 20);
    match hits.len() {
        0 => Ok(None),
        1 => Ok(hits.pop().map(|(h, _)| h)),
        // The original's attP1 and attP2 definitions differ by a single
        // base, so a runner-up one mismatch behind is the *other* site, not
        // an ambiguity; only a tie is.
        m if hits[1].1 == hits[0].1 => Err(GatewayError::Ambiguous(
            format!("{prefix}{number}"),
            seq.name.clone(),
            m,
        )),
        _ => Ok(hits.into_iter().next().map(|(h, _)| h)),
    }
}

fn sites(
    seq: &Sequence,
    collection: &Collection,
    prefix: &str,
    consensus: AttConsensus,
) -> Result<(Site, Site), GatewayError> {
    let s1 = find_site(seq, collection, prefix, 1, consensus)?
        .ok_or_else(|| GatewayError::MissingSite(format!("{prefix}1"), seq.name.clone()))?;
    let s2 = find_site(seq, collection, prefix, 2, consensus)?
        .ok_or_else(|| GatewayError::MissingSite(format!("{prefix}2"), seq.name.clone()))?;
    Ok((s1, s2))
}

/// Finds site 1 and site 2 of a kind, reverse-complementing the sequence
/// when site 1 reads backwards (the "antisense destination vector" case of
/// changelog 2.6), and rotating a circular sequence so that site 1 precedes
/// site 2.
fn locate_pair(
    seq: &Sequence,
    collection: &Collection,
    prefix: &str,
    consensus: AttConsensus,
) -> Result<(Sequence, Site, Site), GatewayError> {
    let (s1, _) = sites(seq, collection, prefix, consensus)?;
    let mut seq = seq.clone();
    if !s1.forward {
        seq = antiparallel(&seq);
    }
    let (s1, s2) = sites(&seq, collection, prefix, consensus)?;
    if s1.cross > s2.cross {
        if !seq.topology.is_circular() {
            return Err(GatewayError::Invalid(format!(
                "{prefix}2 precedes {prefix}1 in \"{}\"",
                seq.name
            )));
        }
        seq.set_origin(s1.lo as u32 + 1);
        let (s1, s2) = sites(&seq, collection, prefix, consensus)?;
        if s1.cross > s2.cross {
            return Err(GatewayError::Invalid(format!(
                "{prefix}1 and {prefix}2 read in opposite directions in \"{}\"",
                seq.name
            )));
        }
        return Ok((seq, s1, s2));
    }
    Ok((seq, s1, s2))
}

/// Runs a BP (`insert` has attB1/attB2, `vector` attP1/attP2) or LR
/// (`insert` attL1/attL2, `vector` attR1/attR2) reaction: the vector's
/// region between its two crossover points is replaced by the insert's.
pub fn recombine(
    reaction: Reaction,
    insert: &Sequence,
    vector: &Sequence,
    collection: &Collection,
) -> Result<Sequence, GatewayError> {
    recombine_with(reaction, insert, vector, collection, AttConsensus::Full)
}

/// [`recombine`] with the sites located by the chosen [`AttConsensus`].
pub fn recombine_with(
    reaction: Reaction,
    insert: &Sequence,
    vector: &Sequence,
    collection: &Collection,
    consensus: AttConsensus,
) -> Result<Sequence, GatewayError> {
    let (ins_prefix, vec_prefix, product_kind) = match reaction {
        Reaction::Bp => ("attB", "attP", "Entry clone"),
        Reaction::Lr => ("attL", "attR", "Expression clone"),
    };
    let (ins, i1, i2) = locate_pair(insert, collection, ins_prefix, consensus)?;
    let (vec, v1, v2) = locate_pair(vector, collection, vec_prefix, consensus)?;
    let mut bases = vec.bases[..v1.cross].to_vec();
    bases.extend_from_slice(&ins.bases[i1.cross..i2.cross]);
    bases.extend_from_slice(&vec.bases[v2.cross..]);
    let mut features = Vec::new();
    inherit_into(&mut features, &vec, 0, v1.cross, 0);
    inherit_into(&mut features, &ins, i1.cross, i2.cross, v1.cross);
    let tail_offset = v1.cross + (i2.cross - i1.cross);
    inherit_into(&mut features, &vec, v2.cross, vec.len(), tail_offset);
    let n = bases.len() as u32;
    features.retain(|f| f.start <= n && f.end <= n);
    let reaction_name = match reaction {
        Reaction::Bp => "BP",
        Reaction::Lr => "LR",
    };
    // The original's GateWayAnnotate marks the inserted region.
    if tail_offset > v1.cross {
        features.push(sc_seq::Feature {
            name: format!("Insert from {}", insert.name),
            description: format!(
                "Fragment inserted from {} into {} [{}] reaction",
                insert.name, vector.name, reaction_name
            ),
            kind: "misc_feature".into(),
            start: v1.cross as u32 + 1,
            end: tail_offset as u32,
            strand: sc_seq::Strand::Forward,
            show: true,
            protect: false,
            arrow: false,
            color: sc_seq::Rgb(0x84, 0xa4, 0xc0),
        });
    }
    Ok(Sequence {
        name: format!("{} ({} x {})", product_kind, insert.name, vector.name),
        bases,
        seq_type: sc_seq::SeqType::Dna,
        topology: vec.topology,
        comment: format!(
            "Gateway {} reaction: {} [{} bp, {}..{}] into {} [{} bp], replacing {}..{}\r{}\r{}",
            match reaction {
                Reaction::Bp => "BP",
                Reaction::Lr => "LR",
            },
            insert.name,
            insert.len(),
            i1.name,
            i2.name,
            vector.name,
            vector.len(),
            v1.name,
            v2.name,
            first_line(&insert.comment),
            first_line(&vector.comment)
        ),
        features,
        ends: ins.ends.clone(),
        source: Source::New,
        strider_header: None,
        snapgene_extras: None,
    })
}

/// Yeast-style homologous recombination: the insert's two ends share
/// homology with the vector; the vector's region between the homology arms
/// is replaced by the insert. `min_homology` bases of each end must match.
pub fn homologous_recombination(
    insert: &Sequence,
    vector: &Sequence,
    min_homology: usize,
) -> Result<Sequence, GatewayError> {
    let n = vector.len();
    let up: Vec<u8> = vector
        .bases
        .iter()
        .map(|b| b.to_ascii_uppercase())
        .collect();
    let ins: Vec<u8> = insert
        .bases
        .iter()
        .map(|b| b.to_ascii_uppercase())
        .collect();
    let min = min_homology.max(1);
    let find = |needle: &[u8], from: usize| {
        up[from.min(up.len())..]
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|p| p + from)
    };
    let mut left = None;
    for k in (min..=ins.len().min(60)).rev() {
        if let Some(p) = find(&ins[..k], 0) {
            left = Some((p, k));
            break;
        }
    }
    let (l_pos, l_len) = left.ok_or_else(|| {
        GatewayError::MissingSite(format!("{min}-bp 5' homology arm"), vector.name.clone())
    })?;
    let mut right = None;
    for k in (min..=ins.len().min(60)).rev() {
        if let Some(p) = find(&ins[ins.len() - k..], l_pos + l_len) {
            right = Some((p, k));
            break;
        }
    }
    let (r_pos, r_len) = right.ok_or_else(|| {
        GatewayError::MissingSite(
            format!("{min}-bp 3' homology arm downstream of the 5' arm"),
            vector.name.clone(),
        )
    })?;
    let mut bases = vector.bases[..l_pos].to_vec();
    bases.extend_from_slice(&insert.bases);
    bases.extend_from_slice(&vector.bases[r_pos + r_len..]);
    let mut features = Vec::new();
    inherit_into(&mut features, vector, 0, l_pos, 0);
    inherit_into(&mut features, insert, 0, insert.len(), l_pos);
    inherit_into(
        &mut features,
        vector,
        r_pos + r_len,
        n,
        l_pos + insert.len(),
    );
    Ok(Sequence {
        name: format!("Recombinant ({} x {})", insert.name, vector.name),
        bases,
        seq_type: sc_seq::SeqType::Dna,
        topology: if vector.topology.is_circular() {
            Topology::Circular
        } else {
            Topology::Linear
        },
        comment: format!(
            "Homologous recombination of {} [{} bp] into {} [{} bp] between the {}-bp arm at {} and the {}-bp arm at {}\r{}",
            insert.name,
            insert.len(),
            vector.name,
            n,
            l_len,
            l_pos + 1,
            r_len,
            r_pos + 1,
            first_line(&vector.comment)
        ),
        features,
        ends: Default::default(),
        source: Source::New,
        strider_header: None,
        snapgene_extras: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_features::builtin_collections;
    use sc_seq::{Feature, Strand};

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/sequences");

    fn gateway() -> Collection {
        builtin_collections()
            .into_iter()
            .find(|c| c.name == "Gateway")
            .unwrap()
    }
    fn read(name: &str) -> Sequence {
        sc_format::xdna::read(&std::fs::read(format!("{FIXTURES}/{name}")).unwrap(), name).unwrap()
    }
    fn def(c: &Collection, name: &str) -> String {
        c.features
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case(name))
            .unwrap()
            .five
            .clone()
    }
    const ORF: &str = "ATGGCTAGCAAAGGAGAAGAACTTTTCACTGGAGTTGTCCCAATTCTTGTTGAATTAGATGGTGATGTTAATGGGCACAAATTTTCTGTCAGTGGAGAGGGTGAAGGTTGA";

    fn attb_insert(c: &Collection) -> Sequence {
        // Concrete attB1/attB2 sites (Invitrogen's), which the recovered
        // degenerate definitions must recognise.
        const ATTB1: &str = "ACAAGTTTGTACAAAAAAGCAGGCT";
        const ATTB2: &str = "ACCCAGCTTTCTTGTACAAAGTGGT";
        assert_eq!(def(c, "AttB1").len(), ATTB1.len());
        let mut insert = Sequence::new("attB-GFP", format!("{ATTB1}{ORF}{ATTB2}").into_bytes());
        insert.features.push(Feature {
            name: "orf".into(),
            description: String::new(),
            kind: "CDS".into(),
            start: 26,
            end: 25 + ORF.len() as u32,
            strand: Strand::Forward,
            show: true,
            protect: false,
            arrow: true,
            color: sc_seq::Rgb(0, 0, 0),
        });
        insert
    }

    #[test]
    fn bp_then_lr_on_the_shipped_gateway_vectors() {
        let c = gateway();
        let insert = attb_insert(&c);
        let donor = read("pDONR201.xdna");
        let entry = recombine(Reaction::Bp, &insert, &donor, &c).unwrap();
        assert_eq!(entry.topology, Topology::Circular);
        // The original's GateWayAnnotate feature over the inserted region.
        let ins = entry
            .features
            .iter()
            .find(|f| f.name == "Insert from attB-GFP")
            .expect("Insert from feature");
        assert_eq!(
            ins.description,
            "Fragment inserted from attB-GFP into pDONR201.xdna [BP] reaction"
        );
        assert!(ins.end > ins.start);
        assert!(
            String::from_utf8_lossy(&entry.bases)
                .to_ascii_uppercase()
                .contains(ORF)
        );
        assert!(!entry.features.iter().any(|f| f.name == "ccdB"));
        assert!(entry.features.iter().any(|f| f.name == "orf"));
        // attL junctions: the donor's P arm followed by the attB core ends.
        let text = String::from_utf8_lossy(&entry.bases).to_ascii_uppercase();
        assert!(
            text.contains("ATGCCAACTTTGTACAAAAAAGCAGGCT"),
            "attL1 junction missing"
        );
        assert!(
            text.contains("ACCCAGCTTTCTTGTACAAAGTTGG"),
            "attL2 junction missing"
        );
        let dest = read("pDEST14.xdna");
        let expr = recombine(Reaction::Lr, &entry, &dest, &c).unwrap();
        assert!(
            String::from_utf8_lossy(&expr.bases)
                .to_ascii_uppercase()
                .contains(ORF)
        );
        // The definitions are degenerate (W, R, K), so match IUPAC-wise.
        let text = String::from_utf8_lossy(&expr.bases).to_ascii_uppercase();
        let regenerated = |name: &str| {
            let d = def(&c, name).into_bytes();
            text.as_bytes()
                .windows(d.len())
                .any(|w| sc_seq::alphabet::matches_at(&d, w))
        };
        assert!(regenerated("AttB1"), "attB1 should be regenerated by LR");
        assert!(regenerated("AttB2"), "attB2 should be regenerated by LR");
        assert!(expr.features.iter().any(|f| f.name == "orf"));
        assert!(expr.comment.starts_with("Gateway LR reaction"));
        assert!(matches!(
            recombine(Reaction::Lr, &insert, &dest, &c),
            Err(GatewayError::MissingSite(..))
        ));
    }

    #[test]
    fn minimal_att_consensus_gives_the_same_entry_clone() {
        // The original's "Use minimal att consensus": the short core
        // consensus locates every kind of site, here on the shipped donor
        // vector and the concrete attB insert, with the same product as the
        // full definitions.
        let c = gateway();
        let insert = attb_insert(&c);
        let donor = read("pDONR201.xdna");
        let full = recombine(Reaction::Bp, &insert, &donor, &c).unwrap();
        let minimal =
            recombine_with(Reaction::Bp, &insert, &donor, &c, AttConsensus::Minimal).unwrap();
        assert_eq!(minimal.bases, full.bases);
        // A vector whose arms differ from the definitions is still handled
        // with the minimal consensus: a synthetic donor built from the cores
        // only, which the full 232-bp attP definitions cannot match.
        let synthetic = Sequence::new(
            "synthetic",
            format!(
                "gggggggggg{}ccdBccdBccdB{}tttttttttt",
                std::str::from_utf8(MINIMAL_SITE1).unwrap(),
                std::str::from_utf8(MINIMAL_SITE2).unwrap()
            )
            .into_bytes(),
        );
        assert!(recombine(Reaction::Bp, &insert, &synthetic, &c).is_err());
        let p =
            recombine_with(Reaction::Bp, &insert, &synthetic, &c, AttConsensus::Minimal).unwrap();
        assert!(
            !p.bases.windows(4).any(|w| w == b"ccdB"),
            "the ccdB stuffer is replaced"
        );
        assert!(p.bases.windows(ORF.len()).any(|w| w == ORF.as_bytes()));
    }

    #[test]
    fn antisense_insert_is_flipped() {
        let c = gateway();
        let insert = attb_insert(&c);
        let flipped = antiparallel(&insert);
        let donor = read("pDONR201.xdna");
        let a = recombine(Reaction::Bp, &insert, &donor, &c).unwrap();
        let b = recombine(Reaction::Bp, &flipped, &donor, &c).unwrap();
        assert_eq!(a.bases.to_ascii_uppercase(), b.bases.to_ascii_uppercase());
    }

    #[test]
    fn homologous_recombination_replaces_between_arms() {
        let arm1 = "TTGACCGGTTAAGGCCAATTGG";
        let arm2 = "ATCGTAGCTAGCTAGCTAGCTA";
        let vector = Sequence::new("v", format!("gg{arm1}GGGGGGGGGG{arm2}cc").into_bytes());
        let insert = Sequence::new("i", format!("{arm1}cccccc{arm2}").into_bytes());
        let p = homologous_recombination(&insert, &vector, 15).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&p.bases),
            format!("gg{arm1}cccccc{arm2}cc")
        );
        assert!(
            homologous_recombination(
                &Sequence::new("x", b"CCCCCCCCCCCCCCCCCCCC".to_vec()),
                &vector,
                15
            )
            .is_err()
        );
    }
}
