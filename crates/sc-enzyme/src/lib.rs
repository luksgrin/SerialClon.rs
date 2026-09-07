//! Restriction enzymes for SerialClon.rs.
//!
//! Filled in at milestone **M1** (see `PLAN.md` §4): the enzyme model (type II
//! and IIs), digestion, site usage, isoschizomers and compatible ends.
//!
//! The database is REBASE's `striderc` flavour — the exact format Serial Cloner's
//! `RELibrary` used. It is a plain-text file with a banner followed by one
//! enzyme per line:
//!
//! ```text
//!  REBASE version 609                                             striderc.609
//!  ...
//! AatII,gacgt/c,
//! #AarI,cacctgc,4,8,
//! ```
//!
//! A leading `#` marks a type IIs enzyme whose two integers are the downstream
//! cut offsets. Only the banner is parsed here for now; it is what the About box
//! and the Enzyme Library window report as the database release.

/// Summary of a `striderc` file's banner and size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryInfo {
    /// The `NNN` of `REBASE version NNN`, when the banner is present.
    pub rebase_release: Option<u32>,
    /// Number of enzyme entry lines (`Name,site,` or `#Name,site,a,b,`).
    pub enzyme_count: usize,
}

/// Reads the banner and counts the entries of a `striderc`-formatted text.
///
/// This is deliberately tolerant: a file with no banner still yields a count,
/// and lines that are not entries (blank, banner, prose) are ignored.
pub fn library_info(text: &str) -> LibraryInfo {
    let mut rebase_release = None;
    let mut enzyme_count = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        if rebase_release.is_none()
            && let Some(rest) = trimmed.strip_prefix("REBASE version")
        {
            rebase_release = rest.split_whitespace().next().and_then(|n| n.parse().ok());
            continue;
        }
        if is_entry_line(trimmed) {
            enzyme_count += 1;
        }
    }
    LibraryInfo {
        rebase_release,
        enzyme_count,
    }
}

/// `Name,` or `#Name,` where `Name` is alphanumeric with dots (e.g. `R1.BceSIV`).
fn is_entry_line(line: &str) -> bool {
    let name = line.strip_prefix('#').unwrap_or(line);
    let Some((name, _)) = name.split_once(',') else {
        return false;
    };
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

    #[test]
    fn parses_a_minimal_library() {
        let info = library_info(
            " REBASE version 609   striderc.609\n\nAatII,gacgt/c,\n#AarI,cacctgc,4,8,\n",
        );
        assert_eq!(
            info,
            LibraryInfo {
                rebase_release: Some(609),
                enzyme_count: 2
            }
        );
    }

    #[test]
    fn ignores_prose_and_blank_lines() {
        let info = library_info(
            "Rich Roberts   Aug 27 2026\n\n   =-=-=\nCopyright (c) Dr. Richard J. Roberts, 2024.\n",
        );
        assert_eq!(info.rebase_release, None);
        assert_eq!(info.enzyme_count, 0);
    }

    #[test]
    fn shipped_library_has_a_banner_and_hundreds_of_enzymes() {
        let text = std::fs::read_to_string(format!("{REPO}/resources/RELibrary")).unwrap();
        let info = library_info(&text);
        assert!(
            info.rebase_release.is_some(),
            "resources/RELibrary lost its REBASE banner"
        );
        assert!(
            info.enzyme_count >= 500,
            "only {} enzymes",
            info.enzyme_count
        );
    }

    #[test]
    fn serial_cloner_2_6_1_shipped_rebase_211() {
        let text =
            std::fs::read_to_string(format!("{REPO}/reference/RELibrary-striderc.211")).unwrap();
        let info = library_info(&text);
        assert_eq!(info.rebase_release, Some(211));
        assert!(
            info.enzyme_count >= 500,
            "only {} enzymes",
            info.enzyme_count
        );
    }
}
