//! The `DocumentStore`: every open sequence lives here, keyed by id. Windows
//! hold ids, never copies, so a Sequence Window and its drawers can never
//! diverge (`PLAN.md` §3).

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use sc_seq::alphabet::{complement_all, is_acgt, is_iupac, reverse_complement};
use sc_seq::{Ends, Feature, Overhang, SeqType, Sequence, Source, Strand, Topology};
use serde::{Deserialize, Serialize};

/// Document identifier; also the number in the window label `seq-{id}`.
pub type DocId = u32;

/// Undo depth, as in the original ("5 levels of undo are available").
pub const UNDO_LEVELS: usize = 5;

/// The undoable part of a document.
#[derive(Clone)]
struct Snapshot {
    bases: Vec<u8>,
    features: Vec<Feature>,
    ends: Ends,
    comment: String,
    topology: Topology,
    seq_type: SeqType,
}

/// An open sequence.
pub struct Document {
    /// The sequence.
    pub seq: Sequence,
    /// Where it was opened from / last saved to.
    pub path: Option<PathBuf>,
    /// Modified since open / save.
    pub dirty: bool,
    /// The Lock checkbox: edits are refused.
    pub locked: bool,
    undo: VecDeque<Snapshot>,
    redo: VecDeque<Snapshot>,
}

/// What a window receives: the whole document, ready to render.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentView {
    /// Id.
    pub id: DocId,
    /// Name.
    pub name: String,
    /// Bases as text, case preserved.
    pub bases: String,
    /// `DNA`, `Degenerate DNA`, `RNA`, `Protein`, `Unknown`.
    pub seq_type: String,
    /// True for degenerate windows (IUPAC letters accepted).
    pub degenerate: bool,
    /// True for circular.
    pub circular: bool,
    /// Comment, verbatim.
    pub comment: String,
    /// Features in z-order.
    pub features: Vec<FeatureView>,
    /// Ends.
    pub ends: EndsView,
    /// Format label for the Sequence Window.
    pub source: String,
    /// Lock state.
    pub locked: bool,
    /// Dirty state.
    pub dirty: bool,
    /// File path, if any.
    pub path: Option<String>,
    /// Undo available.
    pub can_undo: bool,
    /// Redo available.
    pub can_redo: bool,
}

/// A feature, flattened for the webview (and accepted back from it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureView {
    /// Name.
    pub name: String,
    /// Description.
    pub description: String,
    /// GenBank kind.
    pub kind: String,
    /// First base in feature direction, 1-based.
    pub start: u32,
    /// Last base in feature direction, 1-based.
    pub end: u32,
    /// True for forward.
    pub forward: bool,
    /// Shown on maps.
    pub show: bool,
    /// Protected from re-scan.
    pub protect: bool,
    /// Drawn with an arrow.
    pub arrow: bool,
    /// `#rrggbb`.
    pub color: String,
}

/// Both ends, flattened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndsView {
    /// Signed left overhang length.
    pub left_length: i32,
    /// Left overhang bases.
    pub left_bases: String,
    /// Signed right overhang length.
    pub right_length: i32,
    /// Right overhang bases.
    pub right_bases: String,
}

impl FeatureView {
    /// Back to the model; a bad colour becomes grey.
    pub fn into_feature(self) -> Feature {
        Feature {
            name: self.name,
            description: self.description,
            kind: if self.kind.trim().is_empty() {
                "misc_feature".into()
            } else {
                self.kind
            },
            start: self.start.max(1),
            end: self.end.max(1),
            strand: if self.forward {
                Strand::Forward
            } else {
                Strand::Reverse
            },
            show: self.show,
            protect: self.protect,
            arrow: self.arrow,
            color: sc_features::parse_color(&self.color).unwrap_or(sc_seq::Rgb(132, 164, 192)),
        }
    }
}

impl From<&Feature> for FeatureView {
    fn from(f: &Feature) -> Self {
        FeatureView {
            name: f.name.clone(),
            description: f.description.clone(),
            kind: f.kind.clone(),
            start: f.start,
            end: f.end,
            forward: f.strand == Strand::Forward,
            show: f.show,
            protect: f.protect,
            arrow: f.arrow,
            color: format!("#{:02x}{:02x}{:02x}", f.color.0, f.color.1, f.color.2),
        }
    }
}

