//! Protein windows, genetic codes, pI, codon usage and reverse
//! translation, bisulfite, alignment, and the network features (Web Access,
//! BLAST). Settings (genetic code, pK table, codon table) persist in
//! `settings.json`; user codon tables in `{app data}/codon-usage/`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sc_seq::codon::{CodonTable, CodonUsage, builtin_tables};
use sc_seq::pi::{PK_TABLES, isoelectric_point, pk_table};
use sc_seq::translate::{GeneticCode, STANDARD};
use sc_seq::{Feature, SeqType, Sequence, Source, Strand};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;
use crate::commands::{broadcast, open_product, view_of};
use crate::documents::{DocId, Document, DocumentView, FeatureView};
use crate::{menu, windows};

/// Persisted choices, mirrored in Preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// NCBI genetic code id.
    pub genetic_code: u8,
    /// pK table name (`sc_seq::pi`).
    pub pk_table: String,
    /// Selected codon usage table slug.
    pub codon_table: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            genetic_code: 1,
            pk_table: sc_seq::pi::DEFAULT.name.to_owned(),
            codon_table: "h-sapiens".to_owned(),
        }
    }
}

/// Protein, codon usage and alignment state.
pub struct ProteinState {
    /// Choices.
    pub settings: Mutex<Settings>,
    /// Built-in plus user codon tables.
    pub codon_tables: Mutex<Vec<CodonTable>>,
    /// The HTTP client, created on first use (dropped when the proxy
    /// preference changes).
    pub net: Mutex<Option<Arc<sc_net::Client>>>,
}

impl Default for ProteinState {
    fn default() -> Self {
        ProteinState {
            settings: Mutex::new(Settings::default()),
            codon_tables: Mutex::new(builtin_tables()),
            net: Mutex::new(None),
        }
    }
}

fn settings_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("settings.json"))
}

fn codon_dir(app: &AppHandle) -> Option<PathBuf> {
    let d = app.path().app_data_dir().ok()?.join("codon-usage");
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

/// Loads settings and user codon tables at startup.
pub fn load(app: &AppHandle) {
    let state = app.state::<ProteinState>();
    if let Some(f) = settings_file(app)
        && let Ok(text) = std::fs::read_to_string(f)
        && let Ok(s) = serde_json::from_str::<Settings>(&text)
    {
        *state.settings.lock().unwrap() = s;
    }
    if let Some(dir) = codon_dir(app)
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        let mut tables = state.codon_tables.lock().unwrap();
        for e in entries.filter_map(|e| e.ok()) {
            if let Ok(text) = std::fs::read_to_string(e.path())
                && let Ok(mut t) = CodonTable::from_json(&text)
            {
                t.builtin = false;
                tables.retain(|x| x.slug != t.slug);
                tables.push(t);
            }
        }
    }
}

fn save_settings(app: &AppHandle) {
    let state = app.state::<ProteinState>();
    if let Some(f) = settings_file(app) {
        if let Some(d) = f.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let s = state.settings.lock().unwrap();
        let _ = std::fs::write(f, serde_json::to_string_pretty(&*s).unwrap_or_default());
    }
}

/// The shared HTTP client (built on first use, so an offline start costs
/// nothing), honouring the proxy preference.
pub fn client(app: &AppHandle) -> Result<Arc<sc_net::Client>, String> {
    let state = app.state::<ProteinState>();
    let mut slot = state.net.lock().unwrap();
    if let Some(c) = slot.as_ref() {
        return Ok(c.clone());
    }
    let proxy = crate::prefs::get(app).proxy;
    let c = sc_net::Client::with_proxy(
        "SerialClon",
        "serialclon@users.noreply.github.com",
        if proxy.trim().is_empty() {
            None
        } else {
            Some(proxy.as_str())
        },
    )
    .map(Arc::new)
    .map_err(|e| e.to_string())?;
    *slot = Some(c.clone());
    Ok(c)
}

/// Forgets the HTTP client so the next request uses the new proxy setting.
pub fn reset_net(app: &AppHandle) {
    *app.state::<ProteinState>().net.lock().unwrap() = None;
}

