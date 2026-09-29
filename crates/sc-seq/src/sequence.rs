//! The document model: a sequence with its comment, features and ends.

use serde::{Deserialize, Serialize};

/// The kind of molecule a sequence describes, as encoded by DNA Strider and
/// shown in the Sequence Window "Type" field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SeqType {
    /// Type code 0.
    Unknown,
    /// Type code 1: plain DNA (`ACGT` only).
    #[default]
    Dna,
    /// Type code 2: degenerate DNA (full IUPAC alphabet).
    DegenerateDna,
    /// Type code 3.
    Rna,
    /// Type code 4.
    Protein,
}

impl SeqType {
    /// Decodes the header byte at offset 1 of an `.xdna` file.
    pub fn from_xdna_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Unknown),
            1 => Some(Self::Dna),
            2 => Some(Self::DegenerateDna),
            3 => Some(Self::Rna),
            4 => Some(Self::Protein),
            _ => None,
        }
    }

    /// Encodes the header byte at offset 1 of an `.xdna` file.
    pub fn to_xdna_byte(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Dna => 1,
            Self::DegenerateDna => 2,
            Self::Rna => 3,
            Self::Protein => 4,
        }
    }

    /// The label shown in the Sequence Window.
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Dna => "DNA",
            Self::DegenerateDna => "Degenerate DNA",
            Self::Rna => "RNA",
            Self::Protein => "Protein",
        }
    }
}

/// Whether the ends of a sequence are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Topology {
    /// Topology code 0.
    #[default]
    Linear,
    /// Topology code 1.
    Circular,
}

impl Topology {
    /// Decodes the header byte at offset 2 of an `.xdna` file.
    pub fn from_xdna_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Linear),
            1 => Some(Self::Circular),
            _ => None,
        }
    }

    /// Encodes the header byte at offset 2 of an `.xdna` file.
    pub fn to_xdna_byte(self) -> u8 {
        match self {
            Self::Linear => 0,
            Self::Circular => 1,
        }
    }

    /// The other topology — what the Sequence Window's Linearize / Circularize
    /// context-menu entry switches to.
    pub fn toggled(self) -> Self {
        match self {
            Self::Linear => Self::Circular,
            Self::Circular => Self::Linear,
        }
    }

    /// True for [`Topology::Circular`].
    pub fn is_circular(self) -> bool {
        matches!(self, Self::Circular)
    }
}

/// Which strand a feature or site is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Strand {
    /// The top strand, 5'→3' left to right. "CW" on a circular map.
    Forward,
    /// The bottom strand. "CCW" on a circular map.
    Reverse,
}

/// An RGB colour, as stored in `.xdna` (`"70,130,180,"`) and GenBank
/// (`&h4682B4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// The colour Serial Cloner gives a feature of this kind when it is not
    /// stated by the file ("Colorize annotation when importing from
    /// non-serial cloner format", changelog 2.5). Values are the ones found in
    /// the shipped `.xdna` and GenBank samples.
    pub fn default_for_kind(kind: &str) -> Rgb {
        match kind {
            "CDS" | "gene" | "mat_peptide" => Rgb(143, 188, 143),
            "rep_origin" => Rgb(70, 130, 180),
            "primer_bind" | "primer" => Rgb(0, 139, 139),
            "misc_binding" | "protein_bind" => Rgb(100, 149, 237),
            "promoter" | "enhancer" | "RBS" => Rgb(255, 127, 0),
            "terminator" | "polyA_signal" | "polyA_site" => Rgb(178, 34, 34),
            _ => Rgb(132, 164, 192),
        }
    }
}

/// A feature / annotation attached to a sequence (Features ReadMe §I).
///
/// `start` and `end` are 1-based, inclusive, and given **in the feature's own
/// direction**: for a forward feature `start ≤ end` unless it wraps the origin
/// of a circular sequence; for a reverse feature `start` is its 5' end on the
/// bottom strand, so normally `start > end`. This is exactly how `.xdna` and
/// Serial Cloner's text exports ("`ColE1 origin : [1824 : 1196 - CCW]`") state
/// coordinates, and it makes both formats round-trip without reinterpretation.
/// Use [`Feature::lo_hi`] for the plain sequence span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    /// Display name (`/label` in GenBank).
    pub name: String,
    /// Free text; multiple qualifiers are separated by `\r` as in `.xdna`.
    pub description: String,
    /// GenBank feature key: `CDS`, `rep_origin`, `primer_bind`, `misc_feature`…
    pub kind: String,
    /// 1-based first base in feature direction.
    pub start: u32,
    /// 1-based last base in feature direction.
    pub end: u32,
    /// Strand.
    pub strand: Strand,
    /// Drawn on maps ("Show" checkbox).
    pub show: bool,
    /// User-edited: a re-scan must not replace it (Features ReadMe §VIII.D).
    pub protect: bool,
    /// Drawn with a pointing arrow.
    pub arrow: bool,
    /// Colour.
    pub color: Rgb,
}

