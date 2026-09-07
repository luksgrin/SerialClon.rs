//! Sequence file formats for SerialClon.rs.
//!
//! Every format has a `read` that yields [`Sequence`]s and, where the original
//! could write it, a `write`. [`detect`] sniffs the format from the bytes and
//! the file name the way Serial Cloner's Open command does; [`read`] is the
//! one-stop entry point.
//!
//! Formats in this milestone (M1): the native [`xdna`] (DNA Strider and Serial
//! Cloner), [`fasta`] and [`genbank`] (also EMBL-style continuation lines),
//! plus raw text. ApE, Vector NTI `.ma4`, MacVector `.nucl`, DNAStar `.seq`
//! and pDRAW32 `.pdw` follow in later milestones.

pub mod fasta;
pub mod genbank;
pub mod raw;
pub mod xdna;

use std::path::Path;

pub use sc_seq::Sequence;

/// Errors from reading or writing a sequence file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a valid file of the requested format.
    #[error("{0}")]
    Malformed(String),
    /// The file contained no sequence.
    #[error("no sequence found")]
    Empty,
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A recognised file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// `.xdna` — DNA Strider, with or without the Serial Cloner block.
    Xdna,
    /// FASTA, single or multi-record.
    Fasta,
    /// GenBank / EMBL / DDBJ flat file.
    GenBank,
    /// Anything else: bases are extracted from the text.
    RawText,
}

impl Format {
    /// The file extensions Serial Cloner associated with the format (from the
    /// manual: `.xdna`; `.fasta`/`.fsta`; `.txt`/`.text`/`.asc`/`.ascii`).
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Xdna => &["xdna"],
            Self::Fasta => &["fasta", "fsta", "fa", "fas", "fna"],
            Self::GenBank => &["gb", "gbk", "genbank", "embl"],
            Self::RawText => &["txt", "text", "asc", "ascii"],
        }
    }
}

/// Guesses the format from the content first and the extension second.
pub fn detect(bytes: &[u8], path: Option<&Path>) -> Format {
    if xdna::looks_like(bytes) {
        return Format::Xdna;
    }
    let head: String = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).into_owned();
    let first = head
        .lines()
        .map(str::trim_start)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if first.starts_with('>') {
        return Format::Fasta;
    }
    if head.contains("LOCUS") || head.starts_with("ID   ") || head.contains("\nID   ") {
        return Format::GenBank;
    }
    if let Some(ext) = path.and_then(|p| p.extension()).and_then(|e| e.to_str()) {
        let ext = ext.to_ascii_lowercase();
        for f in [Format::Xdna, Format::Fasta, Format::GenBank] {
            if f.extensions().contains(&ext.as_str()) {
                return f;
            }
        }
    }
    Format::RawText
}

/// Name for a sequence read from `path`: the file stem, as Serial Cloner shows
/// in the Sequence Window when the format carries no name.
pub fn name_from_path(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Reads every sequence in `bytes`, detecting the format. `path` is only used
/// for the extension and the default name.
pub fn read(bytes: &[u8], path: Option<&Path>) -> Result<Vec<Sequence>> {
    let name = path.map(name_from_path).unwrap_or_default();
    let seqs = match detect(bytes, path) {
        Format::Xdna => vec![xdna::read(bytes, &name)?],
        Format::Fasta => fasta::read(bytes),
        Format::GenBank => genbank::read(bytes)?,
        Format::RawText => vec![raw::read(bytes, &name)],
    };
    if seqs.is_empty() {
        Err(Error::Empty)
    } else {
        Ok(seqs)
    }
}

/// Reads every sequence in the file at `path`.
pub fn read_file(path: &Path) -> Result<Vec<Sequence>> {
    read(&std::fs::read(path)?, Some(path))
}

/// Splits text into lines on `\n`, `\r\n` or bare `\r` (classic Mac files,
/// which several of the original's sample files are).
pub(crate) fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split("\r\n")
        .flat_map(|chunk| chunk.split(['\r', '\n']))
}

/// Decodes bytes as ISO-8859-1: every byte maps to one char, so any input
/// survives a round trip through [`encode_latin1`] unchanged.
pub(crate) fn decode_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Inverse of [`decode_latin1`]; characters above U+00FF become `?`.
pub(crate) fn encode_latin1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect()
}
