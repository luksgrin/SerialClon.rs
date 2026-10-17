# SerialClon.rs

[![CI](https://github.com/luksgrin/SerialClon.rs/actions/workflows/ci.yml/badge.svg)](https://github.com/luksgrin/SerialClon.rs/actions/workflows/ci.yml)
[![Docs](https://github.com/luksgrin/SerialClon.rs/actions/workflows/docs.yml/badge.svg)](https://luksgrin.xyz/SerialClon.rs/)
[![Release](https://img.shields.io/github/v/release/luksgrin/SerialClon.rs?include_prereleases&sort=semver)](https://github.com/luksgrin/SerialClon.rs/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A free, open-source DNA cloning workbench for macOS, Windows and Linux: a faithful, maintained
reimplementation of **Serial Cloner 2.6.1** (F. Perez, SerialBasics, 2013). Same windows, same
keystrokes, same file formats, same numbers, on a stack that still builds.

Built with Rust (the `sc-*` science crates) and Tauri 2, with a Svelte user interface.

**Documentation: <https://luksgrin.xyz/SerialClon.rs/>**

> **Coming from Serial Cloner?** Your `.xdna` files, your shortcuts and your habits carry over.
> See [Migration from Serial Cloner](https://luksgrin.xyz/SerialClon.rs/migration/) for what is
> identical, what is different and why.

## Features

- **Sequence Window**: Sequence, Comments, Features and Extremities tabs; from / to / length
  spinners that drive the selection; lock; linear or circular; Selection Information &
  Translation; five-level undo, autosave and crash recovery
- **Find drawer**: sequence, restriction site, ORF and peptide search on both strands, across the
  origin of circular sequences
- **Graphic Map**: circular and linear, feature tracks, unique / double / particular sites, Choose
  site, draggable labels, two-click fragment selection with spacebar CW / CCW, PNG export, print
- **Sequence Map**: the text restriction map with multi-frame translation, reproducing the
  original's printed map line for line
- **Features**: scan against the original's own built-in Collections (Generic, Yeast, Other, Tags
  & Cut, Gateway), Manage Features, import and export
- **Cloning**: Build a Construct, Create an Adaptor, Run a PCR, Make shRNA, Gateway BP / LR and
  homologous recombination, Extract Fragment, Site Usage, Enzyme Library, Virtual Cutter with a
  simulated gel, Silent Map
- **Alignment**: Align two Sequences (local Smith-Waterman or NCBI BLAST2Seq) and Align Multiple
  Sequences on a pure-Rust MAFFT
- **Protein**: protein windows, reverse translation with codon usage tables, pI with the
  original's pK tables, 27 genetic codes, bisulfite conversion
- **Files**: `.xdna` read and written byte for byte; GenBank with the original's colour and end
  qualifiers; FASTA and multi-FASTA; SnapGene `.dna` with features and primers; EMBL and raw text
- **Network**: NCBI and ENA import, remote BLAST. Off unless you ask
- **Preferences**: every knob of the original, plus an Aqua Classic theme that reproduces the
  2.6 look
- **Help window** that narrates whatever your mouse is over, and **keyboard shortcuts** for every
  command of the original

## Installation

### Pre-built binaries (recommended)

Download the installer from the [GitHub Releases](https://github.com/luksgrin/SerialClon.rs/releases)
page. Every release provides:

- **macOS**: `SerialClon.rs_<version>_aarch64.dmg` (Apple Silicon),
  `SerialClon.rs_<version>_x64.dmg` (Intel) and `SerialClon.rs_<version>_universal.dmg` (either)
- **Windows**: an `.msi` and a per-user `-setup.exe` installer
- **Linux**: a `.deb` (Debian / Ubuntu) and a portable `.AppImage`
- `SHA256SUMS.txt`

The builds are not code-signed:

**Note for macOS users:** if you see an error that the app is "damaged and can't be opened", this
is macOS Gatekeeper. After dragging the app to Applications, run
`xattr -cr "/Applications/SerialClon.rs.app"`, or right-click the app and choose "Open". See the
[Installation Guide](https://luksgrin.xyz/SerialClon.rs/installation/) for all three methods.

**Note for Windows users:** Windows SmartScreen may show a warning when installing. This is normal
for unsigned applications. Click "More info" then "Run anyway" to proceed.

**Linux:** `sudo apt install ./SerialClon.rs_<version>_amd64.deb`, or `chmod +x` the AppImage and
run it.

**Updates:** SerialClon.rs checks for new releases at most once a day (Preferences ▸ *Check for a
new version* checks now) and installs updates within the same major version in place on macOS,
Windows and the AppImage. A `.deb` install is updated with `apt` from the next release's file.

### Building from source

See [Development](https://luksgrin.xyz/SerialClon.rs/development/) for detailed instructions, or
follow the quick start under [Building](#building) below.

## Quick start

1. Launch SerialClon.rs. The Toolbar strip and the Help window appear; the Help window explains
   whatever your mouse is over
2. **File ▸ Open** (⌘O / Ctrl+O) an `.xdna`, GenBank, FASTA or SnapGene file, or **New** (⌘N)
   and paste a sequence
3. **Graphic Map** (⌘G): tick *Unique sites*; click a site box (green, begin), shift-click another
   (red, end); the spacebar flips the direction
4. **Extract Fragment** (⌘6) opens the fragment, cohesive ends and all, in a new window whose
   comment says how it was made
5. **Features ▸ Scan** finds promoters, origins, resistance genes and tags from the built-in
   Collections; **Sequence Map** (⌘M) prints the restriction map with translation
6. **Build a Construct** (⌘1) ligates fragments; **Run a PCR** (⌘3) amplifies with primers and
   tails; **Save** (⌘S) writes `.xdna`, or **Export** writes GenBank, FASTA or `.dna`

See [Getting started](https://luksgrin.xyz/SerialClon.rs/getting-started/) for everything else.

## Documentation

The full documentation is at **<https://luksgrin.xyz/SerialClon.rs/>**:

- [Installation](https://luksgrin.xyz/SerialClon.rs/installation/)
- [Getting started](https://luksgrin.xyz/SerialClon.rs/getting-started/) and
  [Keyboard shortcuts](https://luksgrin.xyz/SerialClon.rs/shortcuts/)
- The windows: [Sequence Window](https://luksgrin.xyz/SerialClon.rs/windows/sequence-window/),
  [Graphic Map](https://luksgrin.xyz/SerialClon.rs/windows/graphic-map/),
  [Sequence Map](https://luksgrin.xyz/SerialClon.rs/windows/sequence-map/),
  [Features](https://luksgrin.xyz/SerialClon.rs/windows/features/),
  [Cloning](https://luksgrin.xyz/SerialClon.rs/windows/cloning/),
  [Alignment](https://luksgrin.xyz/SerialClon.rs/windows/alignment/),
  [Protein](https://luksgrin.xyz/SerialClon.rs/windows/protein/),
  [Web Access](https://luksgrin.xyz/SerialClon.rs/windows/web-access/),
  [Preferences](https://luksgrin.xyz/SerialClon.rs/windows/preferences/)
- [File formats](https://luksgrin.xyz/SerialClon.rs/file-formats/) and
  [Data](https://luksgrin.xyz/SerialClon.rs/data/)
- [Migration from Serial Cloner](https://luksgrin.xyz/SerialClon.rs/migration/)
- [Development](https://luksgrin.xyz/SerialClon.rs/development/) and
  [Design](https://luksgrin.xyz/SerialClon.rs/design/)

The sources are in [`docs/`](docs/).

## Building

Requirements: Rust (stable, with `rustfmt` and `clippy`), Node ≥ 22,
`cargo install tauri-cli --version "^2" --locked`, and the platform's build tools (Xcode
command-line tools on macOS, Visual C++ Build Tools on Windows, `libwebkit2gtk-4.1-dev`,
`libxdo-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` and `patchelf` on Ubuntu).

```sh
git clone https://github.com/luksgrin/SerialClon.rs.git
cd SerialClon.rs
npm install
cargo test --workspace                                   # the science crates and the shell
cargo clippy --workspace --all-targets -- -D warnings    # what CI enforces
npm run check                                            # svelte-check
cargo tauri dev                                          # run the app with hot reload
cargo tauri build                                        # installers in src-tauri/target/release/bundle/
```

### Repository layout

```
crates/sc-seq        sequence model, translation, genetic codes, Tm / MW / pI, ORFs, bisulfite
crates/sc-format     .xdna, FASTA, GenBank / EMBL, SnapGene, raw text
crates/sc-enzyme     REBASE striderc, digestion, site usage, the text Sequence Map
crates/sc-features   Feature Collections and the scan engine
crates/sc-clone      extract, ligate, PCR, adaptor, shRNA, Gateway, virtual cutter, silent map
crates/sc-align      Smith-Waterman, consensus, differences; multiple alignment on mafft
crates/sc-net        NCBI E-utilities, ENA, BLAST URL API
src-tauri            Tauri 2 shell: documents, windows, menu, commands, preferences, autosave
src                  Svelte user interface, one entry per window
resources            RELibrary, the Feature Collections, codon usage tables, DNA ladders
reference            the original's sample files, golden files and recovered data
docs                 documentation (MkDocs)
```

Releases are cut with a manual GitHub Actions workflow after a version bump; see
[Development ▸ Releases](https://luksgrin.xyz/SerialClon.rs/development/#releases).

## Platform support

- **macOS**: 10.15 (Catalina) or later, Apple Silicon and Intel
- **Windows**: Windows 10 or later (x64)
- **Linux**: Ubuntu 22.04+, Debian 12+ (x86_64)

## Fidelity

The numbers and the files are checked against the original itself (details in
[`reference/docs/original-behaviour.md`](reference/docs/original-behaviour.md)):

- **Files**: every `.xdna` the original shipped round-trips byte for byte; fragments with every
  cohesive-end combination saved by the original read back with the same ends and are written
  back identically; GenBank exports carry the original's qualifiers
- **Digestion and maps**: the printed restriction map of pBluescript II SK(+), 51 enzymes, is
  reproduced position for position, and the Sequence Map line for line
- **Features**: the built-in database is the original's own, all 155 definitions; a scan gives
  the original's result on its sample files
- **Numbers**: Tm, molecular weight, pI (the original's five pK tables and algorithm) and the ORF
  list match what the original displays

Known differences: the Graphic Map is a new design with feature tracks and automatic label
budgeting; the PCR window takes whole primers in one field instead of a tail and an annealing
field; the GenBank header is standard-width; a few dialog texts are worded differently.

## Independent implementation

SerialClon.rs is an **independent reimplementation**. It is not affiliated with, endorsed by, or
derived from the source code of Serial Cloner or SerialBasics. Serial Cloner is © F. Perez and was
distributed as freeware; its sample files are included under `reference/` as format fixtures, and
its distributed binaries were examined to recover the built-in data and to compare behaviour. See [`reference/README.md`](reference/README.md).

The restriction enzyme database is [REBASE](http://rebase.neb.com), © Dr. Richard J. Roberts,
New England Biolabs, free for academic use. "Gateway" is a trademark of Thermo Fisher Scientific
and is used descriptively to name the cloning chemistry the corresponding window automates.

## License

MIT OR Apache-2.0, at your option, Copyright (c) 2026 Lucas Goiriz. See
[LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE). Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in the work by you shall be
dual licensed as above, without any additional terms or conditions.
