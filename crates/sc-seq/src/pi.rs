//! Isoelectric point (changelog 2.6: "Estimates Isoelectric point of protein
//! sequences", with "an option in the Preferences to select pK value tables").
//!
//! The net charge at a pH is the sum over ionisable groups of the
//! Henderson–Hasselbalch fractions; the pI is found by bisection where the
//! charge crosses zero. Six published pK sets are offered; the default is
//! Bjellqvist's, which is what ExPASy Compute pI/Mw uses, so results can be
//! checked against it (the tests do).

/// A set of pK values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PkTable {
    /// Name shown in Preferences.
    pub name: &'static str,
    /// N-terminal amino group (default; Bjellqvist refines it per residue).
    pub n_term: f64,
    /// C-terminal carboxyl.
    pub c_term: f64,
    /// Cys side chain.
    pub c: f64,
    /// Asp.
    pub d: f64,
    /// Glu.
    pub e: f64,
    /// His.
    pub h: f64,
    /// Lys.
    pub k: f64,
    /// Arg.
    pub r: f64,
    /// Tyr.
    pub y: f64,
}

/// Bjellqvist et al. 1993 — the ExPASy set (default).
pub const BJELLQVIST: PkTable = PkTable {
    name: "Bjellqvist (ExPASy)",
    n_term: 7.5,
    c_term: 3.55,
    c: 9.0,
    d: 4.05,
    e: 4.45,
    h: 5.98,
    k: 10.0,
    r: 12.0,
    y: 10.0,
};
/// EMBOSS `iep`.
pub const EMBOSS: PkTable = PkTable {
    name: "EMBOSS",
    n_term: 8.6,
    c_term: 3.6,
    c: 8.5,
    d: 3.9,
    e: 4.1,
    h: 6.5,
    k: 10.8,
    r: 12.5,
    y: 10.1,
};
/// Sillero & Ribeiro 1989.
pub const SILLERO: PkTable = PkTable {
    name: "Sillero",
    n_term: 8.2,
    c_term: 3.2,
    c: 9.0,
    d: 4.0,
    e: 4.5,
    h: 6.4,
    k: 10.4,
    r: 12.0,
    y: 10.0,
};
/// Solomon 1998 (Organic Chemistry).
pub const SOLOMON: PkTable = PkTable {
    name: "Solomon",
    n_term: 9.6,
    c_term: 2.4,
    c: 8.3,
    d: 3.9,
    e: 4.3,
    h: 6.0,
    k: 10.5,
    r: 12.5,
    y: 10.1,
};
/// Lehninger (Principles of Biochemistry).
pub const LEHNINGER: PkTable = PkTable {
    name: "Lehninger",
    n_term: 9.69,
    c_term: 2.34,
    c: 8.33,
    d: 3.86,
    e: 4.25,
    h: 6.0,
    k: 10.5,
    r: 12.4,
    y: 10.0,
};
/// Rodwell 1982.
pub const RODWELL: PkTable = PkTable {
    name: "Rodwell",
    n_term: 8.0,
    c_term: 3.1,
    c: 8.33,
    d: 3.68,
    e: 4.25,
    h: 6.0,
    k: 11.5,
    r: 11.5,
    y: 10.07,
};

/// Every table, default first.
pub const PK_TABLES: [PkTable; 6] = [BJELLQVIST, EMBOSS, SILLERO, SOLOMON, LEHNINGER, RODWELL];

/// Looks a table up by name; the default when unknown.
pub fn pk_table(name: &str) -> &'static PkTable {
    PK_TABLES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
        .unwrap_or(&BJELLQVIST)
}

/// Bjellqvist's N-terminal pK depends on the first residue (ExPASy does this).
fn n_term_pk(table: &PkTable, first: u8) -> f64 {
    if table.name != BJELLQVIST.name {
        return table.n_term;
    }
    match first.to_ascii_uppercase() {
        b'A' => 7.59,
        b'M' => 7.0,
        b'S' => 6.93,
        b'P' => 8.36,
        b'T' => 6.82,
        b'V' => 7.44,
        b'E' => 7.7,
        _ => 7.5,
    }
}