/// The genetic code chosen in Preferences — or, with "use the codon usage
/// table as the genetic code" on, the code implied by the selected table.
pub fn current_code(app: &AppHandle) -> &'static GeneticCode {
    if crate::prefs::get(app).codon_as_code
        && let Some(table) = current_codon_table(app)
    {
        // Codes derived from tables are leaked once per table and cached.
        static CACHE: Mutex<Vec<(String, &'static GeneticCode)>> = Mutex::new(Vec::new());
        let mut cache = CACHE.lock().unwrap();
        if let Some((_, code)) = cache.iter().find(|(slug, _)| *slug == table.slug) {
            return code;
        }
        let code: &'static GeneticCode = Box::leak(Box::new(table.genetic_code()));
        cache.push((table.slug.clone(), code));
        return code;
    }
    let id = app
        .state::<ProteinState>()
        .settings
        .lock()
        .unwrap()
        .genetic_code;
    GeneticCode::by_id(id).unwrap_or(&STANDARD)
}

fn current_codon_table(app: &AppHandle) -> Option<CodonTable> {
    let state = app.state::<ProteinState>();
    let slug = state.settings.lock().unwrap().codon_table.clone();
    let tables = state.codon_tables.lock().unwrap();
    tables
        .iter()
        .find(|t| t.slug == slug)
        .or_else(|| tables.first())
        .cloned()
}

// ---------------------------------------------------------- settings ----

/// A genetic code for the menu.
#[derive(Debug, Serialize)]
pub struct GeneticCodeView {
    /// NCBI id.
    pub id: u8,
    /// Name.
    pub name: String,
    /// Start codons.
    pub starts: Vec<String>,
    /// Stop codons.
    pub stops: Vec<String>,
    /// Currently selected.
    pub selected: bool,
}

/// Protein ▸ Choose genetic code: every NCBI table.
#[tauri::command]
pub fn genetic_codes(app: AppHandle) -> Vec<GeneticCodeView> {
    let current = current_code(&app).id;
    sc_seq::genetic_codes::GENETIC_CODES
        .iter()
        .map(|c| GeneticCodeView {
            id: c.id,
            name: c.name.to_owned(),
            starts: c.start_codons(),
            stops: c.stop_codons(),
            selected: c.id == current,
        })
        .collect()
}

/// Selects the genetic code used by translation.
#[tauri::command]
pub fn set_genetic_code(app: AppHandle, id: u8) -> Result<(), String> {
    GeneticCode::by_id(id).ok_or("unknown genetic code")?;
    app.state::<ProteinState>()
        .settings
        .lock()
        .unwrap()
        .genetic_code = id;
    save_settings(&app);
    menu::refresh_protein(&app);
    let _ = app.emit("settings:changed", ());
    Ok(())
}

/// The pK tables (Preferences ▸ pI).
#[tauri::command]
pub fn pk_tables(app: AppHandle) -> Vec<(String, bool)> {
    let current = app
        .state::<ProteinState>()
        .settings
        .lock()
        .unwrap()
        .pk_table
        .clone();
    PK_TABLES
        .iter()
        .map(|t| (t.name.to_owned(), t.name == current))
        .collect()
}

/// Selects the pK table used for pI.
#[tauri::command]
pub fn set_pk_table(app: AppHandle, name: String) {
    app.state::<ProteinState>()
        .settings
        .lock()
        .unwrap()
        .pk_table = pk_table(&name).name.to_owned();
    save_settings(&app);
    menu::refresh_protein(&app);
    let _ = app.emit("settings:changed", ());
}

/// pI of a peptide with the chosen table.
pub fn pi_of(app: &AppHandle, peptide: &[u8]) -> Option<f64> {
    let name = app
        .state::<ProteinState>()
        .settings
        .lock()
        .unwrap()
        .pk_table
        .clone();
    isoelectric_point(peptide, pk_table(&name))
}

// ------------------------------------------------------- codon usage ----

/// A codon table for the Manage Codon Usage window.
#[derive(Debug, Serialize)]
pub struct CodonTableView {
    /// Name.
    pub name: String,
    /// Id.
    pub slug: String,
    /// Source.
    pub source: String,
    /// Retrieval date.
    pub retrieved: String,
    /// CDS count.
    pub cds: Option<u64>,
    /// Built in.
    pub builtin: bool,
    /// Selected for reverse translation.
    pub selected: bool,
}

