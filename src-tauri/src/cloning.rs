//! The cloning command surface: Build a Construct, Run a PCR, Create an
//! Adaptor, Make shRNA, Gateway cloning, Site Usage, the Enzyme Library,
//! the Virtual Cutter and the Silent Map.

use sc_clone::ligate::{BluntMode, Side, blunt_with, closes, ligate};
use sc_clone::{SiteRef, extract_between};
use sc_enzyme::library::parse_entry;
use sc_seq::{Overhang, Sequence};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::AppState;
use crate::commands::{EnzymeEntry, SiteRefView, open_product, view_of};
use crate::documents::{DocId, DocumentView};
use crate::windows;

// ------------------------------------------------------- Build a Construct --

/// One slot of the Build a Construct window.
#[derive(Debug, Clone, Deserialize)]
pub struct FragmentSpec {
    /// Source document.
    pub doc: DocId,
    /// BEGIN site (absent = the whole linear sequence).
    pub begin: Option<SiteRefView>,
    /// END site (absent on a circle = the whole linearised plasmid).
    pub end: Option<SiteRefView>,
    /// CW / sense.
    pub clockwise: bool,
    /// How the left end is processed before ligation.
    pub blunt_left: BluntChoice,
    /// How the right end is processed.
    pub blunt_right: BluntChoice,
}

/// The original's three radio buttons per extremity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BluntChoice {
    /// Non-processed: the end keeps its overhang.
    #[default]
    None,
    /// Blunt (Fill-In).
    Fill,
    /// Blunt (Remove).
    Remove,
}

/// What a ligation would produce, for the original's two confirmations.
#[derive(Debug, Clone, Serialize)]
pub struct LigationPlan {
    /// The last right end ligates to the first left end: a circular plasmid.
    /// Otherwise the product is linear ("Ligation will produce a linear
    /// fragment and not a circular plasmid").
    pub circular: bool,
    /// Only one fragment ("Only one fragment/vector was selected and no
    /// insert").
    pub single: bool,
}

/// Checks the ends before ligating, like the original's Ligate button.
#[tauri::command]
pub fn ligation_plan(app: AppHandle, fragments: Vec<FragmentSpec>) -> Result<LigationPlan, String> {
    let parts: Vec<Sequence> = fragments
        .iter()
        .map(|s| resolve(&app, s))
        .collect::<Result<_, _>>()?;
    if parts.is_empty() {
        return Err("No ligation can proceed without a Fragment #1.".into());
    }
    Ok(LigationPlan {
        circular: closes(&parts),
        single: parts.len() == 1,
    })
}

/// What a slot resolves to, for display before ligation.
#[derive(Debug, Clone, Serialize)]
pub struct FragmentPreview {
    /// Name.
    pub name: String,
    /// Length in bases (extent).
    pub length: usize,
    /// Left end description.
    pub left: String,
    /// Right end description.
    pub right: String,
    /// Number of features it carries.
    pub features: usize,
}

fn resolve(app: &AppHandle, spec: &FragmentSpec) -> Result<Sequence, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(spec.doc).ok_or("no such document")?;
    let mut fragment = match &spec.begin {
        None => {
            if doc.seq.topology.is_circular() {
                return Err(format!(
                    "\"{}\" is circular: choose a site to open it",
                    doc.seq.name
                ));
            }
            doc.seq.clone()
        }
        Some(b) => {
            let begin: SiteRef = b.clone().into();
            let end: SiteRef = spec
                .end
                .clone()
                .map(Into::into)
                .unwrap_or_else(|| begin.clone());
            extract_between(&doc.seq, &begin, &end, spec.clockwise)
                .ok_or("nothing between these sites")?
        }
    };
    let mode = |c: BluntChoice| match c {
        BluntChoice::None => None,
        BluntChoice::Fill => Some(BluntMode::Fill),
        BluntChoice::Remove => Some(BluntMode::Remove),
    };
    if let Some(m) = mode(spec.blunt_left) {
        blunt_with(&mut fragment, Side::Left, m);
    }
    if let Some(m) = mode(spec.blunt_right) {
        blunt_with(&mut fragment, Side::Right, m);
    }
    Ok(fragment)
}

fn describe(o: &Overhang) -> String {
    sc_clone::ligate::describe(o)
}

