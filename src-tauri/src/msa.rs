//! Generate ▸ Align Multiple Sequences (M6b): the MAFFT-based window.
//!
//! The alignment runs on a worker thread; the window gets every row with
//! its document's features so it can draw them above the aligned residues.

use sc_align::multiple::{self, Input, Msa, Options};
use sc_seq::{SeqType, Sequence, Source};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::AppState;
use crate::commands::open_product;
use crate::documents::{DocId, DocumentView, FeatureView};

/// An open document offered by the sequence picker.
#[derive(Debug, Serialize)]
pub struct MsaCandidate {
    /// Document id.
    pub id: DocId,
    /// Name.
    pub name: String,
    /// Type label.
    pub seq_type: String,
    /// True for protein.
    pub protein: bool,
    /// Residues.
    pub length: usize,
}

/// The open documents, in id order.
#[tauri::command]
pub fn msa_candidates(app: AppHandle) -> Vec<MsaCandidate> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let mut out: Vec<MsaCandidate> = store
        .iter()
        .map(|(id, d)| MsaCandidate {
            id,
            name: d.seq.name.clone(),
            seq_type: d.seq.seq_type.label().to_owned(),
            protein: d.seq.seq_type == SeqType::Protein,
            length: d.seq.len(),
        })
        .collect();
    out.sort_by_key(|c| c.id);
    out
}

/// One aligned row with its document's features.
#[derive(Debug, Serialize)]
pub struct MsaRowView {
    /// Document id.
    pub id: DocId,
    /// Features of the document (original coordinates).
    pub features: Vec<FeatureView>,
}

/// The window's result.
#[derive(Debug, Serialize)]
pub struct MultipleAlignmentView {
    /// The alignment.
    pub msa: Msa,
    /// Per-row document data, in the same order as `msa.rows`.
    pub rows: Vec<MsaRowView>,
    /// Wall-clock time of the alignment.
    pub elapsed_ms: u128,
}

/// Aligns the given open documents on a worker thread.
#[tauri::command]
pub async fn align_multiple(
    app: AppHandle,
    ids: Vec<DocId>,
    options: Options,
) -> Result<MultipleAlignmentView, String> {
    let (inputs, rows) = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let mut inputs = Vec::new();
        let mut rows = Vec::new();
        for id in &ids {
            let doc = store
                .get(*id)
                .ok_or_else(|| format!("document {id} is not open any more"))?;
            inputs.push(Input {
                name: doc.seq.name.clone(),
                bases: doc.seq.bases.clone(),
                seq_type: doc.seq.seq_type,
            });
            rows.push(MsaRowView {
                id: *id,
                features: doc.seq.features.iter().map(FeatureView::from).collect(),
            });
        }
        (inputs, rows)
    };
    let started = std::time::Instant::now();
    let msa =
        tauri::async_runtime::spawn_blocking(move || multiple::align_multiple(&inputs, &options))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
    Ok(MultipleAlignmentView {
        msa,
        rows,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

/// The alignment as text (Clustal or FASTA), for Copy.
#[tauri::command]
pub fn msa_text(msa: Msa, clustal: bool) -> String {
    if clustal {
        multiple::to_clustal(&msa)
    } else {
        multiple::to_fasta(&msa)
    }
}

/// Opens the consensus as a new Sequence Window.
#[tauri::command]
pub fn extract_msa_consensus(app: AppHandle, msa: Msa) -> DocumentView {
    let names: Vec<&str> = msa.rows.iter().map(|r| r.name.as_str()).collect();
    let mut seq = Sequence::new(
        format!("Consensus of {} sequences", msa.rows.len()),
        msa.consensus.as_bytes().to_vec(),
    );
    seq.seq_type = match msa.kind {
        sc_align::Kind::Protein => SeqType::Protein,
        sc_align::Kind::Dna => SeqType::DegenerateDna,
    };
    seq.source = Source::New;
    seq.comment = format!(
        "Majority consensus of the MAFFT {} alignment of {} ({} columns; ties as IUPAC codes, columns with a majority of gaps dropped{})",
        msa.mode.label(),
        names.join(", "),
        msa.length,
        if msa.rows.iter().any(|r| r.reversed) {
            "; reverse-complemented by adjustdirection: ".to_owned()
                + &msa
                    .rows
                    .iter()
                    .filter(|r| r.reversed)
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
        } else {
            String::new()
        }
    );
    open_product(&app, seq)
}

/// Saves the alignment as FASTA or Clustal. `Ok(false)` if cancelled.
#[tauri::command]
pub async fn export_msa(app: AppHandle, msa: Msa, clustal: bool) -> Result<bool, String> {
    let (filter, ext, text) = if clustal {
        ("Clustal alignment", "aln", multiple::to_clustal(&msa))
    } else {
        ("Aligned FASTA", "fasta", multiple::to_fasta(&msa))
    };
    let Some(file) = app
        .dialog()
        .file()
        .set_title("Save the alignment")
        .add_filter(filter, &[ext])
        .set_file_name(format!("alignment.{ext}"))
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}
