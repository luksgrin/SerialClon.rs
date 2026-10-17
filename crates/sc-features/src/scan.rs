//! The scan engine.

use sc_seq::alphabet::{matches_at, reverse_complement};
use sc_seq::translate::STANDARD;
use sc_seq::{Feature, Sequence, Strand};

use crate::{Collection, FeatureDef, feature_from};

/// A raw match on one strand of a (possibly doubled) sequence: 0-based,
/// half-open, in the coordinates of the scanned string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hit {
    start: usize,
    end: usize,
}

/// Every occurrence of an IUPAC pattern in `hay`.
fn find_all(hay: &[u8], pattern: &[u8], max_start: usize) -> Vec<usize> {
    let len = pattern.len();
    if len == 0 || hay.len() < len {
        return Vec::new();
    }
    (0..=(hay.len() - len).min(max_start))
        .filter(|&i| matches_at(pattern, &hay[i..i + len]))
        .collect()
}

fn upper(s: &str) -> Vec<u8> {
    s.bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect()
}

/// Like [`upper`] but keeps `#`, the original's gap wildcard: its
/// definition-to-regex table maps every IUPAC letter to a character class,
/// `#` to `.*`, and drops any other character (`>`, digits, spaces).
fn upper_dna(s: &str) -> Vec<u8> {
    s.bytes()
        .filter(|b| b.is_ascii_alphabetic() || *b == b'#')
        .map(|b| b.to_ascii_uppercase())
        .collect()
}

/// Every span matched by a consensus that may contain `#` gaps, as
/// `(start, end)` half-open. A gap behaves like a greedy `.*` between two
/// literal parts (what the original's regex does): the part after the gap
/// is taken at its *last* occurrence, as long as the span stays within
/// `max_span` bases.
fn find_all_spans(
    hay: &[u8],
    pattern: &[u8],
    max_start: usize,
    max_span: usize,
) -> Vec<(usize, usize)> {
    let parts: Vec<&[u8]> = pattern
        .split(|b| *b == b'#')
        .filter(|p| !p.is_empty())
        .collect();
    let Some((&first, rest)) = parts.split_first() else {
        return Vec::new();
    };
    if rest.is_empty() {
        return find_all(hay, first, max_start)
            .into_iter()
            .map(|s| (s, s + first.len()))
            .collect();
    }
    let mut out = Vec::new();
    'starts: for s in find_all(hay, first, max_start) {
        let mut end = s + first.len();
        let limit = hay.len().min(s + max_span);
        for part in rest {
            let Some(t) = (end..(limit + 1).saturating_sub(part.len()))
                .rev()
                .find(|&i| matches_at(part, &hay[i..i + part.len()]))
            else {
                continue 'starts;
            };
            end = t + part.len();
        }
        out.push((s, end));
    }
    out
}

/// Matches a definition against one strand given as text (`hay`), which
/// may be a doubled circular sequence; only matches starting before
/// `max_start` count.
fn match_strand(def: &FeatureDef, hay: &[u8], max_start: usize) -> Vec<Hit> {
    let five = upper_dna(&def.five);
    let three = upper(&def.three);
    let p5 = upper(&def.potential_five);
    let p3 = upper(&def.potential_three);
    let up = upper(&def.upstream);
    let down = upper(&def.downstream);
    let mut hits = Vec::new();
    for (s, e) in find_all_spans(hay, &five, max_start, max_start) {
        let mut start = s;
        let mut end = e;
        if !three.is_empty() {
            // The nearest 3' consensus after the 5' one.
            let Some(t) = (end..hay.len().saturating_sub(three.len()) + 1)
                .find(|&i| matches_at(&three, &hay[i..i + three.len()]))
            else {
                continue;
            };
            end = t + three.len();
        }
        if !p5.is_empty() && start >= p5.len() && matches_at(&p5, &hay[start - p5.len()..start]) {
            start -= p5.len();
        }
        if !p3.is_empty()
            && end + p3.len() <= hay.len()
            && matches_at(&p3, &hay[end..end + p3.len()])
        {
            end += p3.len();
        }
        if !up.is_empty() && !(start >= up.len() && matches_at(&up, &hay[start - up.len()..start]))
        {
            continue;
        }
        if !down.is_empty()
            && !(end + down.len() <= hay.len() && matches_at(&down, &hay[end..end + down.len()]))
        {
            continue;
        }
        hits.push(Hit { start, end });
    }
    hits
}