/// Previews every slot.
#[tauri::command]
pub fn preview_fragments(
    app: AppHandle,
    fragments: Vec<FragmentSpec>,
) -> Result<Vec<FragmentPreview>, String> {
    fragments
        .iter()
        .map(|spec| {
            let f = resolve(&app, spec)?;
            Ok(FragmentPreview {
                name: f.name.clone(),
                length: f.len(),
                left: describe(&f.ends.left),
                right: describe(&f.ends.right),
                features: f.features.len(),
            })
        })
        .collect()
}

/// Ligate: assembles the slots in order into a new Sequence Window.
#[tauri::command]
pub fn ligate_construct(
    app: AppHandle,
    fragments: Vec<FragmentSpec>,
    circular: bool,
    name: String,
) -> Result<DocumentView, String> {
    let parts: Vec<Sequence> = fragments
        .iter()
        .map(|s| resolve(&app, s))
        .collect::<Result<_, _>>()?;
    // The original names the new window `Unsaved Ligation #n`.
    let name = if name.trim().is_empty() {
        let state = app.state::<AppState>();
        let mut counters = state.product_counters.lock().unwrap();
        counters.ligation += 1;
        format!("Ligation #{}", counters.ligation)
    } else {
        name
    };
    let mut product = ligate(&parts, circular, &name).map_err(|_| {
        "Serial Cloner cannot ligate fragments in the indicated order.\nPlease check order, orientation and end compatibility".to_owned()
    })?;
    rescan_junctions(&app, &mut product);
    Ok(open_product(&app, product))
}

/// The original's `LigUpdateAnnot` and `GateWayAnnotate` end with
/// `SetAnnotations(seq, from, to, false)` over every junction: features
/// recreated across a junction (a site rebuilt by compatible ends, a tag
/// completed by the insert) are found and merged like a scan would.
fn rescan_junctions(app: &AppHandle, product: &mut Sequence) {
    let junctions = sc_clone::ligate::junctions(product);
    if junctions.is_empty() {
        return;
    }
    let state = app.state::<AppState>();
    let collections = state.collections.lock().unwrap();
    let found = sc_features::scan_junctions(product, &collections, &junctions);
    sc_features::merge(&mut product.features, found);
}

// ---------------------------------------------------------------- PCR --

/// Run a PCR.
#[tauri::command]
pub fn run_pcr(
    app: AppHandle,
    id: DocId,
    forward: String,
    reverse: String,
    ta: bool,
) -> Result<DocumentView, String> {
    let product = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        sc_clone::pcr::pcr(&doc.seq, &forward, &reverse, ta).map_err(|e| e.to_string())?
    };
    Ok(open_product(&app, product))
}

/// Where a primer would anneal, for the PCR window's feedback.
#[derive(Debug, Clone, Serialize)]
pub struct PrimerCheck {
    /// 1-based start on the top strand.
    pub start: usize,
    /// 1-based end.
    pub end: usize,
    /// Un-annealed tail length.
    pub tail: usize,
    /// Tm of the annealed core (°C).
    pub tm: f64,
    /// The primer anneals to the strand the box did not expect (a forward
    /// primer on the bottom strand, a reverse one on the top): the region of
    /// interest runs on the file's reverse strand. Fine if both primers do.
    pub other_strand: bool,
}

/// Checks one primer against the matrix.
#[tauri::command]
pub fn check_primer(
    app: AppHandle,
    id: DocId,
    primer: String,
    reverse: bool,
) -> Result<PrimerCheck, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    let clean: Vec<u8> = primer
        .bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    let (a, other_strand) = sc_clone::pcr::anneal_either(
        &doc.seq.bases,
        doc.seq.topology.is_circular(),
        &clean,
        reverse,
        if reverse { "reverse" } else { "forward" },
    )
    .map_err(|e| e.to_string())?;
    let counts = sc_seq::stats::BaseCounts::of(&a.core);
    Ok(PrimerCheck {
        start: a.start + 1,
        end: a.end,
        tail: a.tail.len(),
        tm: sc_seq::stats::melting_temperature(&counts, sc_seq::stats::DEFAULT_NA_MOLAR),
        other_strand,
    })
}

/// True if the PCR window is open (the Sequence Window offers "use selection
/// as primer" only then).
#[tauri::command]
pub fn pcr_window_open(app: AppHandle) -> bool {
    app.get_webview_window("pcr").is_some()
}

// ------------------------------------------------------------ Adaptor --

