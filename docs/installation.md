# Installation

SerialClon.rs ships as installers for macOS, Windows and Ubuntu, built from the
[GitHub releases](https://github.com/luksgrin/SerialClon.rs/releases). The installers are
**not code-signed** — the original was in the same situation for most of its life — so each OS
shows a warning the first time. The steps below get you past it once; after that the app opens
like any other.

## macOS

1. Download the `.dmg` for your Mac: `aarch64` for Apple Silicon (M1 and later), `x86_64` for
   Intel, or the `universal` build if you are not sure.
2. Open the disk image and drag **SerialClon** to `Applications`.
3. The first launch is blocked by Gatekeeper ("SerialClon is damaged and can't be opened" or
   "cannot be opened because the developer cannot be verified"). Use one of these:

=== "Terminal (recommended)"

    Remove the quarantine attribute once:

    ```sh
    xattr -cr /Applications/SerialClon.app
    ```

=== "Right-click ▸ Open"

    In Finder, right-click (or Control-click) `SerialClon.app`, choose **Open**, then click
    **Open** in the dialog. macOS remembers the choice.

=== "System Settings"

    Launch once, then go to **System Settings ▸ Privacy & Security**, scroll to the message about
    SerialClon and click **Open Anyway**.

## Windows

1. Download the `.msi` installer (or the NSIS `-setup.exe`, which installs per user without
   administrator rights).
2. Run it. SmartScreen shows "Windows protected your PC": click **More info**, then
   **Run anyway**.
3. Alternatively, right-click the downloaded file, open **Properties**, tick **Unblock** and
   click OK before running it.

## Linux (Ubuntu / Debian)

=== "DEB package (recommended)"

    ```sh
    sudo dpkg -i SerialClon_*_amd64.deb
    sudo apt-get install -f   # only if dpkg reports missing dependencies
    ```

    The package depends on the system WebKitGTK (`libwebkit2gtk-4.1`), which Ubuntu 22.04 and
    later provide.

=== "AppImage"

    ```sh
    chmod +x SerialClon_*_amd64.AppImage
    ./SerialClon_*_amd64.AppImage
    ```

    The AppImage needs `libfuse2` on distributions that do not ship it.

## Verifying the installation

Launch SerialClon. Two windows appear: the **Toolbar** strip at the top left and the **Help**
window at the bottom right. Open any `.xdna`, FASTA or GenBank file (File ▸ Open…, or the
Toolbar's Open button) and a Sequence Window opens for it. The version is in Window ▸ About
SerialClon….

## File associations

The installers register `.xdna` (DNA Strider / Serial Cloner) so that double-clicking a file
opens it in SerialClon. Files given on the command line are opened at startup as well.

## Updates

SerialClon checks for a newer release on GitHub when it starts (it only reads the releases
page; nothing about you is sent) and tells you in the Help window. Install the new version over
the old one; your settings and data are kept.

## Where your data lives

Everything the app writes goes to the per-user directories of the OS; nothing is written next to
the application.

| What | macOS | Windows | Linux |
|---|---|---|---|
| Settings and lists | `~/Library/Application Support/io.github.luksgrin.serialclon/` | `%APPDATA%\io.github.luksgrin.serialclon\` | `~/.config/io.github.luksgrin.serialclon/` |
| Data (autosave, Collections, codon tables) | same directory | `%APPDATA%\io.github.luksgrin.serialclon\` | `~/.local/share/io.github.luksgrin.serialclon/` |

Files you may want to know about:

| File | Content |
|---|---|
| `recent.json` | File ▸ Open Recent |
| `prefs.json` | the Preferences (colours, sizes, defaults, theme, proxy…); `--reset-prefs` at launch recreates it |
| `settings.json` | the genetic code, pK table and codon-usage table chosen in the Protein menu |
| `update-check.json` | the day of the last update check |
| `shrna-layouts.json` | Make shRNA layouts |
| `user-enzymes.striderc` | enzymes you added in the Enzyme Library (REBASE itself is compiled in) |
| `hidden-enzymes.json` | enzymes you unchecked in the Enzyme Library |
| `autosave/` | unsaved sequences, written every 30 s and offered back after a crash |
| `features/` | the five built-in Feature Collections (seeded on first run) and your own |
| `codon-usage/` | codon usage tables you imported |

Deleting any of these restores the shipped default.

## Licence

SerialClon.rs is released under **MIT OR Apache-2.0**, at your option. The restriction enzyme
database is REBASE, © Dr. Richard J. Roberts, New England Biolabs, free for academic use. The
multiple alignment engine is [rust-MAFFT](https://github.com/luksgrin/rust-MAFFT) (MIT), a port
of MAFFT (BSD-3-Clause). "Gateway" is a trademark of Thermo Fisher Scientific and is used
descriptively.