fn table_view(t: &CodonTable, selected: &str) -> CodonTableView {
    CodonTableView {
        name: t.name.clone(),
        slug: t.slug.clone(),
        source: t.source.clone(),
        retrieved: t.retrieved.clone(),
        cds: t.cds,
        builtin: t.builtin,
        selected: t.slug == selected,
    }
}

/// Manage Codon Usage: every table.
#[tauri::command]
pub fn codon_tables(app: AppHandle) -> Vec<CodonTableView> {
    let state = app.state::<ProteinState>();
    let selected = state.settings.lock().unwrap().codon_table.clone();
    state
        .codon_tables
        .lock()
        .unwrap()
        .iter()
        .map(|t| table_view(t, &selected))
        .collect()
}

/// The 64 rows of a table.
#[tauri::command]
pub fn codon_table_rows(app: AppHandle, slug: String) -> Result<Vec<CodonUsage>, String> {
    let state = app.state::<ProteinState>();
    let tables = state.codon_tables.lock().unwrap();
    tables
        .iter()
        .find(|t| t.slug == slug)
        .map(|t| t.codons.clone())
        .ok_or_else(|| "no such table".into())
}

/// Selects the table used by reverse translation (shown in the Protein menu).
#[tauri::command]
pub fn select_codon_table(app: AppHandle, slug: String) -> Result<(), String> {
    let state = app.state::<ProteinState>();
    if !state
        .codon_tables
        .lock()
        .unwrap()
        .iter()
        .any(|t| t.slug == slug)
    {
        return Err("no such table".into());
    }
    state.settings.lock().unwrap().codon_table = slug;
    save_settings(&app);
    menu::refresh_protein(&app);
    Ok(())
}

fn store_user_table(app: &AppHandle, mut table: CodonTable) -> CodonTableView {
    table.builtin = false;
    if table.slug.is_empty() {
        table.slug = sc_seq::codon::slug(&table.name);
    }
    if let Some(dir) = codon_dir(app) {
        let _ = std::fs::write(dir.join(format!("{}.json", table.slug)), table.to_json());
    }
    let state = app.state::<ProteinState>();
    let selected = state.settings.lock().unwrap().codon_table.clone();
    let view = table_view(&table, &selected);
    let mut tables = state.codon_tables.lock().unwrap();
    tables.retain(|t| t.slug != table.slug);
    tables.push(table);
    view
}

/// Import Codon Usage: from pasted text (Kazusa or GCG layouts) or from a
/// URL fetched on request.
#[tauri::command]
pub async fn import_codon_table(
    app: AppHandle,
    name: String,
    text: Option<String>,
    url: Option<String>,
) -> Result<CodonTableView, String> {
    let (body, source) = match (text, url) {
        (Some(t), _) if !t.trim().is_empty() => (t, "pasted text".to_owned()),
        (_, Some(u)) if !u.trim().is_empty() => (
            client(&app)?
                .get_text(u.trim())
                .map_err(|e| e.to_string())?,
            u.trim().to_owned(),
        ),
        _ => return Err("paste a table or give a URL".into()),
    };
    let name = if name.trim().is_empty() {
        "Imported table".to_owned()
    } else {
        name.trim().to_owned()
    };
    let mut table = CodonTable::parse_text(&body, &name).map_err(|e| e.to_string())?;
    table.source = source;
    table.retrieved = crate::io::today();
    Ok(store_user_table(&app, table))
}

/// Deletes a user table (built-in ones cannot be deleted).
#[tauri::command]
pub fn delete_codon_table(app: AppHandle, slug: String) -> Result<(), String> {
    let state = app.state::<ProteinState>();
    {
        let mut tables = state.codon_tables.lock().unwrap();
        let Some(t) = tables.iter().find(|t| t.slug == slug) else {
            return Err("no such table".into());
        };
        if t.builtin {
            return Err("built-in tables cannot be deleted".into());
        }
        tables.retain(|t| t.slug != slug);
    }
    if let Some(dir) = codon_dir(&app) {
        let _ = std::fs::remove_file(dir.join(format!("{slug}.json")));
    }
    let mut s = state.settings.lock().unwrap();
    if s.codon_table == slug {
        s.codon_table = "ecoli-k12".into();
    }
    drop(s);
    save_settings(&app);
    menu::refresh_protein(&app);
    Ok(())
}

