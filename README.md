# SerialClon.rs

<p align="center">
  <img src="assets/app-icon.svg" width="128" alt="SerialClon.rs">
</p>

**SerialClon.rs** is an open-source, maintained reimplementation of **Serial Cloner**, the DNA
cloning workbench by F. Perez (SerialBasics) whose last release was 2.6.1 in 2013. It is written in
Rust with a Tauri 2 shell and ships installers for **macOS, Windows and Ubuntu**.

Same windows, same keystrokes, same file formats, same numbers, on a stack that still builds.

- **Documentation and installation guide:** <https://luksgrin.xyz/SerialClon.rs/>
- **Downloads:** [GitHub Releases](https://github.com/luksgrin/SerialClon.rs/releases)
- **Changes:** [`CHANGELOG.md`](CHANGELOG.md)

## Why

Serial Cloner is abandonware. It was written in REALbasic, the macOS build is a 32-bit application,
the Linux build was shipped "as is", and there has been no release in over a decade. There are
plenty of alternatives, and none of them work the way Serial Cloner works.

This project does not try to build a better sequence editor. It rebuilds **the same one**: a thin
toolbar and independent windows you tile yourself, a Help window that narrates whatever your mouse
is over, editable from/to/length spinners, enzyme lists you scrub by typing, two-click fragment
selection on the map with the spacebar flipping the direction, draggable labels, and a comment
field that records how every fragment, PCR product and construct was made.

## What you get

| Area | Windows |
|---|---|
| Sequences | Sequence Window with Sequence, Comments, Features and Extremities tabs, Find drawer, Selection Information & Translation, five-level undo, autosave and crash recovery |
| Maps | Graphic Map (circular and linear, draggable labels, two-click selection, PNG export) and the text Sequence Map with multi-frame translation |
| Features | Scan against the original's built-in Collections (Generic, Yeast, Other, Tags & Cut, Gateway), Manage Features, import and export |
| Cloning | Build a Construct, Create an Adaptor, Run a PCR, Make shRNA, Gateway BP/LR and homologous recombination, Extract Fragment, Site Usage, Enzyme Library, Virtual Cutter, Silent Map |
| Alignment | Align two Sequences (local Smith-Waterman or NCBI BLAST2Seq) and Align Multiple Sequences on a pure-Rust MAFFT |
| Protein | Protein windows, reverse translation with codon usage tables, pI with the original's pK tables, 27 genetic codes, bisulfite conversion |
| Network | NCBI and ENA import, remote BLAST. Off unless you ask |
| Preferences | Every knob of the original, plus an Aqua Classic theme that reproduces the 2.6 look |

## Compatibility

Your existing files keep working.

| | |
|---|---|
| **Native** | `.xdna` (extended DNA Strider), read and written byte for byte, including features, colours and cohesive ends |
| **Read and write** | GenBank with the original's colour and end qualifiers, FASTA and multi-FASTA, SnapGene `.dna` with features and primers |
| **Read** | EMBL, raw text |
| **Export** | PNG maps, printable text maps |
| **Enzymes** | REBASE in the `striderc` format the original used, refreshable with `scripts/update-rebase.sh` |
| **Data** | The original's own Feature database, codon usage tables, pK tables and DNA ladders, recovered from the 2.6.1 application |

The science (Tm, molecular weight, pI, ORF search, digestion, feature scanning, cohesive ends)
is tested against what the original printed, saved or displayed. The comparison, format by format
and function by function, is in [`reference/docs/original-behaviour.md`](reference/docs/original-behaviour.md).

## Installation

Download the installer for your platform from the
[latest release](https://github.com/luksgrin/SerialClon.rs/releases/latest):

| Platform | File |
|---|---|
| macOS, Apple Silicon | `SerialClon.rs_<version>_aarch64.dmg` |
| macOS, Intel | `SerialClon.rs_<version>_x64.dmg` |
| macOS, either | `SerialClon.rs_<version>_universal.dmg` |
| Windows | `.msi` or the NSIS `.exe` |
| Ubuntu | `.deb` or `.AppImage` |

The installers are not code-signed, so the first launch needs a one-time unblocking step on macOS
and Windows. The [installation guide](https://luksgrin.xyz/SerialClon.rs/installation/) walks
through it. The app checks GitHub for a new version once a day; you install updates yourself.

## Building from source

Prerequisites: Rust stable with `rustfmt` and `clippy`, Node 22 or later, and on Ubuntu the Tauri 2
system packages (`libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libayatana-appindicator3-dev`,
`librsvg2-dev`, `patchelf`).

```sh
npm install
npm run tauri dev      # development build with hot reload
npm run tauri build    # installers under src-tauri/target/release/bundle/
```

```sh
cargo test --workspace                                   # the core crates
cargo clippy --workspace --all-targets -- -D warnings    # what CI enforces
cargo fmt --all --check
npm run check                                            # svelte-check
```

The repository layout, the architecture and the release procedure are described on the
[Development](https://luksgrin.xyz/SerialClon.rs/development/) and
[Design](https://luksgrin.xyz/SerialClon.rs/design/) pages of the documentation.

## Contributing

Issues and pull requests are welcome. Keep the science in the `sc-*` crates and testable without a
window, and back any claim of "same as the original" with a test against something the original
printed, saved or documented. The reference material is under [`reference/`](reference/README.md).

## Licence and attribution

SerialClon.rs is an **independent reimplementation**. It is not affiliated with, endorsed by, or
derived from the source code of Serial Cloner or SerialBasics. Serial Cloner is © F. Perez and was
distributed as freeware; its manuals and sample files are included under `reference/` as
compatibility and interface documentation, and its distributed binaries were examined to recover
the built-in data and to compare behaviour. See [`reference/README.md`](reference/README.md).

The restriction enzyme database is [REBASE](http://rebase.neb.com), © Dr. Richard J. Roberts,
New England Biolabs, free for academic use.

"Gateway" is a trademark of Thermo Fisher Scientific and is used descriptively to name the cloning
chemistry the corresponding window automates.

SerialClon.rs is licensed under either the [MIT licence](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option. Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual
licensed as above, without any additional terms or conditions.