/// An edit requested by a window. Positions are 0-based, `from..to` half-open.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOp {
    /// Replace `from..to` with `text` (typing, deleting, pasting).
    Replace {
        /// Start.
        from: usize,
        /// End (exclusive).
        to: usize,
        /// Replacement; letters outside the window's alphabet are filtered.
        text: String,
    },
    /// Sequence ▸ UPPERCASE.
    Uppercase {
        /// Start.
        from: usize,
        /// End.
        to: usize,
    },
    /// Sequence ▸ Lowercase.
    Lowercase {
        /// Start.
        from: usize,
        /// End.
        to: usize,
    },
    /// Sequence ▸ Antiparallel (reverse complement).
    Antiparallel {
        /// Start.
        from: usize,
        /// End.
        to: usize,
    },
    /// Sequence ▸ Reverse.
    Reverse {
        /// Start.
        from: usize,
        /// End.
        to: usize,
    },
    /// Sequence ▸ Complement.
    Complement {
        /// Start.
        from: usize,
        /// End.
        to: usize,
    },
    /// Sequence ▸ Linearize / Circularize.
    SetTopology {
        /// New topology.
        circular: bool,
    },
    /// Sequence ▸ Set Origin (1-based position becomes base 1).
    SetOrigin {
        /// Position.
        position: usize,
    },
    /// Comments tab.
    SetComment {
        /// New comment.
        text: String,
    },
    /// Rename (context menu on the name).
    Rename {
        /// New name.
        name: String,
    },
    /// Lock checkbox.
    SetLocked {
        /// New state.
        locked: bool,
    },
    /// Extremities tab.
    SetEnds {
        /// New ends.
        ends: EndsView,
    },
    /// Transform DNA ↔ degenerate DNA.
    SetDegenerate {
        /// New state.
        degenerate: bool,
    },
    /// Features tab: Show checkbox (index into the feature list).
    SetFeatureShow {
        /// Feature index.
        index: usize,
        /// New state.
        show: bool,
    },
    /// Features tab `+`: appends a feature (marked protected: user-made).
    AddFeature {
        /// The feature.
        feature: FeatureView,
    },
    /// Features tab editor: replaces a feature (marks it protected).
    UpdateFeature {
        /// Index.
        index: usize,
        /// New values.
        feature: FeatureView,
    },
    /// Features tab `−`.
    DeleteFeature {
        /// Index.
        index: usize,
    },
    /// Drag to reorder (list order = drawing order).
    MoveFeature {
        /// From index.
        from: usize,
        /// To index.
        to: usize,
    },
    /// Features ▸ Remove duplicated Features.
    RemoveDuplicates,
    /// Features ▸ Scan Sequence, with the hits computed by the shell.
    MergeScan {
        /// Found features.
        found: Vec<FeatureView>,
    },
}

/// What [`Document::apply`] reports back.
#[derive(Debug, Clone, Serialize, Default)]
pub struct EditOutcome {
    /// Letters dropped from a `Replace` because they are not in the alphabet.
    pub filtered: usize,
}

impl Document {
    /// Wraps a sequence.
    pub fn new(seq: Sequence, path: Option<PathBuf>) -> Self {
        Document {
            seq,
            path,
            dirty: false,
            locked: false,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            bases: self.seq.bases.clone(),
            features: self.seq.features.clone(),
            ends: self.seq.ends.clone(),
            comment: self.seq.comment.clone(),
            topology: self.seq.topology,
            seq_type: self.seq.seq_type,
        }
    }

    fn restore(&mut self, s: Snapshot) {
        self.seq.bases = s.bases;
        self.seq.features = s.features;
        self.seq.ends = s.ends;
        self.seq.comment = s.comment;
        self.seq.topology = s.topology;
        self.seq.seq_type = s.seq_type;
    }

    fn push_undo(&mut self) {
        self.undo.push_back(self.snapshot());
        while self.undo.len() > UNDO_LEVELS {
            self.undo.pop_front();
        }
        self.redo.clear();
        self.dirty = true;
    }