/// The name of the selected codon table (for the Protein menu).
pub fn current_codon_table_name(app: &AppHandle) -> String {
    current_codon_table(app).map(|t| t.name).unwrap_or_default()
}

// ---------------------------------------------- protein documents ----

/// File ▸ New Protein Sequence.
#[tauri::command]
pub fn new_protein(app: AppHandle) -> DocumentView {
    let id = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().unwrap();
        let id = store.new_blank(false);
        if let Some(d) = store.get_mut(id) {
            d.seq.seq_type = SeqType::Protein;
        }
        id
    };
    windows::open_sequence(&app, id);
    menu::refresh_windows(&app);
    view_of(&app, id).expect("just inserted")
}

/// File ▸ Open Protein Sequence: like Open, but the sequences are typed as
/// protein whatever their letters look like.
#[tauri::command]
pub async fn open_protein_files(app: AppHandle) -> Vec<DocId> {
    let ids = crate::io::open_dialog(&app);
    let state = app.state::<AppState>();
    for &id in &ids {
        if let Some(d) = state.store.lock().unwrap().get_mut(id) {
            d.seq.seq_type = SeqType::Protein;
        }
        broadcast(&app, id);
    }
    ids
}

/// The Info panel's peptide → a new Protein window (context menu / File
/// menu when the protein field is active).
#[tauri::command]
pub fn peptide_to_document(
    app: AppHandle,
    id: DocId,
    from: usize,
    to: usize,
    first_orf: bool,
) -> Result<DocumentView, String> {
    let (peptide, name, comment) = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        let n = doc.seq.len();
        let (from, to) = (from.min(n), to.min(n).max(from.min(n)));
        let peptide = current_code(&app).translate(&doc.seq.bases[from..to], !first_orf);
        (
            peptide,
            format!("{} [{}-{}] protein", doc.seq.name, from + 1, to),
            format!(
                "Translation of {} from {} to {} ({})",
                doc.seq.name,
                from + 1,
                to,
                current_code(&app).name
            ),
        )
    };
    let mut seq = Sequence::new(name, peptide.trim_end_matches('*').as_bytes().to_vec());
    seq.seq_type = SeqType::Protein;
    seq.comment = comment;
    seq.source = Source::New;
    Ok(open_product(&app, seq))
}

/// Protein ▸ Reverse translation of a protein window (or of the selected
/// residues), with the selected codon usage table; features come along
/// (changelog 2.6) with their coordinates ×3.
#[tauri::command]
pub fn reverse_translate(
    app: AppHandle,
    id: DocId,
    from: usize,
    to: usize,
) -> Result<DocumentView, String> {
    let table = current_codon_table(&app).ok_or("no codon usage table selected")?;
    let seq = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        if doc.seq.seq_type != SeqType::Protein {
            return Err("Reverse translation applies to a Protein window (Info panel ▸ Open as Protein Sequence makes one).".into());
        }
        let n = doc.seq.len();
        let (from, to) = if from == to {
            (0, n)
        } else {
            (from.min(n), to.min(n))
        };
        let peptide = &doc.seq.bases[from..to];
        let dna = table.reverse_translate(peptide);
        let features: Vec<Feature> = doc
            .seq
            .features
            .iter()
            .filter(|f| {
                let (lo, hi, wraps) = f.lo_hi();
                !wraps && lo as usize > from && hi as usize <= to
            })
            .map(|f| {
                let shift = |p: u32| (p as usize - from) as u32;
                let (a, b) = (shift(f.start), shift(f.end));
                Feature {
                    start: if f.strand == Strand::Forward {
                        a * 3 - 2
                    } else {
                        a * 3
                    },
                    end: if f.strand == Strand::Forward {
                        b * 3
                    } else {
                        b * 3 - 2
                    },
                    ..f.clone()
                }
            })
            .collect();
        let mut s = Sequence::new(format!("{} (reverse translated)", doc.seq.name), dna);
        s.seq_type = SeqType::Dna;
        s.comment = format!(
            "Reverse translation of {} [{}-{}] with the {} codon usage table (most used codon per amino acid)\r{}",
            doc.seq.name,
            from + 1,
            to,
            table.name,
            doc.seq.comment.split(['\r', '\n']).next().unwrap_or("")
        );
        s.features = features;
        s
    };
    Ok(open_product(&app, seq))
}