impl Feature {
    /// The lowest and highest 1-based coordinates covered, plus whether the
    /// feature wraps the origin (in which case it covers `lo..=len` and
    /// `1..=hi`).
    pub fn lo_hi(&self) -> (u32, u32, bool) {
        let (a, b) = match self.strand {
            Strand::Forward => (self.start, self.end),
            Strand::Reverse => (self.end, self.start),
        };
        if a <= b { (a, b, false) } else { (b, a, true) }
    }

    /// Length in bases for a sequence of `len` bases (handles origin wrap).
    pub fn len(&self, len: u32) -> u32 {
        let (lo, hi, wraps) = self.lo_hi();
        if wraps {
            len - hi + lo + 1
        } else {
            hi - lo + 1
        }
    }
}

/// One cohesive end of a linear fragment (Sequence Window ▸ Extremities).
///
/// `length` is signed as `.xdna` stores it: positive = 5' overhang, negative =
/// 3' overhang, zero = blunt. `bases` is the single-stranded sequence, read
/// 5'→3' on the strand that carries it, and has `length.unsigned_abs()`
/// bases.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Overhang {
    /// Signed overhang length.
    pub length: i32,
    /// The protruding bases.
    pub bases: Vec<u8>,
}

impl Overhang {
    /// A blunt end.
    pub const BLUNT: Overhang = Overhang {
        length: 0,
        bases: Vec::new(),
    };

    /// Builds a 5' overhang.
    pub fn five_prime(bases: &[u8]) -> Self {
        Overhang {
            length: bases.len() as i32,
            bases: bases.to_vec(),
        }
    }

    /// Builds a 3' overhang.
    pub fn three_prime(bases: &[u8]) -> Self {
        Overhang {
            length: -(bases.len() as i32),
            bases: bases.to_vec(),
        }
    }

    /// True for a blunt end.
    pub fn is_blunt(&self) -> bool {
        self.length == 0
    }

    /// True for a 5' overhang.
    pub fn is_five_prime(&self) -> bool {
        self.length > 0
    }
}

/// Both ends of a linear sequence. Ignored for circular ones.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ends {
    /// The 5' (left) end.
    pub left: Overhang,
    /// The 3' (right) end.
    pub right: Overhang,
}

impl Ends {
    /// Both ends blunt.
    pub fn is_blunt(&self) -> bool {
        self.left.is_blunt() && self.right.is_blunt()
    }
}

/// Where a sequence came from — the Sequence Window "Format" field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Source {
    /// Created in the application, never saved.
    #[default]
    New,
    /// A plain DNA Strider `.xdna` (no Serial Cloner annotation block).
    Strider,
    /// A Serial Cloner `.xdna` (annotation block present).
    SerialCloner,
    /// FASTA.
    Fasta,
    /// GenBank / EMBL / DDBJ.
    GenBank,
    /// SnapGene `.dna`.
    SnapGene,
    /// Raw text.
    RawText,
}

impl Source {
    /// The label shown in the Sequence Window.
    pub fn label(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Strider => "DNA Strider",
            Self::SerialCloner => "Serial Cloner",
            Self::Fasta => "FASTA",
            Self::GenBank => "GenBank",
            Self::SnapGene => "SnapGene",
            Self::RawText => "Text",
        }
    }
}

/// Size of the `.xdna` header, kept verbatim in [`Sequence::strider_header`].
pub const XDNA_HEADER_LEN: usize = 112;

/// A sequence document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Sequence {
    /// Display name. `.xdna` has no name field, so it is the file stem.
    pub name: String,
    /// The bases, ASCII, case preserved.
    pub bases: Vec<u8>,
    /// Molecule type.
    pub seq_type: SeqType,
    /// Topology.
    pub topology: Topology,
    /// The comment field, stored verbatim (`.xdna` separates lines with `\r`).
    pub comment: String,
    /// Features in drawing order (list order = z-order).
    pub features: Vec<Feature>,
    /// Cohesive ends (linear sequences only).
    pub ends: Ends,
    /// Format the sequence was read from.
    pub source: Source,
    /// The 112-byte `.xdna` header the sequence was read from, if any.
    ///
    /// DNA Strider stores fields we do not interpret (timestamps, window
    /// geometry, selection). They are preserved byte for byte so that a file
    /// opened and saved again is unchanged apart from what the user edited.
    pub strider_header: Option<Vec<u8>>,
    /// Every packet of the SnapGene `.dna` file the sequence was read from,
    /// in file order: `(packet type, payload)`.
    ///
    /// SnapGene stores primers, notes, cloning history, sequencing traces and
    /// enzyme caches we do not model. Keeping the packets lets
    /// `sc_format::snapgene::write` give back the original file byte for
    /// byte when nothing was edited, and keep everything else when the
    /// sequence, features or comment were.
    #[serde(default)]
    pub snapgene_extras: Option<Vec<(u8, Vec<u8>)>>,
}

