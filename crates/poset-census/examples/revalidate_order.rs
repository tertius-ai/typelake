//! Revalidate a frozen order into a new review root, retaining original bytes.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use noether::{combin::Poset, topology::SimplicialComplex};
use poset_census::{
    enrich::{complex_key, complex_record},
    records::{ComplexKey, ComplexRecord, IntegralHomologyRecord, PosetRecord},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser)]
struct Options {
    #[arg(long)]
    data_root: PathBuf,
    #[arg(long)]
    output_root: PathBuf,
    #[arg(long)]
    order: usize,
}
fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    std::io::copy(&mut file, &mut hash)?;
    Ok(hex::encode(hash.finalize()))
}
fn main() -> Result<()> {
    let options = Options::parse();
    let timer = Instant::now();
    let relative = format!("census/record-version=1/order={}", options.order);
    let source = options.data_root.join(&relative);
    let destination = options.output_root.join(&relative);
    ensure!(!destination.exists(), "successor order already exists");
    let mut run: Value = serde_json::from_slice(&fs::read(source.join("run.json"))?)?;
    for artifact in run["outputs"]
        .as_array()
        .context("missing source artifacts")?
    {
        let path = source.join(artifact["path"].as_str().context("missing artifact path")?);
        ensure!(
            artifact["sha256"] == hash_file(&path)?,
            "source hash mismatch"
        );
    }
    let mut complexes = BTreeMap::<ComplexKey, (Vec<IntegralHomologyRecord>, bool)>::new();
    let mut previous = None;
    for line in BufReader::new(File::open(source.join("simplicial-complexes.jsonl"))?).lines() {
        let record: ComplexRecord = serde_json::from_str(&line?)?;
        ensure!(
            previous.as_ref().is_none_or(|key| key < &record.key),
            "complex keys are not sorted"
        );
        let complex = SimplicialComplex::from_facets(&record.facets);
        ensure!(
            complex_record(record.vertex_count, &complex)? == record,
            "direct source-complex homology differs"
        );
        let euler: i128 = record
            .integral_homology
            .iter()
            .map(|group| {
                if group.degree % 2 == 0 {
                    group.free_rank as i128
                } else {
                    -(group.free_rank as i128)
                }
            })
            .sum();
        ensure!(
            euler == i128::from(record.euler_characteristic),
            "Euler/Betti identity failed"
        );
        previous = Some(record.key.clone());
        complexes.insert(record.key, (record.integral_homology, false));
    }
    eprintln!(
        "order={} directly validated {} source complexes in {:.1}s",
        options.order,
        complexes.len(),
        timer.elapsed().as_secs_f64()
    );
    let mut previous = None;
    let mut count = 0_u128;
    let mut connected = 0_u128;
    let mut labeled = 0_u128;
    let factorial: u128 = (1..=options.order as u128).product();
    let mut core_homology = BTreeMap::new();
    let mut keys = Vec::new();
    for line in BufReader::new(File::open(source.join("posets.jsonl"))?).lines() {
        let record: PosetRecord = serde_json::from_str(&line?)?;
        ensure!(record.order == options.order, "mixed source order");
        ensure!(
            previous.as_ref().is_none_or(|key| key < &record.key),
            "poset keys are not sorted"
        );
        let poset = Poset::new(
            record.order,
            &record
                .covers
                .iter()
                .map(|edge| (edge[0], edge[1]))
                .collect::<Vec<_>>(),
        )?;
        let complex = poset.order_complex()?;
        let facets = complex
            .facets()
            .into_iter()
            .map(|facet| facet.vertices().to_vec())
            .collect::<Vec<_>>();
        let key = complex_key(record.order, &facets)?;
        ensure!(
            key == record.relations.order_complex.target,
            "order-complex target differs"
        );
        let stored = complexes.get_mut(&key).context("order complex is absent")?;
        stored.1 = true;
        let core = poset.core()?;
        ensure!(core.verify(&poset), "core reduction failed replay");
        let core_key = (
            core.core().order(),
            core.core()
                .covers()
                .edges()
                .iter()
                .map(|edge| (edge.source, edge.target))
                .collect::<Vec<_>>(),
        );
        if !core_homology.contains_key(&core_key) {
            ensure!(
                core.core().is_core() && core.core().core()?.core() == core.core(),
                "core is not idempotent"
            );
            let core_complex = complex_record(core.core().order(), &core.core().order_complex()?)?;
            core_homology.insert(core_key.clone(), signature(&core_complex.integral_homology));
        }
        ensure!(
            signature(&stored.0) == core_homology[&core_key],
            "source/core integral homology differs"
        );
        let group_order: u128 = record.automorphism_group.order.parse()?;
        ensure!(
            factorial.is_multiple_of(group_order),
            "automorphism order does not divide factorial"
        );
        labeled += factorial / group_order;
        let is_connected = poset.order() > 0 && poset.connected_components().len() == 1;
        ensure!(
            is_connected == record.invariants.is_connected,
            "connectivity differs"
        );
        connected += u128::from(is_connected);
        count += 1;
        if options.order <= 5 {
            keys.push(record.key.clone());
        }
        previous = Some(record.key);
        if count.is_multiple_of(100_000) {
            eprintln!(
                "order={} posets={count} elapsed={:.1}s",
                options.order,
                timer.elapsed().as_secs_f64()
            );
        }
    }
    let accepted = poset_census::validate::accepted_order_counts(options.order)?;
    ensure!(
        count == accepted.unlabeled
            && connected == accepted.connected_unlabeled
            && labeled == accepted.labeled,
        "accepted count identity failed"
    );
    ensure!(
        complexes.values().all(|(_, used)| *used),
        "unreferenced order complex"
    );
    if options.order <= 5 {
        let (_, reference) =
            poset_census::enumerate::reference_representatives(options.order, 1024)?;
        ensure!(
            reference.keys().eq(keys.iter()),
            "independent reference key set differs"
        );
    }
    let mut validation: Value = serde_json::from_slice(&fs::read(source.join("validation.json"))?)?;
    for key in [
        "core_replay_and_idempotence",
        "order_complex_reconstruction",
        "euler_and_integral_homology",
        "core_homology_invariance",
    ] {
        validation["checks"][key] = json!(true);
    }
    validation["checks"]["reference_key_set_complete"] = json!(options.order <= 5);
    validation["revalidation"] = json!({"producer":poset_census::source_identity(),"scope":"direct source homology/Euler, source/core homology invariance, core replay/idempotence, order-complex links, sorted keys and count identities; other gates retain source certification","source_validation_sha256":hash_file(&source.join("validation.json"))?,"posets_checked":count.to_string(),"complexes_checked":complexes.len(),"elapsed_seconds":timer.elapsed().as_secs_f64()});
    fs::create_dir_all(&destination)?;
    for name in ["posets.jsonl", "simplicial-complexes.jsonl"] {
        fs::copy(source.join(name), destination.join(name))?;
    }
    fs::write(
        destination.join("validation.json"),
        serde_json::to_vec_pretty(&validation)?,
    )?;
    for artifact in run["outputs"]
        .as_array_mut()
        .context("missing output inventory")?
    {
        if artifact["path"] == "validation.json" {
            artifact["sha256"] = json!(hash_file(&destination.join("validation.json"))?);
            artifact["byte_count"] =
                json!(fs::metadata(destination.join("validation.json"))?.len());
        }
    }
    run["validation_sha256"] = json!(hash_file(&destination.join("validation.json"))?);
    run["revalidation"] = validation["revalidation"].clone();
    fs::write(
        destination.join("run.json"),
        serde_json::to_vec_pretty(&run)?,
    )?;
    eprintln!(
        "order={} passed {count} posets in {:.1}s",
        options.order,
        timer.elapsed().as_secs_f64()
    );
    Ok(())
}

fn signature(groups: &[IntegralHomologyRecord]) -> Vec<(usize, usize, Vec<String>)> {
    groups
        .iter()
        .filter(|group| group.free_rank != 0 || !group.invariant_factors.is_empty())
        .map(|group| {
            (
                group.degree,
                group.free_rank,
                group.invariant_factors.clone(),
            )
        })
        .collect()
}