/// Sequence ▸ Bisulfite conversion (sense or antisense strand).
#[tauri::command]
pub fn bisulfite(app: AppHandle, id: DocId, antisense: bool) -> Result<DocumentView, String> {
    let seq = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        if doc.seq.seq_type == SeqType::Protein {
            return Err("Bisulfite conversion applies to DNA.".into());
        }
        let strand = if antisense {
            sc_seq::bisulfite::Strand::Antisense
        } else {
            sc_seq::bisulfite::Strand::Sense
        };
        let bases = sc_seq::bisulfite::convert(&doc.seq.bases, strand);
        let n = doc.seq.len() as u32;
        let features: Vec<Feature> = doc
            .seq
            .features
            .iter()
            .map(|f| {
                if antisense {
                    Feature {
                        start: n + 1 - f.start,
                        end: n + 1 - f.end,
                        strand: if f.strand == Strand::Forward {
                            Strand::Reverse
                        } else {
                            Strand::Forward
                        },
                        ..f.clone()
                    }
                } else {
                    f.clone()
                }
            })
            .collect();
        let which = if antisense { "antisense" } else { "sense" };
        let mut s = Sequence::new(format!("{} bisulfite {which}", doc.seq.name), bases);
        s.seq_type = doc.seq.seq_type;
        s.topology = doc.seq.topology;
        s.features = features;
        s.comment = format!(
            "Bisulfite-converted {which} strand of {} [{} bp]: unmethylated C read as T\r{}",
            doc.seq.name,
            n,
            doc.seq.comment.split(['\r', '\n']).next().unwrap_or("")
        );
        s
    };
    Ok(open_product(&app, seq))
}

/// Translates text in the three frames with the chosen genetic code (Align
/// window's 3-frame display).
#[tauri::command]
pub fn translate_frames(app: AppHandle, text: String) -> [String; 3] {
    let code = current_code(&app);
    let bases: Vec<u8> = text.bytes().filter(|b| b.is_ascii_alphabetic()).collect();
    let frame = |f: usize| {
        if bases.len() > f {
            code.translate(&bases[f..], true)
        } else {
            String::new()
        }
    };
    [frame(0), frame(1), frame(2)]
}

// ------------------------------------------------------------- align ----

/// The Align window's result.
#[derive(Debug, Serialize)]
pub struct AlignView {
    /// The alignment.
    pub alignment: sc_align::Alignment,
    /// Name of sequence 1.
    pub a_name: String,
    /// Name of sequence 2.
    pub b_name: String,
    /// Length of sequence 1 (as aligned: residues when `protein`).
    pub a_len: usize,
    /// Length of sequence 2.
    pub b_len: usize,
    /// Features of sequence 1 (coordinates of the aligned molecule).
    pub a_features: Vec<FeatureView>,
    /// Features of sequence 2.
    pub b_features: Vec<FeatureView>,
    /// True when the two ids were swapped ("align short sequences").
    pub swapped: bool,
    /// Sequence 1 id.
    pub a_id: DocId,
    /// Sequence 2 id.
    pub b_id: DocId,
}

fn aligned_form(app: &AppHandle, doc: &Document, protein: bool) -> (Vec<u8>, Vec<FeatureView>) {
    if protein && doc.seq.seq_type != SeqType::Protein {
        // DNA ↔ protein switch: align the frame-1 translations.
        let pep = current_code(app)
            .translate(&doc.seq.bases, true)
            .into_bytes();
        let features = doc
            .seq
            .features
            .iter()
            .filter(|f| f.strand == Strand::Forward)
            .map(|f| FeatureView {
                start: f.start.div_ceil(3),
                end: f.end.div_ceil(3),
                ..FeatureView::from(f)
            })
            .collect();
        (pep, features)
    } else {
        (
            doc.seq.bases.clone(),
            doc.seq.features.iter().map(FeatureView::from).collect(),
        )
    }
}