/// Oligos of an adaptor or shRNA, with the in-memory sequence.
#[derive(Debug, Clone, Serialize)]
pub struct Oligos {
    /// Top oligo 5'→3'.
    pub top: String,
    /// Bottom oligo 5'→3'.
    pub bottom: String,
    /// The product window, when opened.
    pub view: Option<DocumentView>,
}

/// The cohesive extensions of an adaptor, signed as in the Extremities tab
/// (+ 5', − 3', 0 blunt).
#[derive(Debug, Clone, Deserialize)]
pub struct AdaptorEnds {
    /// Left extension length.
    pub left_length: i32,
    /// Left extension bases.
    pub left_bases: String,
    /// Right extension length.
    pub right_length: i32,
    /// Right extension bases.
    pub right_bases: String,
}

/// Create an Adaptor. With `open` the adaptor is also put in a Sequence
/// Window ("in memory").
#[tauri::command]
pub fn make_adaptor(
    app: AppHandle,
    name: String,
    core: String,
    ends: AdaptorEnds,
    open: bool,
) -> Result<Oligos, String> {
    let AdaptorEnds {
        left_length,
        left_bases,
        right_length,
        right_bases,
    } = ends;
    let ov = |len: i32, bases: &str| {
        let b: Vec<u8> = bases
            .bytes()
            .filter(|b| b.is_ascii_alphabetic())
            .map(|b| b.to_ascii_uppercase())
            .collect();
        if len == 0 || b.is_empty() {
            Overhang::BLUNT
        } else if len > 0 {
            Overhang::five_prime(&b)
        } else {
            Overhang::three_prime(&b)
        }
    };
    if core.bytes().filter(|b| b.is_ascii_alphabetic()).count() == 0 {
        return Err("enter the core sequence of the adaptor".into());
    }
    let name = if name.trim().is_empty() {
        "Adaptor".to_owned()
    } else {
        name
    };
    let a = sc_clone::adaptor::adaptor(
        &name,
        &core,
        ov(left_length, &left_bases),
        ov(right_length, &right_bases),
    );
    Ok(Oligos {
        top: String::from_utf8_lossy(&a.top_oligo).into_owned(),
        bottom: String::from_utf8_lossy(&a.bottom_oligo).into_owned(),
        view: open.then(|| open_product(&app, a.sequence)),
    })
}

// -------------------------------------------------------------- shRNA --

fn layouts_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    let d = app.path().app_config_dir().ok()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d.join("shrna-layouts.json"))
}

/// The stored shRNA layouts (the pSUPER one by default).
#[tauri::command]
pub fn shrna_layouts(app: AppHandle) -> Vec<sc_clone::shrna::Layout> {
    layouts_file(&app)
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| vec![sc_clone::shrna::Layout::psuper()])
}

/// Saves the layouts.
#[tauri::command]
pub fn save_shrna_layouts(app: AppHandle, layouts: Vec<sc_clone::shrna::Layout>) {
    if let Some(f) = layouts_file(&app) {
        let _ = std::fs::write(
            f,
            serde_json::to_string_pretty(&layouts).unwrap_or_default(),
        );
    }
}

/// Make shRNA.
#[tauri::command]
pub fn make_shrna(
    app: AppHandle,
    name: String,
    layout: sc_clone::shrna::Layout,
    core: String,
    mutations: Vec<usize>,
    open: bool,
) -> Result<Oligos, String> {
    let n = core.bytes().filter(|b| b.is_ascii_alphabetic()).count();
    if n < 15 {
        return Err("the siRNA core should be at least 15 nt (19–21 usually)".into());
    }
    let name = if name.trim().is_empty() {
        "shRNA".to_owned()
    } else {
        name
    };
    let s = sc_clone::shrna::shrna(&name, &layout, &core, &mutations);
    Ok(Oligos {
        top: String::from_utf8_lossy(&s.top_oligo).into_owned(),
        bottom: String::from_utf8_lossy(&s.bottom_oligo).into_owned(),
        view: open.then(|| open_product(&app, s.sequence)),
    })
}

// ------------------------------------------------------------ Gateway --

