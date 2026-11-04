//! Recompute selected cohomology batches while retaining their historical inputs.

use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};

#[derive(Parser)]
struct Options {
    #[arg(long)]
    data_root: PathBuf,
    #[arg(long)]
    output_root: PathBuf,
    #[arg(long)]
    report: PathBuf,
    #[arg(long)]
    collection: String,
}

fn differences(old: &Value, new: &Value, path: &str, output: &mut Vec<String>) {
    if old == new {
        return;
    }
    match (old, new) {
        (Value::Object(a), Value::Object(b)) => {
            let keys = a
                .keys()
                .chain(b.keys())
                .collect::<std::collections::BTreeSet<_>>();
            for key in keys {
                let child_path = format!("{path}/{key}");
                match (a.get(key), b.get(key)) {
                    (Some(a), Some(b)) => differences(a, b, &child_path, output),
                    _ => output.push(child_path),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                differences(a, b, &format!("{path}/{index}"), output);
            }
        }
        _ => output.push(path.to_owned()),
    }
}

fn main() -> Result<()> {
    let options = Options::parse();
    let mut changes = Vec::new();
    let mut field_counts = BTreeMap::<String, usize>::new();
    let rows;
    match options.collection.as_str() {
        "eilenberg-maclane" => {
            use eilenberg_maclane_build::{enrich, jsonl, source, validate};
            let source = source::load_source()?;
            let (_, old) = jsonl::load_census(&options.data_root, 32, 4, 16)?;
            let new = enrich::build_census(&source, 32, 4, 16)?;
            ensure!(
                old.records.len() == new.records.len(),
                "space count changed"
            );
            rows = new.records.len();
            for (old, new) in old.records.iter().zip(&new.records) {
                ensure!(
                    old.label == new.label
                        && old.space_canonical_sha256 == new.space_canonical_sha256,
                    "symbolic space identity changed for {}",
                    old.label
                );
                let mut paths = Vec::new();
                differences(
                    &serde_json::to_value(old)?,
                    &serde_json::to_value(new)?,
                    "",
                    &mut paths,
                );
                if !paths.is_empty() {
                    for path in &paths {
                        *field_counts.entry(path.clone()).or_default() += 1;
                    }
                    changes.push(json!({"label": old.label, "paths": paths}));
                }
            }
            let report = validate::validate_census(&source, &new)?;
            jsonl::emit_census(
                &options.output_root,
                &new,
                source::validate_source(&source)?,
                report,
            )?;
        }
        "finite-group-modular" => {
            use finite_group_cohomology_census::{enrich, jsonl, source, validate};
            let source = source::load_source()?;
            let (_, old) = jsonl::load_modular_census(&options.data_root, 32, 8, 12)?;
            let new = enrich::complete_modular_census(&source, 32, 8, 12)?;
            ensure!(
                old.records.len() == new.records.len(),
                "coefficient count changed"
            );
            rows = new.records.len();
            for (old, new) in old.records.iter().zip(&new.records) {
                ensure!(
                    old.group_label == new.group_label
                        && old.group_canonical_sha256 == new.group_canonical_sha256,
                    "group identity changed for {}",
                    old.group_label
                );
                let mut paths = Vec::new();
                differences(
                    &serde_json::to_value(old)?,
                    &serde_json::to_value(new)?,
                    "",
                    &mut paths,
                );
                if !paths.is_empty() {
                    for path in &paths {
                        *field_counts.entry(path.clone()).or_default() += 1;
                    }
                    changes.push(json!({"label": old.group_label, "coefficient": old.coefficient_context, "paths": paths}));
                }
            }
            let report = validate::validate_modular_census(&source, &new)?;
            jsonl::emit_modular_census(&options.output_root, &new, report)?;
        }
        _ => anyhow::bail!("unknown migration collection"),
    }
    fs::write(
        &options.report,
        serde_json::to_vec_pretty(&json!({
            "status": "passed", "collection": options.collection,
            "noether_version": noether::VERSION, "noether_revision": eilenberg_maclane_build::NOETHER_REVISION,
            "rows": rows, "object_identities_unchanged": true,
            "changed_records": changes.len(), "changed_fields": field_counts, "changes": changes,
            "historical_inputs_preserved": true, "successor_root": options.output_root,
        }))?,
    )?;
    Ok(())
}
