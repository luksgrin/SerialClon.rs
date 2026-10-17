# Development

## Building

Prerequisites: Rust (stable, with `rustfmt` and `clippy`), Node 22+, and on Ubuntu the usual
Tauri 2 system packages (`libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libayatana-appindicator3-dev`,
`librsvg2-dev`, `patchelf`, …).

```sh
npm install
npm run tauri dev      # development: all windows, hot reload
npm run tauri build    # platform installers under src-tauri/target/release/bundle/

cargo test --workspace                                   # the core crates (network tests are #[ignore]d)
cargo clippy --workspace --all-targets -- -D warnings    # what CI enforces
cargo fmt --all --check
npm run check                                            # svelte-check
```

The documentation site:

```sh
python -m venv .venv && . .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve            # http://127.0.0.1:8000/
mkdocs build --strict
```

## Layout

```text
crates/
  sc-seq/                sequence model, alphabets, translation, genetic codes, Tm / MW / pI, ORFs,
                         codon usage, bisulfite
  sc-format/             .xdna (read + write), FASTA, GenBank/EMBL, raw text, format detection
  sc-enzyme/             REBASE striderc, sites, digestion, site usage, the text Sequence Map
  sc-features/           Feature Collections, the scan engine, import/export formats
  sc-clone/              extract, ligate, PCR, adaptor, shRNA, Gateway, virtual cutter, silent map
  sc-align/              Smith-Waterman, consensus, differences; multiple alignment on `mafft`
  sc-net/                NCBI E-utilities, ENA, BLAST URL API
src-tauri/               the Tauri shell: documents store, window manager, menu, commands, autosave
src/windows/<name>/      one Vite entry + Svelte component per Serial Cloner window
src/lib/                 the typed command API, the help bus, the shortcut map
resources/               RELibrary, the Feature Collections, the codon usage tables
reference/               the original's sample files, manuals, screenshots and 2012 enzyme database
docs/                    this site
```

The science lives in the seven `sc-*` crates and has no UI dependency; the shell is thin. A
sequence lives in the Rust `DocumentStore` under an id; windows hold ids, never copies, and get a
`doc:changed` event after every edit, so a map can never disagree with its Sequence Window.

## Tests and golden fixtures

The tests are what makes "the same" checkable. Besides unit tests, the suite pins itself to
the original's own output:

- every shipped `.xdna` round-trips byte for byte;
- digestion reproduces the original's printed restriction map of pBluescript II SK(+) (51
  enzymes, positions and strand arrows), and the Sequence Map renderer reproduces that map line
  for line;
- a scan with the built-in Collections reproduces, feature for feature, what the original's own
  scan produced on six of its sample files;
- `.xdna` files and GenBank exports with cohesive ends, saved by the original, read back with the
  same ends and are written back byte for byte;
- the ORF search lists the original's ORFs of pBR322 in its order;
- an end-to-end subcloning (EGFP cut out of pEGFP-N1 with BamHI and NotI and ligated into
  pBluescript II KS(+)) checks the product, its ends and its inherited Features;
- Tm and MW match the manual's Info-panel screenshot to the printed decimal; pI matches ExPASy;
- pBluescript KS(+), SK(+) and pBR322 align with MAFFT and agree where they should.

Network tests (NCBI, ENA, Kazusa, BLAST) are `#[ignore]`d so `cargo test` stays offline; run them
with `cargo test -- --ignored` when you want to.

## Releases

Every change is a branch and a pull request against `main`, verified with the commands above
plus a build of the bundle and a launch smoke test. Releases are manual: bump the version in
`src-tauri/tauri.conf.json`, the workspace `Cargo.toml` and `package.json` (they must agree),
merge, then run the **Installers** workflow from the Actions tab. It refuses a version that is
not newer than the latest release, builds the installers for every platform and drafts the
GitHub Release `v<version>`. The architecture and the product decisions are on the
[Design](design.md) page.

## Contributing

- Keep the science in the crates and testable without a window.
- Anything that claims to match the original needs a test against something the original
  printed, saved or documented — the reference material is in `reference/`.
- Never guess a sequence for the built-in Collections; cite the record.
- Every control gets a `data-help` text for the Help window, in the manual's voice.
- The most useful contribution from users of the original: files it saved, in any format, that
  SerialClon.rs reads differently.

Licence: MIT OR Apache-2.0.