/// Gateway cloning: `reaction` is `bp`, `lr` or `hr` (homologous
/// recombination, with `min_homology` bases per arm).
#[tauri::command]
pub fn gateway(
    app: AppHandle,
    insert: DocId,
    vector: DocId,
    reaction: String,
    min_homology: Option<usize>,
    minimal_att: Option<bool>,
) -> Result<DocumentView, String> {
    let mut product = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let ins = &store.get(insert).ok_or("no such insert document")?.seq;
        let vec = &store.get(vector).ok_or("no such vector document")?.seq;
        match reaction.as_str() {
            "hr" => {
                sc_clone::gateway::homologous_recombination(ins, vec, min_homology.unwrap_or(20))
                    .map_err(|e| e.to_string())?
            }
            kind => {
                let collections = state.collections.lock().unwrap();
                let gateway = collections.iter().find(|c| c.name == "Gateway").ok_or("the Gateway Feature Collection is missing (Manage Features ▸ Restore built-in)")?;
                let r = if kind == "lr" {
                    sc_clone::gateway::Reaction::Lr
                } else {
                    sc_clone::gateway::Reaction::Bp
                };
                // "Use minimal att consensus" (the original's attDefCheckBox).
                let consensus = if minimal_att.unwrap_or(false) {
                    sc_clone::gateway::AttConsensus::Minimal
                } else {
                    sc_clone::gateway::AttConsensus::Full
                };
                sc_clone::gateway::recombine_with(r, ins, vec, gateway, consensus)
                    .map_err(|e| e.to_string())?
            }
        }
    };
    if reaction != "hr" {
        rescan_junctions(&app, &mut product);
    }
    Ok(open_product(&app, product))
}

// --------------------------------------------------------- Site Usage --

/// A Site Usage row, optionally with the count in a second sequence.
#[derive(Debug, Clone, Serialize)]
pub struct UsageRow {
    /// Enzyme.
    pub name: String,
    /// Site.
    pub site: String,
    /// Cuts in the sequence.
    pub count: usize,
    /// Cuts in the comparison sequence, if one was given.
    pub other: Option<usize>,
}

/// Site Usage (interactive / table) with an optional second sequence to
/// filter against ("unique here, absent there").
#[tauri::command]
pub fn site_usage(
    app: AppHandle,
    id: DocId,
    other: Option<DocId>,
) -> Result<Vec<UsageRow>, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    let other_doc = other.and_then(|o| store.get(o));
    let library = state.library();
    let hidden = state.hidden_enzymes.lock().unwrap();
    let mut rows: Vec<UsageRow> = library
        .enzymes
        .iter()
        .filter(|e| !hidden.contains(&e.name))
        .map(|e| UsageRow {
            name: e.name.clone(),
            site: e.display_site(),
            count: sc_enzyme::find_sites(&doc.seq.bases, doc.seq.topology, e).len(),
            other: other_doc.map(|d| sc_enzyme::find_sites(&d.seq.bases, d.seq.topology, e).len()),
        })
        .collect();
    rows.sort_by_key(|r| r.name.to_ascii_lowercase());
    Ok(rows)
}

// ----------------------------------------------------- Enzyme Library --

/// One enzyme of the library, for the Enzyme Library window.
#[derive(Debug, Clone, Serialize)]
pub struct LibraryEntry {
    /// Name.
    pub name: String,
    /// Site with the cut marked.
    pub site: String,
    /// Recognition site length.
    pub length: usize,
    /// Signed overhang (+ 5', − 3', 0 blunt).
    pub overhang: i32,
    /// Offset notation (type IIs).
    pub type_iis: bool,
    /// Added by the user.
    pub user: bool,
    /// Unchecked in the library (hidden from maps and lists).
    pub hidden: bool,
}

/// The whole library.
#[tauri::command]
pub fn library_entries(app: AppHandle) -> Vec<LibraryEntry> {
    let state = app.state::<AppState>();
    let library = state.library();
    let hidden = state.hidden_enzymes.lock().unwrap();
    let user = state.user_enzymes.lock().unwrap();
    library
        .enzymes
        .iter()
        .map(|e| LibraryEntry {
            name: e.name.clone(),
            site: e.display_site(),
            length: e.site.len(),
            overhang: e.overhang(),
            type_iis: e.type_iis,
            user: user.contains(&e.name),
            hidden: hidden.contains(&e.name),
        })
        .collect()
}

