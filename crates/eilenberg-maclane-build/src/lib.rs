//! Certified Eilenberg--Mac Lane invariant collection builder.

pub mod enrich;
pub mod jsonl;
pub mod records;
pub mod source;
pub mod validate;

use records::EilenbergMacLaneRecord;

pub const SOURCE_CONTRACT: &str = "eilenberg-maclane/abelian-source-v1";
pub const INVARIANT_CONTRACT: &str = "eilenberg-maclane/invariants-v8";
pub const RECORD_VERSION: u32 = 8;
pub const NOETHER_REVISION: &str = "eebc33d4f37ed47af83c88dbc9f8886b254f72c6";
/// Historical producer before the public 0.3.2 source import.
pub const PRERELEASE_NOETHER_REVISION: &str = "4c92c269c59da02a4d32c833c2ee093407f8e22f";
/// Producer of the immutable first-edition artifacts.
pub const LEGACY_NOETHER_REVISION: &str = "cb2619170e52a11a6791b4d44df5f74771b7268d";
pub const SOURCE_BYTES: &[u8] = include_bytes!(
    "../../../collections/eilenberg-maclane/sources/finite-abelian-groups-through-32.json"
);

#[derive(Debug)]
pub struct EilenbergMacLaneCensus {
    pub maximum_finite_order: usize,
    pub maximum_delooping_degree: usize,
    pub exact_through_degree: usize,
    pub records: Vec<EilenbergMacLaneRecord>,
}
