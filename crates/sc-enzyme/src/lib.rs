//! Restriction enzymes for SerialClon.rs.
//!
//! The database is REBASE's `striderc` flavour — the exact format Serial
//! Cloner's `RELibrary` used ([`library`]). [`Enzyme`] models both type II
//! enzymes (cut inside a palindromic site) and type IIs (cut at offsets
//! outside an asymmetric site); [`sites`] finds them on both strands, wrapping
//! the origin of circular sequences; [`digest`] turns cuts into fragments with
//! cohesive ends; [`usage`] is the Site Usage table.

pub mod digest;
pub mod library;
pub mod sites;
pub mod usage;

pub use digest::{Fragment, digest};
pub use library::{Enzyme, Library, LibraryInfo, library_info};
pub use sites::{Site, find_sites};
pub use usage::{SiteUsage, site_usage};