/// Peptide matching: exact letters, `X` matches anything.
fn peptide_matches(pattern: &[u8], text: &[u8]) -> bool {
    pattern.len() == text.len()
        && pattern
            .iter()
            .zip(text)
            .all(|(p, t)| *p == b'X' || p.eq_ignore_ascii_case(t))
}

fn find_all_peptide(hay: &[u8], pattern: &[u8]) -> Vec<usize> {
    let len = pattern.len();
    if len == 0 || hay.len() < len {
        return Vec::new();
    }
    (0..=hay.len() - len)
        .filter(|&i| peptide_matches(pattern, &hay[i..i + len]))
        .collect()
}

/// Peptide definitions: translate each frame of `hay` and match; hits are
/// returned in nucleotide coordinates of `hay`.
fn match_peptide(def: &FeatureDef, hay: &[u8], max_start: usize) -> Vec<Hit> {
    let five = upper(&def.five);
    let three = upper(&def.three);
    let p5 = upper(&def.potential_five);
    let p3 = upper(&def.potential_three);
    let up = upper(&def.upstream);
    let down = upper(&def.downstream);
    let mut hits = Vec::new();
    for frame in 0..3 {
        if hay.len() < frame + 3 {
            continue;
        }
        let pep: Vec<u8> = STANDARD.translate(&hay[frame..], true).into_bytes();
        for s in find_all_peptide(&pep, &five) {
            let mut start = s;
            let mut end = s + five.len();
            if !three.is_empty() {
                let Some(t) = (end..pep.len().saturating_sub(three.len()) + 1)
                    .find(|&i| peptide_matches(&three, &pep[i..i + three.len()]))
                else {
                    continue;
                };
                end = t + three.len();
            }
            if !p5.is_empty()
                && start >= p5.len()
                && peptide_matches(&p5, &pep[start - p5.len()..start])
            {
                start -= p5.len();
            }
            if !p3.is_empty()
                && end + p3.len() <= pep.len()
                && peptide_matches(&p3, &pep[end..end + p3.len()])
            {
                end += p3.len();
            }
            if !up.is_empty()
                && !(start >= up.len() && peptide_matches(&up, &pep[start - up.len()..start]))
            {
                continue;
            }
            if !down.is_empty()
                && !(end + down.len() <= pep.len()
                    && peptide_matches(&down, &pep[end..end + down.len()]))
            {
                continue;
            }
            let nt_start = frame + start * 3;
            if nt_start < max_start {
                hits.push(Hit {
                    start: nt_start,
                    end: frame + end * 3,
                });
            }
        }
    }
    hits
}