/// Enzymes recognising the same site.
#[tauri::command]
pub fn isoschizomers(app: AppHandle, name: String) -> Vec<String> {
    app.state::<AppState>()
        .library()
        .isoschizomers(&name)
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

/// Enzymes leaving compatible ends.
#[tauri::command]
pub fn compatible_ends(app: AppHandle, name: String) -> Vec<String> {
    app.state::<AppState>()
        .library()
        .compatible_ends(&name)
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

/// Adds a type II (`Name,site/with/cut,`) or IIs (`#Name,site,a,b,`)
/// enzyme, in `striderc` syntax, and remembers it across runs.
#[tauri::command]
pub fn add_enzyme(app: AppHandle, line: String) -> Result<LibraryEntry, String> {
    let enzyme = parse_entry(&line)
        .ok_or("not a valid striderc entry: Name,site/with/cut, or #Name,site,a,b,")?;
    let state = app.state::<AppState>();
    if state.library().get(&enzyme.name).is_some() {
        return Err(format!("{} is already in the library", enzyme.name));
    }
    let entry = LibraryEntry {
        name: enzyme.name.clone(),
        site: enzyme.display_site(),
        length: enzyme.site.len(),
        overhang: enzyme.overhang(),
        type_iis: enzyme.type_iis,
        user: true,
        hidden: false,
    };
    state.user_enzymes.lock().unwrap().push(enzyme.name.clone());
    state.library.write().unwrap().push(enzyme);
    persist_user_enzymes(&app);
    Ok(entry)
}

/// Removes a user-added enzyme.
#[tauri::command]
pub fn remove_enzyme(app: AppHandle, name: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    if !state.user_enzymes.lock().unwrap().contains(&name) {
        return Err("only enzymes you added can be removed; uncheck the others instead".into());
    }
    state.user_enzymes.lock().unwrap().retain(|n| n != &name);
    state.library.write().unwrap().remove(&name);
    persist_user_enzymes(&app);
    Ok(())
}

/// Checks or unchecks enzymes (unchecked ones are hidden from maps, Find
/// and Site Usage).
#[tauri::command]
pub fn set_enzymes_hidden(app: AppHandle, names: Vec<String>, hidden: bool) {
    let state = app.state::<AppState>();
    {
        let mut set = state.hidden_enzymes.lock().unwrap();
        for n in names {
            if hidden {
                set.insert(n);
            } else {
                set.remove(&n);
            }
        }
    }
    if let Some(f) = app
        .path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("hidden-enzymes.json"))
    {
        let set = state.hidden_enzymes.lock().unwrap();
        let mut v: Vec<&String> = set.iter().collect();
        v.sort();
        let _ = std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default());
    }
}

fn persist_user_enzymes(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(f) = app
        .path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("user-enzymes.striderc"))
    else {
        return;
    };
    let library = state.library();
    let user = state.user_enzymes.lock().unwrap();
    let text: String = library
        .enzymes
        .iter()
        .filter(|e| user.contains(&e.name))
        .map(|e| e.to_striderc() + "\n")
        .collect();
    let _ = std::fs::write(f, text);
}

/// Loads the user's enzymes and hidden set at startup.
pub fn load_user_settings(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(dir) = app.path().app_config_dir().ok() else {
        return;
    };
    if let Ok(text) = std::fs::read_to_string(dir.join("user-enzymes.striderc")) {
        let mut library = state.library.write().unwrap();
        let mut user = state.user_enzymes.lock().unwrap();
        for e in text.lines().filter_map(parse_entry) {
            if library.get(&e.name).is_none() {
                user.push(e.name.clone());
                library.push(e);
            }
        }
    }
    if let Ok(text) = std::fs::read_to_string(dir.join("hidden-enzymes.json"))
        && let Ok(v) = serde_json::from_str::<Vec<String>>(&text)
    {
        *state.hidden_enzymes.lock().unwrap() = v.into_iter().collect();
    }
}

// ----------------------------------------------------- Virtual Cutter --

/// The Virtual Cutter's result.
#[derive(Debug, Clone, Serialize)]
pub struct Digest {
    /// Fragment sizes, largest first.
    pub sizes: Vec<usize>,
    /// Migration of each fragment (0 = well, 1 = front).
    pub migrations: Vec<f64>,
    /// Ladder sizes.
    pub ladder: Vec<usize>,
    /// Ladder migrations.
    pub ladder_migrations: Vec<f64>,
    /// Which ladder bands are bright reference bands.
    pub ladder_bright: Vec<bool>,
    /// The ladder's name.
    pub ladder_name: String,
}

/// The ladders of the virtual gel, the original's four first.
#[tauri::command]
pub fn ladders() -> Vec<String> {
    sc_clone::cutter::ladders()
        .into_iter()
        .map(|l| l.name)
        .collect()
}

