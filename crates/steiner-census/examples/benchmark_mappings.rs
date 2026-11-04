use std::{env, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use steiner_census::{
    enrich::mapping_records, enumerate::orderly_representatives, jsonl::load_identity_shell,
    records::SteinerIdentityRecord,
};

fn main() -> Result<()> {
    let mut arguments = env::args().skip(1);
    let source = arguments
        .next()
        .context("missing data root or --generate")?;
    let atomic_size = arguments
        .next()
        .context("missing atomic size")?
        .parse::<usize>()?;
    let sample = arguments.next().map(|value| value.parse()).transpose()?;
    let identities = if source == "--generate" {
        orderly_representatives(atomic_size, 20_000_000)?
            .1
            .into_iter()
            .map(|(key, canonical)| {
                Ok(SteinerIdentityRecord {
                    record_version: steiner_census::RECORD_VERSION,
                    key,
                    atom_counts: canonical.category().atom_counts().to_vec(),
                    boundaries: steiner_census::records::boundaries(canonical.category()),
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        load_identity_shell(&PathBuf::from(source), atomic_size)?.records
    };
    let stride = sample
        .map(|count: usize| identities.len().div_ceil(count.max(1)))
        .unwrap_or(1);
    let timer = Instant::now();
    let mut category_count = 0usize;
    let mut mapping_count = 0usize;
    let mut stored_cell_count = 0usize;
    for identity in identities.iter().step_by(stride) {
        let category = identity.reconstruct()?;
        let mappings = mapping_records(&identity.key, &category)?;
        category_count += 1;
        mapping_count += mappings.len();
        stored_cell_count += mappings
            .iter()
            .flat_map(|mapping| &mapping.cells)
            .map(Vec::len)
            .sum::<usize>();
    }
    println!(
        "atomic_size={atomic_size} shell_categories={} sampled_categories={category_count} stride={stride} mappings={mapping_count} stored_cells={stored_cell_count} elapsed_ms={}",
        identities.len(),
        timer.elapsed().as_millis(),
    );
    Ok(())
}