/// Scans a sequence for one definition. Both strands unless `sense_only`;
/// circular sequences are scanned across the origin (features wrapping it
/// get `start > end`).
pub fn scan_one(seq: &Sequence, def: &FeatureDef) -> Vec<Feature> {
    let n = seq.len();
    if n == 0 || def.five.trim().is_empty() {
        return Vec::new();
    }
    // Protein windows (changelog 2.6: "Scan for peptide Features in Protein
    // Sequences"): peptide definitions match the residues directly; DNA
    // definitions do not apply.
    if seq.seq_type == sc_seq::SeqType::Protein {
        if !def.peptide {
            return Vec::new();
        }
        let hay: Vec<u8> = seq.bases.iter().map(|b| b.to_ascii_uppercase()).collect();
        let five = upper(&def.five);
        let up = upper(&def.upstream);
        let down = upper(&def.downstream);
        let mut out = Vec::new();
        for s in find_all_peptide(&hay, &five) {
            let end = s + five.len();
            if !up.is_empty() && !(s >= up.len() && peptide_matches(&up, &hay[s - up.len()..s])) {
                continue;
            }
            if !down.is_empty()
                && !(end + down.len() <= hay.len()
                    && peptide_matches(&down, &hay[end..end + down.len()]))
            {
                continue;
            }
            out.push(feature_from(def, s as u32 + 1, end as u32, Strand::Forward));
        }
        return out;
    }
    let circular = seq.topology.is_circular();
    let mut forward: Vec<u8> = seq.bases.clone();
    if circular {
        forward.extend_from_slice(&seq.bases);
    }
    let matcher = |hay: &[u8]| {
        if def.peptide {
            match_peptide(def, hay, n)
        } else {
            match_strand(def, hay, n)
        }
    };
    let mut out = Vec::new();
    for h in matcher(&forward) {
        if h.end - h.start > n {
            continue;
        }
        let start = h.start + 1;
        let end = (h.end - 1) % n + 1;
        out.push(feature_from(def, start as u32, end as u32, Strand::Forward));
    }
    if !def.sense_only {
        // On the reverse strand every match of a doubled circle appears
        // twice; keep the one whose lowest forward coordinate lies in the
        // first copy. rc index i ↔ forward index len-1-i.
        let rc = reverse_complement(&forward);
        let len = forward.len();
        let rc_hits = if def.peptide {
            match_peptide(def, &rc, len)
        } else {
            match_strand(def, &rc, len)
        };
        for h in rc_hits {
            if h.end - h.start > n {
                continue;
            }
            let f_start = len - h.end;
            let f_end = len - 1 - h.start;
            if f_start >= n {
                continue;
            }
            let start5 = f_end % n + 1;
            let end3 = f_start + 1;
            out.push(feature_from(
                def,
                start5 as u32,
                end3 as u32,
                Strand::Reverse,
            ));
        }
    }
    out.sort_by_key(|f| (f.lo_hi().0, f.strand == Strand::Reverse));
    out.dedup_by(|a, b| a.start == b.start && a.end == b.end && a.strand == b.strand);
    out
}

/// Scans every enabled definition of every enabled collection.
pub fn scan(seq: &Sequence, collections: &[Collection]) -> Vec<Feature> {
    let mut out = Vec::new();
    for c in collections.iter().filter(|c| c.scan) {
        for d in c.features.iter().filter(|d| d.scan) {
            out.extend(scan_one(seq, d));
        }
    }
    out
}

/// Scans for the features that span any of the `junctions` (0-based: the
/// junction after base `j`, i.e. between bases `j` and `j + 1` of the
/// sequence; `seq.len()` is the closing junction of a circular sequence,
/// between its last base and base 1). This is the original's
/// `SetAnnotations(seq, from, to, false)` after a ligation or a Gateway
/// reaction: only features recreated across a junction are reported, so
/// that a feature already inherited from a fragment is not found twice.
pub fn scan_junctions(
    seq: &Sequence,
    collections: &[Collection],
    junctions: &[usize],
) -> Vec<Feature> {
    let n = seq.len();
    if n == 0 || junctions.is_empty() {
        return Vec::new();
    }
    let spans = |f: &Feature, j: usize| {
        let (lo, hi, wraps) = f.lo_hi();
        let (lo, hi) = (lo as usize, hi as usize);
        if j >= n {
            // The closing junction: only a feature wrapping the origin spans it.
            wraps
        } else if wraps {
            // A wrapping feature covers `hi..=n` and `1..=lo`.
            hi <= j || j < lo
        } else {
            lo <= j && j < hi
        }
    };
    scan(seq, collections)
        .into_iter()
        .filter(|f| junctions.iter().any(|&j| spans(f, j)))
        .collect()
}

/// Identity as the original's `Annot_to_Chain` sees it: a feature whose
/// name, position, strand, type or colour was edited is no longer "the
/// same" as the scan's hit, so a new scan leaves it alone and adds the hit
/// beside it (Features ReadMe §VIII.D: "if you change something on an
/// automatically found annotation (like its name, color, etc.), it will not
/// anymore be automatically erased upon a new scan").
fn same(a: &Feature, b: &Feature) -> bool {
    a.name == b.name
        && a.start == b.start
        && a.end == b.end
        && a.strand == b.strand
        && a.kind == b.kind
        && a.color == b.color
}

/// Applies scan results to a sequence's feature list (Features ReadMe
/// §VIII.D): unprotected features identical to a hit are replaced; hits
/// identical to a protected feature are dropped; everything else is added
/// (an edited feature counts as different, see [`same`]).
/// Returns how many features were added.
pub fn merge(existing: &mut Vec<Feature>, found: Vec<Feature>) -> usize {
    existing.retain(|e| e.protect || !found.iter().any(|f| same(e, f)));
    let mut added = 0;
    for f in found {
        if existing.iter().any(|e| same(e, &f)) {
            continue;
        }
        existing.push(f);
        added += 1;
    }
    added
}

