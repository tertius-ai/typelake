use std::{cmp::Ordering, fmt};

use anyhow::{Result, bail, ensure};
use noether::combin::PosetCanonicalLabel;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::RECORD_VERSION;

/// Canonical principal-downset sequence, encoded as unbounded decimal values.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PosetKey(Vec<String>);

impl PosetKey {
    pub fn new(entries: Vec<String>) -> Result<Self> {
        for entry in &entries {
            ensure!(
                entry == "0"
                    || (!entry.is_empty()
                        && !entry.starts_with('0')
                        && entry.bytes().all(|byte| byte.is_ascii_digit())),
                "invalid unsigned decimal PosetKey entry {entry:?}"
            );
        }
        Ok(Self(entries))
    }

    #[must_use]
    pub fn from_canonical_label(label: &PosetCanonicalLabel) -> Self {
        Self(
            label
                .incidence_sequence()
                .iter()
                .map(ToString::to_string)
                .collect(),
        )
    }

    #[must_use]
    pub fn entries(&self) -> &[String] {
        &self.0
    }

    #[must_use]
    pub fn order(&self) -> usize {
        self.0.len()
    }
}

impl<'de> Deserialize<'de> for PosetKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Self::new(Vec::<String>::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl PartialOrd for PosetKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PosetKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order().cmp(&other.order()).then_with(|| {
            self.0
                .iter()
                .zip(&other.0)
                .find_map(|(left, right)| {
                    let ordering = left.len().cmp(&right.len()).then_with(|| left.cmp(right));
                    (ordering != Ordering::Equal).then_some(ordering)
                })
                .unwrap_or(Ordering::Equal)
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PosetRecord {
    pub record_version: u32,
    pub key: PosetKey,
    pub order: usize,
    pub covers: Vec<[u32; 2]>,
    pub invariants: PosetInvariants,
    pub automorphism_group: AutomorphismGroupRecord,
    pub relations: PosetRelations,
}

impl PosetRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == RECORD_VERSION,
            "wrong record version"
        );
        ensure!(
            self.order == self.key.order(),
            "order does not match key length"
        );
        ensure!(
            self.automorphism_group.degree == self.order,
            "automorphism degree does not match order"
        );
        ensure!(
            self.invariants.strict_lower_counts.len() == self.order
                && self.invariants.strict_upper_counts.len() == self.order,
            "per-vertex invariant length does not match order"
        );
        ensure!(
            self.automorphism_group.generators_verified,
            "unverified generators"
        );
        ensure!(
            self.relations.has_core.reduction_verified,
            "unverified core reduction"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PosetInvariants {
    pub cover_count: usize,
    pub strict_comparable_pair_count: usize,
    pub strict_lower_counts: Vec<usize>,
    pub strict_upper_counts: Vec<usize>,
    pub minimal_count: usize,
    pub maximal_count: usize,
    pub component_count: usize,
    pub component_sizes: Vec<usize>,
    pub is_connected: bool,
    pub height: usize,
    pub width: usize,
    pub is_lattice: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AutomorphismGroupRecord {
    pub degree: usize,
    pub generators: Vec<Vec<u32>>,
    pub order: String,
    pub orbits: Vec<Vec<u32>>,
    pub is_transitive: bool,
    pub generators_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_group: Option<AbstractGroupRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AbstractGroupRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lmfdb_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_small_group: Option<GapSmallGroupRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GapSmallGroupRecord {
    pub order: String,
    pub id: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PosetRelations {
    pub has_core: CoreRelation,
    pub opposite: OppositeRelation,
    pub order_complex: OrderComplexRelation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoreRelation {
    pub target: PosetKey,
    pub core_size: usize,
    pub is_core: bool,
    pub deletions: Vec<BeatPointRecord>,
    pub retained_vertices: Vec<u32>,
    pub canonical_labeling: Vec<u32>,
    pub reduction_verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BeatPointRecord {
    pub element: u32,
    pub kind: BeatPointKindRecord,
    pub witness: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BeatPointKindRecord {
    Down,
    Up,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OppositeRelation {
    pub target: PosetKey,
    pub is_self_dual: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrderComplexRelation {
    pub target: ComplexKey,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ComplexKey(String);

impl ComplexKey {
    pub fn new(value: String) -> Result<Self> {
        let Some(digest) = value.strip_prefix("oc1-sha256:") else {
            bail!("invalid order-complex key prefix");
        };
        ensure!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "invalid order-complex SHA-256 digest"
        );
        Ok(Self(value))
    }

    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(format!("oc1-sha256:{}", hex::encode(digest)))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ComplexKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl fmt::Display for ComplexKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComplexRecord {
    pub record_version: u32,
    pub key: ComplexKey,
    pub vertex_count: usize,
    pub facets: Vec<Vec<u32>>,
    pub is_empty: bool,
    pub dimension: i64,
    pub f_vector: Vec<usize>,
    pub euler_characteristic: i64,
    pub integral_homology: Vec<IntegralHomologyRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntegralHomologyRecord {
    pub degree: usize,
    pub free_rank: usize,
    pub invariant_factors: Vec<String>,
}
