//! Write fully validated successor shells without changing historical inputs.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    time::{Instant, SystemTime},
};
use steiner_census::{
    Census, Generator,
    enrich::mapping_records,
    jsonl::emit_completed_shell,
    validate::{reconstruct_category, validate_census},
};

#[derive(Parser)]
struct Options {
    #[arg(long)]
    data_root: PathBuf,
    #[arg(long)]
    output_root: PathBuf,
    #[arg(long)]
    report: PathBuf,
}
fn read<T: DeserializeOwned>(directory: &Path, name: &str) -> Result<Vec<T>> {
    let file = File::open(directory.join(name))?;
    let reader: Box<dyn Read> = if name.ends_with(".zst") {
        Box::new(zstd::stream::read::Decoder::new(file)?)
    } else {
        Box::new(file)
    };
    BufReader::new(reader)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect()
}
fn main() -> Result<()> {
    let options = Options::parse();
    ensure!(
        !options.output_root.exists(),
        "successor root already exists"
    );
    let mut changes = Vec::new();
    for size in 0..=11 {
        let started = SystemTime::now();
        let timer = Instant::now();
        let directory = options
            .data_root
            .join(format!("census/record-version=1/atomic-size={size}"));
        steiner_census::jsonl::verify_producer_outputs(&directory)?;
        let run: Value = serde_json::from_slice(&fs::read(directory.join("run.json"))?)?;
        let mut census = Census {
            generator: Generator::CellAttachment,
            atomic_size: size,
            candidate_cap: run["candidate_cap"].as_u64().unwrap_or(20_000_000),
            raw_candidate_count: run["raw_candidate_count"].as_u64().unwrap_or(0),
            categories: read(&directory, "categories.jsonl")?,
            mappings: read(&directory, "mapping-categories.jsonl.zst")?,
            basepoints: read(&directory, "monomorphic-basepoints.jsonl.zst")?,
            homotopy_posets: read(&directory, "homotopy-posets.jsonl.zst")?,
            categorical_homology: read(&directory, "categorical-homology.jsonl.zst")?,
            categorical_homotopy: read(&directory, "categorical-homotopy-posets.jsonl.zst")?,
        };
        let mut corrected = Vec::new();
        for category in &census.categories {
            corrected.extend(mapping_records(
                &category.key,
                &reconstruct_category(category)?,
            )?);
        }
        ensure!(
            corrected.len() == census.mappings.len(),
            "mapping count changed"
        );
        for (old, new) in census.mappings.iter().zip(&corrected) {
            ensure!(
                old.category == new.category
                    && old.source == new.source
                    && old.target == new.target,
                "mapping pair order changed"
            );
            ensure!(
                old.cells == new.cells && old.exact_cell_counts == new.exact_cell_counts,
                "actual mapping cells changed"
            );
            if old != new {
                changes.push(json!({"atomic_size":size,"category":old.category,"source":old.source,"target":old.target,"old_key":old.mapping_category,"new_key":new.mapping_category,"old_has_model":old.has_strong_steiner_model,"new_has_model":new.has_strong_steiner_model}));
            }
        }
        census.mappings = corrected;
        let validation = validate_census(&census)?;
        let destination = emit_completed_shell(
            &options.output_root,
            &census,
            &validation,
            2,
            started,
            timer.elapsed(),
        )?;
        // Retain the original run as a separately hashed upstream input. These
        // records were replayed, not re-enumerated by this correction command.
        fs::copy(
            directory.join("run.json"),
            destination.join("upstream-run.json"),
        )?;
        let run_path = destination.join("run.json");
        let mut report: Value = serde_json::from_slice(&fs::read(&run_path)?)?;
        report["operation"] = json!("correct-and-revalidate-mapping-models");
        report["upstream_run_sha256"] = json!(hash(&destination.join("upstream-run.json"))?);
        fs::write(run_path, serde_json::to_vec_pretty(&report)?)?;
        eprintln!(
            "size={size} categories={} mappings={} elapsed={:.1}s",
            census.categories.len(),
            census.mappings.len(),
            timer.elapsed().as_secs_f64()
        );
    }
    fs::copy(
        options.data_root.join("gray-products.jsonl"),
        options.output_root.join("gray-products.jsonl"),
    )?;
    fs::write(
        options.report,
        serde_json::to_vec_pretty(
            &json!({"status":"passed","producer":poset_census::source_identity(),"changed_mappings":changes,"scope":"all monolithic mapping rows through atomic size 11; size 12 is rebuilt separately"}),
        )?,
    )?;
    Ok(())
}
fn hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    Ok(hex::encode(Sha256::digest(fs::read(path)?)))
}