/// Features ▸ Remove duplicated Features: keeps the first of identical ones.
pub fn remove_duplicates(features: &mut Vec<Feature>) -> usize {
    let before = features.len();
    let mut seen: Vec<Feature> = Vec::new();
    features.retain(|f| {
        if seen.iter().any(|s| same(s, f)) {
            false
        } else {
            seen.push(f.clone());
            true
        }
    });
    before - features.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_seq::Topology;

    fn seq(bases: &str) -> Sequence {
        Sequence::new("t", bases.as_bytes().to_vec())
    }

    #[test]
    fn simple_dna_both_strands() {
        let def = FeatureDef::simple("site", "misc_feature", "#000000", "GAATTC");
        let s = seq("ccGAATTCccGGATCCcc");
        let f = scan_one(&s, &def);
        // Palindrome: one hit per strand at the same span; deduplicated by span+strand.
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].start, f[0].end, f[0].strand), (3, 8, Strand::Forward));
        assert_eq!((f[1].start, f[1].end, f[1].strand), (8, 3, Strand::Reverse));
        let asym = FeatureDef::simple("bsa", "misc_feature", "#000000", "GGTCTC");
        let s = seq("ccGGTCTCccGAGACCcc");
        let f = scan_one(&s, &asym);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].start, f[0].end, f[0].strand), (3, 8, Strand::Forward));
        assert_eq!(
            (f[1].start, f[1].end, f[1].strand),
            (16, 11, Strand::Reverse)
        );
        let mut sense = asym.clone();
        sense.sense_only = true;
        assert_eq!(scan_one(&s, &sense).len(), 1);
    }

    #[test]
    fn degenerate_consensus_and_circular_wrap() {
        let def = FeatureDef::simple("kozak", "misc_feature", "#000000", "GCCRCCATGG");
        let s = seq("tttGCCACCATGGttt");
        assert_eq!(scan_one(&s, &def).len(), 1); // not palindromic: forward only
        let mut c = seq("CATGGtttttGCCAC");
        c.topology = Topology::Circular;
        let f = scan_one(&c, &def);
        let fwd = f.iter().find(|f| f.strand == Strand::Forward).unwrap();
        assert_eq!((fwd.start, fwd.end), (11, 5));
        assert_eq!(fwd.lo_hi(), (5, 11, true));
    }

    #[test]
    fn complex_definition_with_potential_and_context() {
        let mut def = FeatureDef::simple("orf", "CDS", "#000000", "GTGAGCAAG");
        def.three = "GAGCTGTAC".into();
        def.potential_five = "ATG".into();
        def.potential_three = "TAA".into();
        def.sense_only = true;
        let with = seq("ccATGGTGAGCAAGxxxxGAGCTGTACTAAcc"
            .replace('x', "A")
            .as_str());
        let f = scan_one(&with, &def);
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].start, f[0].end), (3, 30));
        let without = seq("ccGTGAGCAAGAAAAGAGCTGTACcc");
        let f = scan_one(&without, &def);
        assert_eq!((f[0].start, f[0].end), (3, 24));
        // Upstream context required.
        def.upstream = "TTT".into();
        assert!(scan_one(&with, &def).is_empty());
        assert_eq!(scan_one(&seq("TTTGTGAGCAAGAAAGAGCTGTAC"), &def).len(), 1);
    }

    #[test]
    fn junction_scan_reports_only_features_spanning_a_junction() {
        // A ligation whose junction recreates a short tag consensus.
        let tag = FeatureDef::simple("tag", "misc_feature", "#000000", "GAATTC");
        let c = Collection {
            name: "t".into(),
            scan: true,
            builtin: false,
            features: vec![tag],
        };
        // GAATTC at 3..8 (spans the junction after base 5) and at 12..17
        // (entirely inside the second fragment).
        let s = seq("ccGAATTCaaaGAATTCaa");
        let found = scan_junctions(&s, std::slice::from_ref(&c), &[5]);
        assert_eq!(found.len(), 2, "forward and reverse hits of the palindrome");
        assert!(found.iter().all(|f| f.lo_hi().0 == 3 && f.lo_hi().1 == 8));
        // A junction inside no hit reports nothing; the exact boundary of a
        // hit is not spanned.
        assert!(scan_junctions(&s, std::slice::from_ref(&c), &[9, 10]).is_empty());
        assert!(scan_junctions(&s, std::slice::from_ref(&c), &[8]).is_empty());
        // The closing junction of a circle is spanned only by a wrapping hit.
        let mut circ = seq("TTCaaaaaaaaaaaGAA");
        circ.topology = sc_seq::Topology::Circular;
        let found = scan_junctions(&circ, std::slice::from_ref(&c), &[circ.len()]);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|f| f.lo_hi().2));
        assert!(scan_junctions(&circ, std::slice::from_ref(&c), &[5]).is_empty());
    }

    #[test]
    fn peptide_definition_in_any_frame_and_strand() {
        let mut def = FeatureDef::simple("HA", "misc_feature", "#000000", "YPYDVPDYA");
        def.peptide = true;
        // YPYDVPDYA = TAC CCA TAC GAC GTC CCA GAC TAC GCT
        let dna = "gTACCCATACGACGTCCCAGACTACGCTgg";
        let f = scan_one(&seq(dna), &def);
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].start, f[0].end, f[0].strand),
            (2, 28, Strand::Forward)
        );
        let rc = String::from_utf8(reverse_complement(dna.as_bytes())).unwrap();
        let f = scan_one(&seq(&rc), &def);
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].strand, f[0].lo_hi()),
            (Strand::Reverse, (3, 29, false))
        );
        // Downstream peptide context ("Cys" of CVLL).
        let mut cys = FeatureDef::simple("Cys", "misc_feature", "#000000", "C");
        cys.peptide = true;
        cys.downstream = "VLL".into();
        cys.sense_only = true;
        let f = scan_one(&seq("TGTGTGCTGCTGTGTAAA"), &cys); // C V L L C K
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].start, f[0].end), (1, 3));
    }

    #[test]
    fn protein_windows_match_peptide_definitions_directly() {
        let mut ha = FeatureDef::simple("HA", "misc_feature", "#000000", "YPYDVPDYA");
        ha.peptide = true;
        let dna_def = FeatureDef::simple("site", "misc_feature", "#000000", "GAATTC");
        let mut prot = seq("MGYPYDVPDYAGAATTC");
        prot.seq_type = sc_seq::SeqType::Protein;
        let f = scan_one(&prot, &ha);
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].start, f[0].end), (3, 11));
        assert!(scan_one(&prot, &dna_def).is_empty());
    }

    #[test]
    fn merge_keeps_protected_and_replaces_unprotected() {
        let def = FeatureDef::simple("site", "misc_feature", "#000000", "GAATTC");
        let s = seq("ccGAATTCcc");
        let found = scan_one(&s, &def);
        // An untouched hit from a previous scan is replaced by the new one.
        let mut existing = vec![found[0].clone()];
        let added = merge(&mut existing, found.clone());
        assert_eq!(added, 2);
        assert_eq!(existing.len(), 2);
        // A recoloured one is no longer identical: it stays and the hit is
        // added beside it (the original's documented duplication).
        let mut existing = vec![found[0].clone()];
        existing[0].color = sc_seq::Rgb(1, 1, 1);
        let added = merge(&mut existing, found.clone());
        assert_eq!(added, 2);
        assert_eq!(existing.len(), 3);
        assert_eq!(existing[0].color, sc_seq::Rgb(1, 1, 1));
        let mut existing = vec![found[0].clone()];
        let added = merge(&mut existing, found.clone());
        assert_eq!(added, 2);
        assert_eq!(existing[0].color, def.rgb());
        // Protected: kept, and the identical hit is not duplicated.
        let mut protected = vec![found[0].clone()];
        protected[0].protect = true;
        protected[0].name = "site".into();
        let added = merge(&mut protected, found);
        assert_eq!(added, 1);
        assert_eq!(protected.len(), 2);
        assert!(protected[0].protect);
        assert_eq!(remove_duplicates(&mut protected), 0);
        protected.push(protected[1].clone());
        assert_eq!(remove_duplicates(&mut protected), 1);
    }
}