impl Sequence {
    /// A new DNA sequence with the given name and bases.
    pub fn new(name: impl Into<String>, bases: impl Into<Vec<u8>>) -> Self {
        Sequence {
            name: name.into(),
            bases: bases.into(),
            ..Default::default()
        }
    }

    /// Number of bases.
    pub fn len(&self) -> usize {
        self.bases.len()
    }

    /// True if there are no bases.
    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }

    /// Bases `from..=to` (1-based, inclusive), wrapping the origin when
    /// `from > to` on a circular sequence. Returns an empty vector for
    /// out-of-range coordinates.
    pub fn range(&self, from: u32, to: u32) -> Vec<u8> {
        let n = self.len() as u32;
        if from == 0 || to == 0 || from > n || to > n {
            return Vec::new();
        }
        let (from, to) = (from as usize - 1, to as usize);
        if from < to {
            self.bases[from..to].to_vec()
        } else if self.topology.is_circular() {
            let mut v = self.bases[from..].to_vec();
            v.extend_from_slice(&self.bases[..to]);
            v
        } else {
            Vec::new()
        }
    }

    /// True if the sequence contains only `ACGT` (any case).
    pub fn is_plain_dna(&self) -> bool {
        self.bases.iter().all(|&b| crate::alphabet::is_acgt(b))
    }

    /// Shifts the origin so that base `position` (1-based) becomes base 1
    /// (Sequence ▸ Set Origin). Features are rotated with it. No-op for
    /// linear sequences.
    pub fn set_origin(&mut self, position: u32) {
        if !self.topology.is_circular() || position <= 1 || position as usize > self.len() {
            return;
        }
        let n = self.len() as u32;
        self.bases.rotate_left(position as usize - 1);
        let shift = |p: u32| (p + n - position) % n + 1;
        for f in &mut self.features {
            f.start = shift(f.start);
            f.end = shift(f.end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(start: u32, end: u32, strand: Strand) -> Feature {
        Feature {
            name: "f".into(),
            description: String::new(),
            kind: "misc_feature".into(),
            start,
            end,
            strand,
            show: true,
            protect: false,
            arrow: true,
            color: Rgb(0, 0, 0),
        }
    }

    #[test]
    fn seq_type_and_topology_round_trip_through_xdna_bytes() {
        for byte in 0..=4u8 {
            assert_eq!(SeqType::from_xdna_byte(byte).unwrap().to_xdna_byte(), byte);
        }
        assert_eq!(SeqType::from_xdna_byte(5), None);
        assert_eq!(Topology::from_xdna_byte(1), Some(Topology::Circular));
        assert_eq!(Topology::from_xdna_byte(2), None);
        assert_eq!(Topology::Linear.toggled(), Topology::Circular);
    }

    #[test]
    fn feature_spans_follow_serial_cloner_coordinates() {
        // pBR322 ColE1 origin: xdna 3144 → 2516, GenBank complement(2516..3144)
        assert_eq!(
            feature(3144, 2516, Strand::Reverse).lo_hi(),
            (2516, 3144, false)
        );
        assert_eq!(
            feature(89, 1273, Strand::Forward).lo_hi(),
            (89, 1273, false)
        );
        // pEGFP-N1 "New Feature": 4000 → 200 forward on a 4733 bp plasmid wraps.
        let f = feature(4000, 200, Strand::Forward);
        assert_eq!(f.lo_hi(), (200, 4000, true));
        assert_eq!(f.len(4733), 934);
        assert_eq!(feature(3144, 2516, Strand::Reverse).len(4361), 629);
    }

    #[test]
    fn range_wraps_on_circular_sequences_only() {
        let mut s = Sequence::new("t", b"ABCDEFGH".to_vec());
        assert_eq!(s.range(2, 4), b"BCD");
        assert_eq!(s.range(7, 2), b"");
        s.topology = Topology::Circular;
        assert_eq!(s.range(7, 2), b"GHAB");
        assert_eq!(s.range(0, 2), b"");
        assert_eq!(s.range(1, 9), b"");
    }

    #[test]
    fn set_origin_rotates_bases_and_features() {
        let mut s = Sequence::new("t", b"ABCDEFGH".to_vec());
        s.topology = Topology::Circular;
        s.features.push(feature(2, 4, Strand::Forward));
        s.features.push(feature(8, 6, Strand::Reverse));
        s.set_origin(3);
        assert_eq!(s.bases, b"CDEFGHAB");
        assert_eq!((s.features[0].start, s.features[0].end), (8, 2));
        assert_eq!((s.features[1].start, s.features[1].end), (6, 4));
    }
}
