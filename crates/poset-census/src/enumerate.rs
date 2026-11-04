use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail};
use noether::{
    combin::{Poset, PosetCanonicalLabel},
    groups::{Permutation, PermutationGroup},
};

use crate::records::PosetKey;

/// Work performed while extending canonical parents to one new order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderlyLevelStats {
    pub order: usize,
    pub parent_count: usize,
    pub downset_count: u64,
    pub orbit_representative_count: u64,
    pub canonical_test_count: u64,
    pub accepted_count: usize,
}

/// Deterministic counters for a complete hereditary orderly enumeration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrderlyStats {
    pub levels: Vec<OrderlyLevelStats>,
    pub downset_count: u64,
    pub canonical_test_count: u64,
}

/// Deterministic work counters and canonical children for one parent.
#[derive(Debug)]
pub struct OrderlyParentOutput {
    pub downset_count: u64,
    pub orbit_representative_count: u64,
    pub canonical_test_count: u64,
    pub children: BTreeMap<PosetKey, PosetCanonicalLabel>,
}

/// Complete but deliberately redundant reference enumeration.
///
/// Every finite poset admits a linear extension. Relabeling by that extension
/// puts every strict comparison among pairs `(i, j)` with `i < j`. Enumerating
/// all subsets of those pairs therefore encounters every isomorphism class;
/// Noether closes, reduces, canonicalizes, and supplies the deduplication key.
pub fn reference_representatives(
    order: usize,
    candidate_cap: u64,
) -> Result<(u64, BTreeMap<PosetKey, PosetCanonicalLabel>)> {
    let order_u32 = u32::try_from(order).context("poset order does not fit a vertex id")?;
    let pairs = (0..order_u32)
        .flat_map(|lower| ((lower + 1)..order_u32).map(move |upper| (lower, upper)))
        .collect::<Vec<_>>();
    let Some(candidate_count) = 1_u128.checked_shl(pairs.len() as u32) else {
        bail!(
            "reference candidate space 2^{} does not fit u128",
            pairs.len()
        );
    };
    if candidate_count > u128::from(candidate_cap) {
        bail!(
            "reference enumeration needs {candidate_count} candidates, exceeding cap {candidate_cap}; raise --candidate-cap explicitly"
        );
    }
    let candidate_count = candidate_count as u64;
    let mut representatives = BTreeMap::new();

    for mask in 0..candidate_count {
        let relations = pairs
            .iter()
            .enumerate()
            .filter_map(|(bit, &pair)| ((mask >> bit) & 1 == 1).then_some(pair))
            .collect::<Vec<_>>();
        let candidate = Poset::new(order, &relations)?;
        let canonical = candidate.canonical_label()?;
        if !canonical.verify(&candidate) {
            bail!("Noether returned unverifiable canonical-label evidence");
        }
        let key = PosetKey::from_canonical_label(&canonical);
        if let std::collections::btree_map::Entry::Vacant(entry) = representatives.entry(key) {
            // The reference search generally starts from a noncanonical labeling,
            // so its automorphism group acts on that labeling. Recompute evidence
            // once for the canonical representative retained by the builder. The
            // production orderly generator does not need this extra search.
            let representative = canonical.poset().canonical_label()?;
            if !representative.verify(canonical.poset())
                || representative.poset() != canonical.poset()
            {
                bail!("Noether failed to certify a canonical reference representative");
            }
            entry.insert(representative);
        }
    }

    Ok((candidate_count, representatives))
}

/// Enumerate one canonical representative of every poset of `order`.
///
/// Starting from the empty poset, each level adjoins a new final element whose
/// strict lower set is a downset of a canonical parent. Only the numerically
/// least downset in each parent-automorphism orbit is extended. The child is
/// retained exactly when Noether confirms that its current labeling is the
/// canonical principal-downset labeling.
pub fn orderly_representatives(
    order: usize,
    candidate_cap: u64,
) -> Result<(OrderlyStats, BTreeMap<PosetKey, PosetCanonicalLabel>)> {
    if order > u64::BITS as usize {
        bail!(
            "orderly downset masks support target order at most {}",
            u64::BITS
        );
    }

    let empty = Poset::new(0, &[])?;
    let empty_label = empty.canonical_label()?;
    let mut parents = BTreeMap::from([(PosetKey::from_canonical_label(&empty_label), empty_label)]);
    let mut stats = OrderlyStats::default();

    for child_order in 1..=order {
        let parent_count = parents.len();
        let mut children = BTreeMap::new();
        let mut level = OrderlyLevelStats {
            order: child_order,
            parent_count,
            downset_count: 0,
            orbit_representative_count: 0,
            canonical_test_count: 0,
            accepted_count: 0,
        };

        for parent_label in parents.values() {
            let output =
                extend_orderly_parent(parent_label.poset(), parent_label.automorphism_group())?;
            level.downset_count = level
                .downset_count
                .checked_add(output.downset_count)
                .context("orderly downset counter overflow")?;
            stats.downset_count = stats
                .downset_count
                .checked_add(output.downset_count)
                .context("orderly downset counter overflow")?;
            if stats.downset_count > candidate_cap {
                bail!(
                    "orderly enumeration visited more than {candidate_cap} downsets; raise --candidate-cap explicitly"
                );
            }
            level.orbit_representative_count = level
                .orbit_representative_count
                .checked_add(output.orbit_representative_count)
                .context("orderly orbit-representative counter overflow")?;
            level.canonical_test_count = level
                .canonical_test_count
                .checked_add(output.canonical_test_count)
                .context("orderly canonical-test counter overflow")?;
            stats.canonical_test_count = stats
                .canonical_test_count
                .checked_add(output.canonical_test_count)
                .context("orderly canonical-test counter overflow")?;
            for (key, child_label) in output.children {
                if children.insert(key.clone(), child_label).is_some() {
                    bail!("orderly generation produced canonical child {key:?} twice");
                }
            }
        }

        level.accepted_count = children.len();
        stats.levels.push(level);
        parents = children;
    }

    Ok((stats, parents))
}