/// Generate ▸ Align two Sequences: local alignment of two open documents.
#[tauri::command]
pub fn align_documents(
    app: AppHandle,
    a: DocId,
    b: DocId,
    protein: bool,
    short: bool,
) -> Result<AlignView, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let da = store.get(a).ok_or("first sequence is not open any more")?;
    let db = store.get(b).ok_or("second sequence is not open any more")?;
    let protein =
        protein || da.seq.seq_type == SeqType::Protein || db.seq.seq_type == SeqType::Protein;
    let (mut ta, mut fa) = aligned_form(&app, da, protein);
    let (mut tb, mut fb) = aligned_form(&app, db, protein);
    let (mut a_id, mut b_id) = (a, b);
    let (mut a_name, mut b_name) = (da.seq.name.clone(), db.seq.name.clone());
    let mut swapped = false;
    if short && ta.len() > tb.len() {
        std::mem::swap(&mut ta, &mut tb);
        std::mem::swap(&mut fa, &mut fb);
        std::mem::swap(&mut a_id, &mut b_id);
        std::mem::swap(&mut a_name, &mut b_name);
        swapped = true;
    }
    let kind = if protein {
        sc_align::Kind::Protein
    } else {
        sc_align::Kind::Dna
    };
    let alignment = sc_align::align(&ta, &tb, kind).map_err(|e| e.to_string())?;
    Ok(AlignView {
        alignment,
        a_name,
        b_name,
        a_len: ta.len(),
        b_len: tb.len(),
        a_features: fa,
        b_features: fb,
        swapped,
        a_id,
        b_id,
    })
}

/// Align ▸ Extract consensus into a new Sequence Window.
#[tauri::command]
pub fn align_consensus(
    app: AppHandle,
    alignment: sc_align::Alignment,
    a_name: String,
    b_name: String,
) -> DocumentView {
    let bases = sc_align::consensus(&alignment);
    let mut seq = Sequence::new(format!("Consensus {a_name} / {b_name}"), bases);
    seq.seq_type = match alignment.kind {
        sc_align::Kind::Protein => SeqType::Protein,
        sc_align::Kind::Dna => SeqType::DegenerateDna,
    };
    seq.comment = format!(
        "Consensus of the local alignment of {a_name} [{}-{}] and {b_name} [{}-{}{}]: {:.1}% identity over {} columns; mismatches as IUPAC codes",
        alignment.a_start + 1,
        alignment.a_end,
        alignment.b_start + 1,
        alignment.b_end,
        if alignment.b_reverse {
            ", reverse strand"
        } else {
            ""
        },
        alignment.identity,
        alignment.length
    );
    open_product(&app, seq)
}

// --------------------------------------------------------- Web Access ----

/// A fetched record.
#[derive(Debug, Serialize)]
pub struct RecordView {
    /// The flat file.
    pub text: String,
    /// `genbank`, `genpept` or `embl`.
    pub format: String,
    /// Suggested name.
    pub name: String,
}

/// Web Access ▸ Fetch: an accession from NCBI (nucleotide or protein) or ENA.
#[tauri::command]
pub async fn web_fetch(
    app: AppHandle,
    source: String,
    accession: String,
) -> Result<RecordView, String> {
    let c = client(&app)?;
    let acc = accession.trim().to_owned();
    if acc.is_empty() {
        return Err("enter an accession number".into());
    }
    let (text, format) = match source.as_str() {
        "ncbi-protein" => (
            c.ncbi_fetch(sc_net::NcbiDb::Protein, &acc)
                .map_err(|e| e.to_string())?,
            "genpept",
        ),
        "ena" => (c.ena_fetch(&acc).map_err(|e| e.to_string())?, "embl"),
        _ => (
            c.ncbi_fetch(sc_net::NcbiDb::Nuccore, &acc)
                .map_err(|e| e.to_string())?,
            "genbank",
        ),
    };
    Ok(RecordView {
        text,
        format: format.into(),
        name: acc,
    })
}

