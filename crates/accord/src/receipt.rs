//! The receipt: one check's report, as text for people and as JSON for
//! agents. Both carry the same record. The JSON schema is documented in
//! `docs/receipt.md`; it is versioned and may change until the first release.

mod diagnostics;
mod json;
mod review;
mod text;

pub use json::{json, rejection};
pub use text::text;

use crate::Standing;

/// The receipt schema's version.
pub const VERSION: u32 = 4;

impl Standing {
    /// The standing's name, as the receipt writes it.
    pub fn name(self) -> &'static str {
        match self {
            Standing::Verified => "verified",
            Standing::Incomplete => "incomplete",
            Standing::Failed => "failed",
            Standing::Unverified => "unverified",
        }
    }
}
