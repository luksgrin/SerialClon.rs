//! Isoelectric point (changelog 2.6: "Estimates Isoelectric point of protein
//! sequences", with "an option in the Preferences to select pK value tables").
//!
//! Done the way the original does it (`GeneralMethods.FindIsoelectricPoint`
//! and `Init_IsolectricPointTables`, see
//! `reference/docs/original-behaviour.md`): the net charge at a pH is the
//! sum over ionisable groups of the Henderson–Hasselbalch fractions,
//! evaluated on a 0.01-pH grid from 0.00 to 14.00; starting at pH 7.00 the
//! grid is walked towards the sign change and the first step where the
//! charge is no longer on the starting side is the pI. The five pK sets are
//! the original's, in its order, with its values; EMBOSS is its default.

/// A set of pK values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PkTable {
    /// Name shown in Preferences.
    pub name: &'static str,
    /// N-terminal amino group.
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

/// EMBOSS `iep` (the original's default).
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
/// The set the original labels "Wikipedia".
pub const WIKIPEDIA: PkTable = PkTable {
    name: "Wikipedia",
    n_term: 8.2,
    c_term: 3.65,
    c: 8.18,
    d: 3.9,
    e: 4.07,
    h: 6.04,
    k: 10.54,
    r: 12.48,
    y: 10.46,
};
/// DTASelect.
pub const DTASELECT: PkTable = PkTable {
    name: "DTASelect",
    n_term: 8.0,
    c_term: 3.1,
    c: 8.5,
    d: 4.4,
    e: 4.4,
    h: 6.5,
    k: 10.0,
    r: 12.0,
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

/// Every table, in the original's order; the first is the default.
pub const PK_TABLES: [PkTable; 5] = [EMBOSS, WIKIPEDIA, DTASELECT, RODWELL, SILLERO];

/// The default table.
pub const DEFAULT: PkTable = EMBOSS;

/// Looks a table up by name; the default when unknown (a 1.0.0 settings file
/// naming one of the tables that are no longer offered gets the default).
pub fn pk_table(name: &str) -> &'static PkTable {
    PK_TABLES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
        .unwrap_or(&PK_TABLES[0])
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
    let count = |aa: u8| residues.iter().filter(|&&r| r == aa).count() as f64;
    positive(table.n_term)
        + negative(table.c_term)
        + count(b'K') * positive(table.k)
        + count(b'R') * positive(table.r)
        + count(b'H') * positive(table.h)
        + count(b'D') * negative(table.d)
        + count(b'E') * negative(table.e)
        + count(b'C') * negative(table.c)
        + count(b'Y') * negative(table.y)
}

/// The isoelectric point of a peptide (`*` and unknown letters ignored),
/// to two decimals like the original's Info panel. `None` for an empty
/// peptide.
///
/// The original's walk: the charge is computed in single precision on the
/// 0.01 grid; at pH 7.00 a charge of exactly zero gives 7.0, a negative one
/// walks down to the first grid point with a non-negative charge, a
/// positive one walks up to the first grid point with a non-positive
/// charge (or stops at the end of the grid).
pub fn isoelectric_point(peptide: &[u8], table: &PkTable) -> Option<f64> {
    if !peptide.iter().any(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let charge = |i: i32| net_charge(peptide, f64::from(i) / 100.0, table) as f32;
    let at7 = charge(700);
    let step = if at7 == 0.0 {
        700
    } else if at7 < 0.0 {
        let mut i = 699;
        while i > 0 && charge(i) < 0.0 {
            i -= 1;
        }
        i
    } else {
        let mut i = 701;
        while i < 1400 && charge(i) > 0.0 {
            i += 1;
        }
        i
    };
    Some(f64::from(step) / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_the_grid_to_the_sign_change() {
        // Lysine alone: N-term 8.6, C-term 3.6, side chain 10.8 (EMBOSS).
        let pi = isoelectric_point(b"K", &EMBOSS).unwrap();
        assert!(net_charge(b"K", pi - 0.01, &EMBOSS) > 0.0);
        assert!(net_charge(b"K", pi, &EMBOSS) <= 0.0);
        assert!((9.6..=9.8).contains(&pi), "{pi}");
        // Aspartate alone is acidic: the walk goes down from 7.
        let pi = isoelectric_point(b"D", &EMBOSS).unwrap();
        assert!(net_charge(b"D", pi, &EMBOSS) >= 0.0);
        assert!(net_charge(b"D", pi + 0.01, &EMBOSS) < 0.0);
        assert!((3.6..=3.9).contains(&pi), "{pi}");
    }

    #[test]
    fn twenty_amino_acids_with_each_table() {
        // The grid is 0.01 wide, so every table lands within 0.01 of the
        // true root of its own charge function.
        for t in &PK_TABLES {
            let pi = isoelectric_point(b"ACDEFGHIKLMNPQRSTVWY", t).unwrap();
            assert!(
                net_charge(b"ACDEFGHIKLMNPQRSTVWY", pi - 0.01, t) as f32 > 0.0,
                "{}",
                t.name
            );
            assert!(
                net_charge(b"ACDEFGHIKLMNPQRSTVWY", pi + 0.01, t) as f32 <= 0.0,
                "{}",
                t.name
            );
        }
        // ExPASy (Bjellqvist) says 6.78 for this peptide; EMBOSS's set
        // gives a value in the same region.
        let pi = isoelectric_point(b"ACDEFGHIKLMNPQRSTVWY", &EMBOSS).unwrap();
        assert!((6.5..=7.5).contains(&pi), "{pi}");
    }

    #[test]
    fn ignores_stops_and_unknown_letters() {
        let a = isoelectric_point(b"MKKR*", &EMBOSS);
        let b = isoelectric_point(b"MKKR", &EMBOSS);
        assert_eq!(a, b);
        assert_eq!(isoelectric_point(b"***", &EMBOSS), None);
        assert_eq!(isoelectric_point(b"", &EMBOSS), None);
    }

    #[test]
    fn tables_are_looked_up_by_name_with_the_default_as_fallback() {
        assert_eq!(pk_table("emboss").name, "EMBOSS");
        assert_eq!(pk_table("DTASelect").name, "DTASelect");
        assert_eq!(pk_table("Bjellqvist (ExPASy)").name, DEFAULT.name);
        assert_eq!(PK_TABLES[0].name, DEFAULT.name);
    }
}