/// Web Access ▸ Create a New Sequence Window from a fetched record.
#[tauri::command]
pub fn web_create_document(
    app: AppHandle,
    text: String,
    name: String,
    protein: bool,
) -> Result<DocumentView, String> {
    let mut seqs = sc_format::read(text.as_bytes(), None).map_err(|e| e.to_string())?;
    let mut seq = seqs.remove(0);
    if seq.name.is_empty() {
        seq.name = name;
    }
    if protein {
        seq.seq_type = SeqType::Protein;
    }
    seq.source = Source::GenBank;
    let mut doc_view = open_product(&app, seq);
    // open_product marks products dirty; a fetched record is a fresh document.
    let state = app.state::<AppState>();
    if let Some(d) = state.store.lock().unwrap().get_mut(doc_view.id) {
        d.dirty = false;
        doc_view.dirty = false;
    }
    broadcast(&app, doc_view.id);
    Ok(doc_view)
}

/// A submitted BLAST job.
#[derive(Debug, Serialize)]
pub struct BlastJobView {
    /// RID.
    pub rid: String,
    /// Estimated seconds.
    pub rtoe: u64,
}

/// Submits a BLAST search of `from..to` of a document (whole document when
/// equal), against a database or against another open document (BLAST2Seq).
#[tauri::command]
pub async fn blast_submit(
    app: AppHandle,
    id: DocId,
    from: usize,
    to: usize,
    program: String,
    database: Option<String>,
    subject: Option<DocId>,
) -> Result<BlastJobView, String> {
    let (query, subject_text) = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        let n = doc.seq.len();
        let (from, to) = if from == to {
            (0, n)
        } else {
            (from.min(n), to.min(n))
        };
        let query = format!(
            ">{}\n{}\n",
            doc.seq.name.replace(' ', "_"),
            String::from_utf8_lossy(&doc.seq.bases[from..to])
        );
        let subject_text = match subject {
            Some(s) => {
                let sd = store
                    .get(s)
                    .ok_or("the subject sequence is not open any more")?;
                Some(format!(
                    ">{}\n{}\n",
                    sd.seq.name.replace(' ', "_"),
                    String::from_utf8_lossy(&sd.seq.bases)
                ))
            }
            None => None,
        };
        (query, subject_text)
    };
    let job = client(&app)?
        .blast_put(&sc_net::BlastRequest {
            program,
            database: database.filter(|d| !d.trim().is_empty()),
            query,
            subject: subject_text,
            hitlist_size: 50,
        })
        .map_err(|e| e.to_string())?;
    Ok(BlastJobView {
        rid: job.rid,
        rtoe: job.rtoe,
    })
}

/// The state of a BLAST job.
#[derive(Debug, Serialize)]
pub struct BlastStatusView {
    /// `waiting`, `ready`, `nohits`, `failed`, `unknown`.
    pub status: sc_net::BlastStatus,
    /// Seconds before the next poll is allowed.
    pub wait: u64,
}

/// Polls a BLAST job (NCBI's once-a-minute rule is enforced; `wait` says
/// how long to hold off).
#[tauri::command]
pub async fn blast_status(app: AppHandle, rid: String) -> Result<BlastStatusView, String> {
    let c = client(&app)?;
    let wait = c.seconds_until_poll(&rid);
    if wait > 0 {
        return Ok(BlastStatusView {
            status: sc_net::BlastStatus::Waiting,
            wait,
        });
    }
    let status = c.blast_status(&rid).map_err(|e| e.to_string())?;
    Ok(BlastStatusView {
        status,
        wait: c.seconds_until_poll(&rid),
    })
}

/// The text report of a finished job.
#[tauri::command]
pub async fn blast_result(app: AppHandle, rid: String) -> Result<String, String> {
    client(&app)?
        .blast_result(&rid, "Text")
        .map_err(|e| e.to_string())
}

/// Opens the Web Access window, optionally preset to BLAST a document's
/// selection (`program`: blastn / blastp).
pub fn open_web(app: &AppHandle, preset: Option<(DocId, usize, usize, String)>) {
    windows::open_reference(app, "web");
    if let Some((id, from, to, program)) = preset {
        let payload = serde_json::json!({ "id": id, "from": from, "to": to, "program": program });
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(600));
            let _ = app.emit_to("web", "web:preset", payload);
        });
    }
}