    /// Edit ▸ Undo.
    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push_back(self.snapshot());
        self.restore(s);
        self.dirty = true;
        true
    }

    /// Edit ▸ Redo.
    pub fn redo(&mut self) -> bool {
        let Some(s) = self.redo.pop_back() else {
            return false;
        };
        self.undo.push_back(self.snapshot());
        self.restore(s);
        self.dirty = true;
        true
    }

    /// Letters this document accepts.
    fn accepts(&self, b: u8) -> bool {
        match self.seq.seq_type {
            SeqType::Protein => b.is_ascii_alphabetic() || b == b'*',
            SeqType::DegenerateDna | SeqType::Rna | SeqType::Unknown => is_iupac(b),
            SeqType::Dna => is_acgt(b),
        }
    }

    /// Applies an edit. Sequence-changing edits are refused on a locked
    /// document.
    pub fn apply(&mut self, op: EditOp) -> Result<EditOutcome, String> {
        let n = self.seq.bases.len();
        let clamp = |from: usize, to: usize| (from.min(n), to.min(n).max(from.min(n)));
        let changes_sequence = !matches!(
            op,
            EditOp::SetLocked { .. }
                | EditOp::Rename { .. }
                | EditOp::SetComment { .. }
                | EditOp::SetFeatureShow { .. }
        );
        if self.locked && changes_sequence {
            return Err("The sequence is locked.".into());
        }
        let mut outcome = EditOutcome::default();
        match op {
            EditOp::Replace { from, to, text } => {
                let (from, to) = clamp(from, to);
                let mut kept = Vec::with_capacity(text.len());
                for b in text.bytes() {
                    if b.is_ascii_whitespace() || b.is_ascii_digit() {
                        continue;
                    }
                    if self.accepts(b) {
                        kept.push(b);
                    } else {
                        outcome.filtered += 1;
                    }
                }
                if kept.is_empty() && from == to {
                    return Ok(outcome);
                }
                self.push_undo();
                let delta = kept.len() as i64 - (to - from) as i64;
                self.seq.bases.splice(from..to, kept);
                shift_features(
                    &mut self.seq.features,
                    from,
                    to,
                    delta,
                    self.seq.bases.len(),
                );
            }
            EditOp::Uppercase { from, to } => {
                let (from, to) = clamp(from, to);
                self.push_undo();
                self.seq.bases[from..to].make_ascii_uppercase();
            }
            EditOp::Lowercase { from, to } => {
                let (from, to) = clamp(from, to);
                self.push_undo();
                self.seq.bases[from..to].make_ascii_lowercase();
            }
            EditOp::Antiparallel { from, to } => {
                let (from, to) = clamp(from, to);
                self.push_undo();
                let rc = reverse_complement(&self.seq.bases[from..to]);
                self.seq.bases[from..to].copy_from_slice(&rc);
                if from == 0 && to == n {
                    flip_features(&mut self.seq.features, n as u32);
                }
            }
            EditOp::Reverse { from, to } => {
                let (from, to) = clamp(from, to);
                self.push_undo();
                self.seq.bases[from..to].reverse();
            }
            EditOp::Complement { from, to } => {
                let (from, to) = clamp(from, to);
                self.push_undo();
                let c = complement_all(&self.seq.bases[from..to]);
                self.seq.bases[from..to].copy_from_slice(&c);
            }
            EditOp::SetTopology { circular } => {
                self.push_undo();
                self.seq.topology = if circular {
                    Topology::Circular
                } else {
                    Topology::Linear
                };
            }
            EditOp::SetOrigin { position } => {
                self.push_undo();
                self.seq.set_origin(position as u32);
            }
            EditOp::SetComment { text } => {
                self.push_undo();
                self.seq.comment = text;
            }
            EditOp::Rename { name } => {
                let name = name.trim();
                if !name.is_empty() {
                    self.seq.name = name.to_owned();
                    self.dirty = true;
                }
            }
            EditOp::SetLocked { locked } => self.locked = locked,
            EditOp::SetEnds { ends } => {
                self.push_undo();
                self.seq.ends = Ends {
                    left: overhang(ends.left_length, &ends.left_bases),
                    right: overhang(ends.right_length, &ends.right_bases),
                };
            }
            EditOp::SetDegenerate { degenerate } => {
                self.push_undo();
                self.seq.seq_type = if degenerate {
                    SeqType::DegenerateDna
                } else {
                    SeqType::Dna
                };
                if !degenerate {
                    let before = self.seq.bases.len();
                    self.seq.bases.retain(|&b| is_acgt(b));
                    outcome.filtered = before - self.seq.bases.len();
                }
            }
            EditOp::SetFeatureShow { index, show } => {
                if let Some(f) = self.seq.features.get_mut(index) {
                    f.show = show;
                    self.dirty = true;
                }
            }
            EditOp::AddFeature { feature } => {
                self.push_undo();
                let mut f = feature.into_feature();
                f.protect = true;
                self.seq.features.push(f);
            }
            EditOp::UpdateFeature { index, feature } => {
                if index >= self.seq.features.len() {
                    return Err("no such feature".into());
                }
                self.push_undo();
                let mut f = feature.into_feature();
                f.protect = true;
                self.seq.features[index] = f;
            }
            EditOp::DeleteFeature { index } => {
                if index >= self.seq.features.len() {
                    return Err("no such feature".into());
                }
                self.push_undo();
                self.seq.features.remove(index);
            }
            EditOp::MoveFeature { from, to } => {
                let n = self.seq.features.len();
                if from >= n || to >= n {
                    return Err("no such feature".into());
                }
                if from != to {
                    self.push_undo();
                    let f = self.seq.features.remove(from);
                    self.seq.features.insert(to, f);
                }
            }
            EditOp::RemoveDuplicates => {
                self.push_undo();
                outcome.filtered = sc_features::scan::remove_duplicates(&mut self.seq.features);
            }
            EditOp::MergeScan { found } => {
                self.push_undo();
                let found: Vec<Feature> =
                    found.into_iter().map(FeatureView::into_feature).collect();
                outcome.filtered = sc_features::merge(&mut self.seq.features, found);
            }
        }
        Ok(outcome)
    }

    /// The view sent to windows.
    pub fn view(&self, id: DocId) -> DocumentView {
        let s = &self.seq;
        DocumentView {
            id,
            name: s.name.clone(),
            bases: String::from_utf8_lossy(&s.bases).into_owned(),
            seq_type: s.seq_type.label().to_owned(),
            degenerate: s.seq_type != SeqType::Dna,
            circular: s.topology.is_circular(),
            comment: s.comment.clone(),
            features: s.features.iter().map(FeatureView::from).collect(),
            ends: EndsView {
                left_length: s.ends.left.length,
                left_bases: String::from_utf8_lossy(&s.ends.left.bases).into_owned(),
                right_length: s.ends.right.length,
                right_bases: String::from_utf8_lossy(&s.ends.right.bases).into_owned(),
            },
            source: s.source.label().to_owned(),
            locked: self.locked,
            dirty: self.dirty,
            path: self.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
        }
    }
}