/// Extend one canonical parent by orbit-minimal downsets.
///
/// This is the deterministic unit assigned to a census shard. The supplied
/// group must be the complete automorphism group acting on the parent's
/// current (canonical) vertex ids.
pub fn extend_orderly_parent(
    parent: &Poset,
    automorphism_group: &PermutationGroup,
) -> Result<OrderlyParentOutput> {
    if parent.order() != automorphism_group.degree() {
        bail!("parent order and automorphism-group degree differ");
    }
    if parent.order() >= u64::BITS as usize {
        bail!(
            "orderly downset masks support parent order below {}",
            u64::BITS
        );
    }

    let principal_downsets = principal_downset_masks(parent)?;
    let mask_count = 1_u64
        .checked_shl(parent.order() as u32)
        .context("orderly parent downset mask space does not fit u64")?;
    let mut output = OrderlyParentOutput {
        downset_count: 0,
        orbit_representative_count: 0,
        canonical_test_count: 0,
        children: BTreeMap::new(),
    };
    for downset in 0..mask_count {
        if !is_downset(downset, &principal_downsets) {
            continue;
        }
        output.downset_count = output
            .downset_count
            .checked_add(1)
            .context("orderly downset counter overflow")?;
        if !is_orbit_minimum(downset, automorphism_group) {
            continue;
        }

        output.orbit_representative_count = output
            .orbit_representative_count
            .checked_add(1)
            .context("orderly orbit-representative counter overflow")?;
        output.canonical_test_count = output
            .canonical_test_count
            .checked_add(1)
            .context("orderly canonical-test counter overflow")?;
        let child = adjoin_maximum(parent, downset)?;
        let child_label = child.canonical_label()?;
        if !child_label.verify(&child) {
            bail!("Noether returned unverifiable canonical-label evidence");
        }
        if child_label.poset() != &child {
            continue;
        }

        let key = PosetKey::from_canonical_label(&child_label);
        if output.children.insert(key.clone(), child_label).is_some() {
            bail!("one orderly parent produced canonical child {key:?} twice");
        }
    }
    Ok(output)
}

fn principal_downset_masks(poset: &Poset) -> Result<Vec<u64>> {
    (0..poset.order())
        .map(|upper| {
            let mut mask = 0_u64;
            for lower in 0..poset.order() {
                if poset.leq(lower as u32, upper as u32)? {
                    mask |= 1_u64 << lower;
                }
            }
            Ok(mask)
        })
        .collect()
}

fn is_downset(subset: u64, principal_downsets: &[u64]) -> bool {
    principal_downsets
        .iter()
        .enumerate()
        .all(|(upper, downset)| subset & (1_u64 << upper) == 0 || downset & !subset == 0)
}

fn is_orbit_minimum(subset: u64, group: &PermutationGroup) -> bool {
    let mut seen = BTreeSet::from([subset]);
    let mut queue = VecDeque::from([subset]);
    while let Some(current) = queue.pop_front() {
        for generator in group.generators() {
            let image = permute_subset(current, generator);
            if image < subset {
                return false;
            }
            if seen.insert(image) {
                queue.push_back(image);
            }
        }
    }
    true
}

fn permute_subset(subset: u64, permutation: &Permutation) -> u64 {
    let mut image = 0_u64;
    for point in 0..permutation.degree() {
        if subset & (1_u64 << point) != 0 {
            image |= 1_u64 << permutation.apply(point as u32);
        }
    }
    image
}

fn adjoin_maximum(parent: &Poset, downset: u64) -> Result<Poset> {
    let maximum = u32::try_from(parent.order()).context("poset order does not fit a vertex id")?;
    let mut relations = parent
        .covers()
        .edges()
        .iter()
        .map(|edge| (edge.source, edge.target))
        .collect::<Vec<_>>();
    relations.extend(
        (0..parent.order())
            .filter(|&element| downset & (1_u64 << element) != 0)
            .map(|element| (element as u32, maximum)),
    );
    Ok(Poset::new(parent.order() + 1, &relations)?)
}
