//! Deterministic mathematical replay of boundary and interior rows of Volume I.
//!
//! Full file integrity is checked separately by scripts/volume-one.py. This
//! command reports its sample size and does not claim exhaustive oracle parity.

use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use clap::Parser;
use noether::{combin::Poset, numthy::FiniteField};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

#[derive(Parser)]
struct Options {
    #[arg(long)]
    data_root: PathBuf,
    #[arg(long)]
    report: PathBuf,
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    BufReader::new(File::open(path)?)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect()
}

fn indices(length: usize) -> Vec<usize> {
    let mut selected = vec![0, length / 2, length.saturating_sub(1)];
    selected.sort_unstable();
    selected.dedup();
    selected.retain(|index| *index < length);
    selected
}

fn same(old: &impl Serialize, new: &impl Serialize, context: &str) -> Result<()> {
    ensure!(
        serde_json::to_value(old)? == serde_json::to_value(new)?,
        "{context}: replay differs"
    );
    Ok(())
}

fn main() -> Result<()> {
    let options = Options::parse();
    let mut counts = serde_json::Map::new();
    let poset_counts = [1, 1, 2, 5, 16, 63, 318, 2045, 16999, 183231, 2567284];
    let mut count = 0;
    for (order, length) in poset_counts.into_iter().enumerate() {
        let path = options.data_root.join(format!(
            "finite-posets/census/record-version=1/order={order}/posets.jsonl"
        ));
        let selected = indices(length);
        for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
            let line = line?;
            if !selected.contains(&index) {
                continue;
            }
            let old: poset_census::records::PosetRecord = serde_json::from_str(&line)?;
            let edges = old
                .covers
                .iter()
                .map(|edge| (edge[0], edge[1]))
                .collect::<Vec<_>>();
            let poset = Poset::new(old.order, &edges)?;
            let (new, _) = poset_census::enrich::enrich_poset(&poset)?;
            same(&old, &new, &format!("poset order {order}, row {index}"))?;
            count += 1;
        }
    }
    counts.insert("finite-posets".to_owned(), json!(count));
    eprintln!("replayed {count} posets");

    count = 0;
    for size in 0..=11 {
        let path = options.data_root.join(format!(
            "strong-steiner-categories/census/record-version=1/atomic-size={size}/categories.jsonl"
        ));
        let rows: Vec<steiner_census::records::SteinerCategoryRecord> = read(&path)?;
        for index in indices(rows.len()) {
            let old = &rows[index];
            let identity = steiner_census::records::SteinerIdentityRecord {
                record_version: old.record_version,
                key: old.key.clone(),
                atom_counts: old.atom_counts.clone(),
                boundaries: old.boundaries.clone(),
            };
            let canonical = identity.reconstruct()?.canonical_label()?;
            let new = steiner_census::enrich::enrich_category(&canonical, 2)?;
            same(
                old,
                &new.category,
                &format!("Steiner size {size}, row {index}"),
            )?;
            // The homology presentation is replayed as well as the light record.
            let path = options.data_root.join(format!("strong-steiner-categories/census/record-version=1/atomic-size={size}/categorical-homology.jsonl.zst"));
            let reader = BufReader::new(zstd::stream::read::Decoder::new(File::open(path)?)?);
            let old_homology = reader
                .lines()
                .map(|line| Ok(serde_json::from_str::<Value>(&line?)?))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .find(|row| row["category"] == serde_json::to_value(&old.key).unwrap())
                .context("missing categorical homology source")?;
            same(
                &old_homology,
                &new.categorical_homology,
                "Steiner categorical homology",
            )?;
            count += 1;
        }
    }
    let identities = steiner_census::jsonl::load_identity_shell(
        &options.data_root.join("strong-steiner-categories"),
        12,
    )?;
    for index in indices(identities.records.len()) {
        let old = &identities.records[index];
        let canonical = old.reconstruct()?.canonical_label()?;
        same(
            old,
            &steiner_census::enrich::identity_record(&canonical)?,
            "Steiner size 12 identity",
        )?;
        count += 1;
    }
    counts.insert("strong-steiner-categories".to_owned(), json!(count));
    eprintln!("replayed {count} Steiner categories");

    count = 0;
    for vertices in 0..=6 {
        use simplicial_complex_census::{enrich, jsonl, records};
        let identities = jsonl::load_identity_shell(&options.data_root, vertices)?;
        let topology = jsonl::load_topology_shell(&options.data_root, vertices)?;
        let f2 = jsonl::load_cohomology_shell(&options.data_root, vertices, 2)?;
        let f3 = jsonl::load_cohomology_shell(&options.data_root, vertices, 3)?;
        for index in indices(identities.len()) {
            let identity = &identities[index];
            same(
                identity,
                &records::SimplicialComplexIdentityRecord::from_canonical(
                    &identity.key.reconstruct(),
                )?,
                "simplicial complex identity",
            )?;
            same(
                &topology[index],
                &enrich::topology_record(identity)?,
                "simplicial complex topology",
            )?;
            for (prime, rows) in [(2, &f2), (3, &f3)] {
                same(
                    &rows[index],
                    &enrich::cohomology_record(
                        identity,
                        &topology[index],
                        &FiniteField::prime(prime)?,
                    )?,
                    "simplicial complex cohomology",
                )?;
            }
            count += 1;
        }
    }
    counts.insert("finite-simplicial-complexes".to_owned(), json!(count));
    eprintln!("replayed {count} simplicial complexes");

    count = 0;
    for (total, dimension) in (0..=6).map(|n| (n, 3)).chain([(7, 2)]) {
        use simplicial_set_census::{enrich, jsonl, records};
        let identities = jsonl::load_identity_shell(&options.data_root, total, dimension)?;
        let topology = jsonl::load_topology_shell(&options.data_root, total, dimension)?;
        let homotopy = jsonl::load_homotopy_shell(&options.data_root, total, dimension)?;
        let f2 = jsonl::load_cohomology_shell(&options.data_root, total, dimension, 2)?;
        let f3 = jsonl::load_cohomology_shell(&options.data_root, total, dimension, 3)?;
        for index in indices(identities.len()) {
            let identity = &identities[index];
            same(
                identity,
                &records::SimplicialSetIdentityRecord::from_canonical(
                    &identity.key.reconstruct()?,
                )?,
                "simplicial set identity",
            )?;
            same(
                &topology[index],
                &enrich::topology_record(identity)?,
                "simplicial set topology",
            )?;
            same(
                &homotopy[index],
                &enrich::homotopy_record(identity)?,
                "simplicial set homotopy",
            )?;
            for (prime, rows) in [(2, &f2), (3, &f3)] {
                same(
                    &rows[index],
                    &enrich::cohomology_record(identity, &FiniteField::prime(prime)?)?,
                    "simplicial set cohomology",
                )?;
            }
            count += 1;
        }
    }
    counts.insert("finite-simplicial-sets".to_owned(), json!(count));
    fs::write(
        options.report,
        serde_json::to_vec_pretty(&json!({
            "status": "passed", "noether_version": noether::VERSION,
            "noether_revision": eilenberg_maclane_build::NOETHER_REVISION,
            "sampling": "first, middle, and last row in each installed shell",
            "sampled_objects": counts, "full_mathematical_recomputation": false,
            "full_artifact_hash_verification": "scripts/volume-one.py verify",
        }))?,
    )?;
    Ok(())
}