fn overhang(length: i32, bases: &str) -> Overhang {
    let mut bases: Vec<u8> = bases.bytes().filter(|&b| is_iupac(b)).collect();
    let n = length.unsigned_abs() as usize;
    bases.resize(n, b'N');
    Overhang { length, bases }
}

/// Keeps features attached to their bases after `from..to` was replaced by
/// `to - from + delta` bases: features after the edit shift, features
/// spanning it stretch or shrink, features entirely inside a deletion are
/// dropped. (M4 refines this; enough not to lose annotations while typing.)
fn shift_features(features: &mut Vec<Feature>, from: usize, to: usize, delta: i64, new_len: usize) {
    let (from, to) = (from as i64 + 1, to as i64); // 1-based inclusive edit span
    let shift = |p: u32| -> Option<u32> {
        let p = p as i64;
        if p < from {
            Some(p as u32)
        } else if p > to {
            Some((p + delta) as u32)
        } else {
            // inside the replaced span
            let np = (p + delta).max(from - 1).min(to + delta);
            if np < 1 { None } else { Some(np as u32) }
        }
    };
    features.retain_mut(|f| {
        let (lo, hi, wraps) = f.lo_hi();
        if !wraps && lo as i64 >= from && hi as i64 <= to && delta <= -((hi - lo + 1) as i64) {
            return false; // deleted with its bases
        }
        match (shift(f.start), shift(f.end)) {
            (Some(s), Some(e))
                if s >= 1 && e >= 1 && s as usize <= new_len && e as usize <= new_len =>
            {
                f.start = s;
                f.end = e;
                true
            }
            _ => false,
        }
    });
}