/// Cuts with the chosen enzymes; `ladder` names one of [`ladders`] (the
/// first when absent or unknown).
#[tauri::command]
pub fn virtual_cut(
    app: AppHandle,
    id: DocId,
    enzymes: Vec<String>,
    ladder: Option<String>,
) -> Result<Digest, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    let library = state.library();
    let chosen: Vec<&sc_enzyme::Enzyme> = enzymes.iter().filter_map(|n| library.get(n)).collect();
    let sizes = sc_clone::cutter::fragment_sizes(&doc.seq, &chosen);
    let mut all = sc_clone::cutter::ladders();
    let pick = ladder
        .and_then(|name| all.iter().position(|l| l.name == name))
        .unwrap_or(0);
    let ladder = all.swap_remove(pick);
    let (lo, hi) = ladder.range();
    let migrate = |s: usize| sc_clone::cutter::migration_between(s, lo, hi);
    Ok(Digest {
        migrations: sizes.iter().map(|&s| migrate(s)).collect(),
        sizes,
        ladder: ladder.bands.iter().map(|b| b.0).collect(),
        ladder_migrations: ladder.bands.iter().map(|b| migrate(b.0)).collect(),
        ladder_bright: ladder.bands.iter().map(|b| b.1).collect(),
        ladder_name: ladder.name,
    })
}

// --------------------------------------------------------- Silent Map --

/// A Silent Map row.
#[derive(Debug, Clone, Serialize)]
pub struct SilentRow {
    /// Enzyme.
    pub enzyme: String,
    /// 1-based position.
    pub position: usize,
    /// Base changes needed.
    pub changes: usize,
    /// The mutated stretch.
    pub mutated: String,
}

/// Silent Map of the reading frame `from..to` (1-based, inclusive; `from`
/// must be the first base of a codon).
#[tauri::command]
pub fn silent_map(
    app: AppHandle,
    id: DocId,
    from: usize,
    to: usize,
) -> Result<Vec<SilentRow>, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    if from == 0 || to < from {
        return Err("select the reading frame to scan first".into());
    }
    let library = state.library();
    let hidden = state.hidden_enzymes.lock().unwrap();
    Ok(
        sc_clone::silent::silent_map(&doc.seq, &library, from - 1, to)
            .into_iter()
            .filter(|s| !hidden.contains(&s.enzyme))
            .map(|s| SilentRow {
                enzyme: s.enzyme,
                position: s.position,
                changes: s.changes,
                mutated: s.mutated,
            })
            .collect(),
    )
}

/// Applies a silent mutation (replaces the site's span with `mutated`).
#[tauri::command]
pub fn apply_silent_mutation(
    app: AppHandle,
    id: DocId,
    position: usize,
    mutated: String,
) -> Result<DocumentView, String> {
    let from = position.saturating_sub(1);
    crate::commands::edit(
        app.clone(),
        id,
        crate::documents::EditOp::Replace {
            from,
            to: from + mutated.len(),
            text: mutated,
        },
    )?;
    view_of(&app, id)
}

// ------------------------------------------------------------ windows --

/// Opens one of the cloning windows (`construct`, `adaptor`, `pcr`, `shrna`,
/// `gateway`, `enzyme-library`) or a per-document one (`usage`, `cutter`).
#[tauri::command]
pub fn open_tool_window(
    app: AppHandle,
    which: String,
    id: Option<DocId>,
    from: Option<u32>,
    to: Option<u32>,
) {
    match (which.as_str(), id) {
        ("usage", Some(id)) => windows::open_usage(&app, id),
        ("cutter", Some(id)) => windows::open_cutter(&app, id),
        ("silent", Some(id)) => windows::open_silent(&app, id, from, to),
        (w, _) => windows::open_reference(&app, w),
    }
}

/// The enzyme chooser list for the front-most document, hidden enzymes
/// excluded (used by the Virtual Cutter).
#[tauri::command]
pub fn visible_enzymes(app: AppHandle, id: DocId) -> Result<Vec<EnzymeEntry>, String> {
    let hidden = app
        .state::<AppState>()
        .hidden_enzymes
        .lock()
        .unwrap()
        .clone();
    Ok(crate::commands::enzyme_list(app, id)?
        .into_iter()
        .filter(|e| !hidden.contains(&e.name))
        .collect())
}