/// Net charge of a peptide at `ph`.
pub fn net_charge(peptide: &[u8], ph: f64, table: &PkTable) -> f64 {
    let residues: Vec<u8> = peptide
        .iter()
        .map(|b| b.to_ascii_uppercase())
        .filter(|b| b.is_ascii_alphabetic())
        .collect();
    if residues.is_empty() {
        return 0.0;
    }
    let positive = |pk: f64| 1.0 / (1.0 + 10f64.powf(ph - pk));
    let negative = |pk: f64| -1.0 / (1.0 + 10f64.powf(pk - ph));
    let mut q = positive(n_term_pk(table, residues[0])) + negative(table.c_term);
    for &r in &residues {
        q += match r {
            b'K' => positive(table.k),
            b'R' => positive(table.r),
            b'H' => positive(table.h),
            b'D' => negative(table.d),
            b'E' => negative(table.e),
            b'C' => negative(table.c),
            b'Y' => negative(table.y),
            _ => 0.0,
        };
    }
    q
}

/// The isoelectric point of a peptide (`*` and unknown letters ignored),
/// to two decimals like the original's Info panel. `None` for an empty
/// peptide.
pub fn isoelectric_point(peptide: &[u8], table: &PkTable) -> Option<f64> {
    if !peptide.iter().any(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let (mut lo, mut hi) = (0.0f64, 14.0f64);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if net_charge(peptide, mid, table) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(((lo + hi) / 2.0 * 100.0).round() / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_expasy_for_the_twenty_amino_acids() {
        // ExPASy Compute pI/Mw, 2026-09-08: ACDEFGHIKLMNPQRSTVWY → pI 6.78.
        let pi = isoelectric_point(b"ACDEFGHIKLMNPQRSTVWY", &BJELLQVIST).unwrap();
        assert!((pi - 6.78).abs() <= 0.02, "{pi}");
    }

    #[test]
    fn matches_expasy_for_serum_albumin() {
        // ExPASy, same day: bovine serum albumin precursor → pI 5.92.
        let bsa = b"MKWVTFISLLFLFSSAYSRGVFRRDAHKSEVAHRFKDLGEENFKALVLIAFAQYLQQCPFEDHVKLVNEVTEFAKTCVADESAENCDKSLHTLFGDKLCTVATLRETYGEMADCCAKQEPERNECFLQHKDDNPNLPRLVRPEVDVMCTAFHDNEETFLKKYLYEIARRHPYFYAPELLFFAKRYKAAFTECCQAADKAACLLPKLDELRDEGKASSAKQRLKCASLQKFGERAFKAWAVARLSQRFPKAEFAEVSKLVTDLTKVHTECCHGDLLECADDRADLAKYICENQDSISSKLKECCEKPLLEKSHCIAEVENDEMPADLPSLAADFVESKDVCKNYAEAKDVFLGMFLYEYARRHPDYSVVLLLRLAKTYETTLEKCCAAADPHECYAKVFDEFKPLVEEPQNLIKQNCELFEQLGEYKFQNALLVRYTKKVPQVSTPTLVEVSRNLGKVGSKCCKHPEAKRMPCAEDYLSVVLNQLCVLHEKTPVSDRVTKCCTESLVNRRPCFSALEVDETYVPKEFNAETFTFHADICTLSEKERQIKKQTALVELVKHKPKATKEQLKAVMDDFAAFVEKCCKADDKETCFAEEGKKLVAASQAALGL";
        let pi = isoelectric_point(bsa, &BJELLQVIST).unwrap();
        assert!((pi - 5.92).abs() <= 0.02, "{pi}");
    }

    #[test]
    fn tables_and_edge_cases() {
        assert_eq!(pk_table("emboss").name, "EMBOSS");
        assert_eq!(pk_table("nope").name, BJELLQVIST.name);
        assert!(isoelectric_point(b"", &EMBOSS).is_none());
        assert!(isoelectric_point(b"***", &EMBOSS).is_none());
        for t in &PK_TABLES {
            assert!(
                isoelectric_point(b"KKKRRR", t).unwrap() > 10.0,
                "{}",
                t.name
            );
            assert!(isoelectric_point(b"DDDEEE", t).unwrap() < 4.5, "{}", t.name);
        }
    }
}