/// After antiparalleling the whole sequence every feature swaps strand and
/// coordinate.
fn flip_features(features: &mut [Feature], n: u32) {
    for f in features {
        let (s, e) = (n + 1 - f.start, n + 1 - f.end);
        f.start = s;
        f.end = e;
        f.strand = match f.strand {
            Strand::Forward => Strand::Reverse,
            Strand::Reverse => Strand::Forward,
        };
    }
}

/// All open documents.
#[derive(Default)]
pub struct DocumentStore {
    docs: HashMap<DocId, Document>,
    next_id: DocId,
}

impl DocumentStore {
    /// Adds a document and returns its id.
    pub fn insert(&mut self, doc: Document) -> DocId {
        self.next_id += 1;
        let id = self.next_id;
        self.docs.insert(id, doc);
        id
    }

    /// Looks a document up.
    pub fn get(&self, id: DocId) -> Option<&Document> {
        self.docs.get(&id)
    }

    /// Looks a document up, mutably.
    pub fn get_mut(&mut self, id: DocId) -> Option<&mut Document> {
        self.docs.get_mut(&id)
    }

    /// Removes a document.
    pub fn remove(&mut self, id: DocId) -> Option<Document> {
        self.docs.remove(&id)
    }

    /// Ids and names, in id order.
    pub fn list(&self) -> Vec<(DocId, String)> {
        let mut v: Vec<_> = self
            .docs
            .iter()
            .map(|(id, d)| (*id, d.seq.name.clone()))
            .collect();
        v.sort();
        v
    }

    /// Ids of documents with unsaved changes.
    pub fn dirty_ids(&self) -> Vec<DocId> {
        let mut v: Vec<_> = self
            .docs
            .iter()
            .filter(|(_, d)| d.dirty)
            .map(|(id, _)| *id)
            .collect();
        v.sort();
        v
    }

    /// Number of open documents.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Iterates over documents.
    pub fn iter(&self) -> impl Iterator<Item = (DocId, &Document)> {
        self.docs.iter().map(|(id, d)| (*id, d))
    }

