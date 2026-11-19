//! Certified finite-group cohomology collection builder.

pub mod enrich;
pub mod integral;
pub mod integral_jsonl;
pub mod jsonl;
pub mod records;
pub mod source;
pub mod validate;

use records::FiniteGroupIntegralCohomologyRecord;
use records::FiniteGroupModularCohomologyRecord;

pub const SOURCE_CONTRACT: &str = "finite-group-sources/smallgroups-v1";
pub const MODULAR_CONTRACT: &str = "finite-group-cohomology/modular-v7";
pub const RECORD_VERSION: u32 = 7;
pub const NOETHER_REVISION: &str = "eebc33d4f37ed47af83c88dbc9f8886b254f72c6";
/// Historical producer before the public 0.3.2 source import.
pub const PRERELEASE_NOETHER_REVISION: &str = "4c92c269c59da02a4d32c833c2ee093407f8e22f";
/// Producer of the immutable first-edition artifacts.
pub const LEGACY_NOETHER_REVISION: &str = "76248daf94ce49a9d297fc6e29f5f9cbda775b27";
pub const INTEGRAL_CONTRACT: &str = "finite-group-cohomology/integral-v4";
pub const INTEGRAL_NOETHER_REVISION: &str = "eebc33d4f37ed47af83c88dbc9f8886b254f72c6";
/// Producer of the immutable first-edition artifacts.
pub const LEGACY_INTEGRAL_NOETHER_REVISION: &str = "2eb46dcf7f6b7e35f28d7a356e67755a001e6880";
pub const SOURCE_BYTES: &[u8] =
    include_bytes!("../../../collections/finite-group-cohomology/sources/groups-through-32.json");

#[derive(Debug)]
pub struct ModularCensus {
    pub maximum_order: usize,
    pub initial_proof_degree: usize,
    pub maximum_proof_degree: usize,
    pub records: Vec<FiniteGroupModularCohomologyRecord>,
}

#[derive(Debug)]
pub struct IntegralCensus {
    pub maximum_order: usize,
    pub additive_through_degree: usize,
    pub bar_entry_limit: usize,
    pub source_group_count: usize,
    pub records: Vec<FiniteGroupIntegralCohomologyRecord>,
}