    /// A fresh, empty, unsaved sequence.
    pub fn new_blank(&mut self, degenerate: bool) -> DocId {
        let n = self.docs.len() + 1;
        let mut seq = Sequence::new(format!("Untitled {n}"), Vec::new());
        seq.seq_type = if degenerate {
            SeqType::DegenerateDna
        } else {
            SeqType::Dna
        };
        seq.source = Source::New;
        let mut doc = Document::new(seq, None);
        doc.dirty = false;
        self.insert(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_seq::Rgb;

    fn doc(bases: &str) -> Document {
        Document::new(Sequence::new("t", bases.as_bytes().to_vec()), None)
    }

    #[test]
    fn replace_filters_and_is_undoable() {
        let mut d = doc("ACGT");
        let out = d
            .apply(EditOp::Replace {
                from: 2,
                to: 2,
                text: "gg x1 N".into(),
            })
            .unwrap();
        assert_eq!(out.filtered, 2); // x and N are not ACGT
        assert_eq!(d.seq.bases, b"ACggGT");
        assert!(d.dirty);
        assert!(d.undo());
        assert_eq!(d.seq.bases, b"ACGT");
        assert!(d.redo());
        assert_eq!(d.seq.bases, b"ACggGT");
    }

    #[test]
    fn degenerate_documents_accept_iupac() {
        let mut d = doc("ACGT");
        d.seq.seq_type = SeqType::DegenerateDna;
        d.apply(EditOp::Replace {
            from: 4,
            to: 4,
            text: "NRY".into(),
        })
        .unwrap();
        assert_eq!(d.seq.bases, b"ACGTNRY");
    }

    #[test]
    fn locked_documents_refuse_sequence_edits_but_not_comments() {
        let mut d = doc("ACGT");
        d.apply(EditOp::SetLocked { locked: true }).unwrap();
        assert!(
            d.apply(EditOp::Replace {
                from: 0,
                to: 1,
                text: "".into()
            })
            .is_err()
        );
        d.apply(EditOp::SetComment { text: "c".into() }).unwrap();
        assert_eq!(d.seq.comment, "c");
    }

    #[test]
    fn five_undo_levels() {
        let mut d = doc("");
        for i in 0..8 {
            d.apply(EditOp::Replace {
                from: i,
                to: i,
                text: "A".into(),
            })
            .unwrap();
        }
        let mut n = 0;
        while d.undo() {
            n += 1;
        }
        assert_eq!(n, UNDO_LEVELS);
        assert_eq!(d.seq.bases.len(), 8 - UNDO_LEVELS);
    }

    #[test]
    fn features_follow_edits() {
        let mut d = doc("AAAAACCCCCGGGGG");
        d.seq.features.push(Feature {
            name: "c".into(),
            description: String::new(),
            kind: "misc_feature".into(),
            start: 6,
            end: 10,
            strand: Strand::Forward,
            show: true,
            protect: false,
            arrow: true,
            color: Rgb(0, 0, 0),
        });
        // Insert two bases before the feature.
        d.apply(EditOp::Replace {
            from: 2,
            to: 2,
            text: "TT".into(),
        })
        .unwrap();
        assert_eq!((d.seq.features[0].start, d.seq.features[0].end), (8, 12));
        // Delete the feature's bases entirely.
        d.apply(EditOp::Replace {
            from: 7,
            to: 12,
            text: "".into(),
        })
        .unwrap();
        assert!(d.seq.features.is_empty());
    }

    #[test]
    fn feature_edits_are_undoable_and_mark_protected() {
        let mut d = doc("ACGTACGT");
        let fv = FeatureView {
            name: "f".into(),
            description: String::new(),
            kind: String::new(),
            start: 2,
            end: 5,
            forward: true,
            show: true,
            protect: false,
            arrow: true,
            color: "#ff0000".into(),
        };
        d.apply(EditOp::AddFeature {
            feature: fv.clone(),
        })
        .unwrap();
        assert_eq!(d.seq.features[0].kind, "misc_feature");
        assert!(d.seq.features[0].protect);
        assert_eq!(d.seq.features[0].color, Rgb(255, 0, 0));
        d.apply(EditOp::AddFeature {
            feature: FeatureView {
                name: "g".into(),
                ..fv.clone()
            },
        })
        .unwrap();
        d.apply(EditOp::MoveFeature { from: 1, to: 0 }).unwrap();
        assert_eq!(d.seq.features[0].name, "g");
        d.apply(EditOp::DeleteFeature { index: 0 }).unwrap();
        assert_eq!(d.seq.features.len(), 1);
        assert!(d.undo());
        assert_eq!(d.seq.features.len(), 2);
        assert!(d.apply(EditOp::DeleteFeature { index: 9 }).is_err());
    }

    #[test]
    fn antiparallel_of_the_whole_sequence_flips_features() {
        let mut d = doc("AAAAACCCCC");
        d.seq.features.push(Feature {
            name: "c".into(),
            description: String::new(),
            kind: "misc_feature".into(),
            start: 6,
            end: 10,
            strand: Strand::Forward,
            show: true,
            protect: false,
            arrow: true,
            color: Rgb(0, 0, 0),
        });
        d.apply(EditOp::Antiparallel { from: 0, to: 10 }).unwrap();
        assert_eq!(d.seq.bases, b"GGGGGTTTTT");
        assert_eq!(
            (
                d.seq.features[0].start,
                d.seq.features[0].end,
                d.seq.features[0].strand
            ),
            (5, 1, Strand::Reverse)
        );
        assert_eq!(d.seq.features[0].lo_hi(), (1, 5, false));
    }
}
