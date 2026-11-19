use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use crate::{
    CONTRACT, Census, IDENTITY_CONTRACT, IdentityCensus, LIGHT_ENRICHMENT_CONTRACT, LightCensus,
    MAPPING_ENRICHMENT_CONTRACT, MappingCensus, RICH_ENRICHMENT_CONTRACT, RichCensus,
    enumerate::EnumerationStats,
    records::{
        CoverageManifest, CoverageUpstream, InvariantCoverage, MappingCategoryRecord,
        SteinerIdentityRecord, SteinerLightEnrichmentRecord, SteinerMappingEnrichmentRecord,
    },
    validate::{
        EnrichmentValidationReport, IdentityValidationReport, MappingValidationReport,
        ValidationReport,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FileReport {
    kind: String,
    path: String,
    row_count: usize,
    byte_count: u64,
    sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct LayerInputReport {
    contract: String,
    atomic_size: usize,
    records_sha256: String,
    validation_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct LayerRunReport<'a> {
    contract: &'a str,
    run_id: String,
    operation: &'a str,
    started_unix_milliseconds: u128,
    elapsed_milliseconds: u128,
    atomic_size: usize,
    noether_version: &'static str,
    producer: poset_census::SourceIdentity,
    noether_revision: &'static str,
    mapping_model_algorithm: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    generator: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_cap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enumeration: Option<&'a EnumerationStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    street_nerve_degree: Option<usize>,
    inputs: Vec<LayerInputReport>,
    outputs: Vec<FileReport>,
    validation_sha256: String,
    reconstruction_command: String,
}

#[derive(Clone, Debug)]
pub struct LoadedIdentityShell {
    pub records: Vec<SteinerIdentityRecord>,
    pub records_sha256: String,
    pub validation_sha256: String,
}

#[derive(Clone, Debug)]
pub struct LoadedLightShell {
    pub records: Vec<SteinerLightEnrichmentRecord>,
    pub records_sha256: String,
    pub validation_sha256: String,
}

#[derive(Clone, Debug)]
pub struct LoadedMappingShell {
    pub records: Vec<SteinerMappingEnrichmentRecord>,
    pub mappings: Vec<MappingCategoryRecord>,
    pub records_sha256: String,
    pub mappings_sha256: String,
    pub validation_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct RunReport {
    contract: &'static str,
    run_id: String,
    operation: &'static str,
    started_unix_milliseconds: u128,
    elapsed_milliseconds: u128,
    generator: &'static str,
    atomic_size: usize,
    candidate_cap: u64,
    raw_candidate_count: u64,
    noether_version: &'static str,
    producer: poset_census::SourceIdentity,
    noether_revision: &'static str,
    mapping_model_algorithm: &'static str,
    street_nerve_degree: usize,
    outputs: Vec<FileReport>,
    validation_sha256: String,
    reconstruction_command: String,
}

pub fn emit_identity_shell(
    data_root: &Path,
    census: &IdentityCensus,
    validation: &IdentityValidationReport,
    started: SystemTime,
    elapsed: Duration,
) -> Result<PathBuf> {
    let version_root = data_root.join("census/identity-version=1");
    let final_directory = version_root.join(format!("atomic-size={}", census.atomic_size));
    let staging = staging_directory(&version_root, &final_directory, census.atomic_size)?;
    let mut outputs = vec![write_jsonl(
        &staging,
        "identities.jsonl",
        "identities",
        &census.identities,
        false,
    )?];
    let coverage = identity_coverage(census.atomic_size, census.identities.len());
    outputs.push(write_json(
        &staging,
        "coverage.json",
        "coverage",
        &coverage,
    )?);
    let validation_output = write_json(&staging, "validation.json", "validation", validation)?;
    let validation_sha256 = validation_output.sha256.clone();
    outputs.push(validation_output);
    let report = LayerRunReport {
        contract: IDENTITY_CONTRACT,
        run_id: layer_run_id("identity", census.atomic_size, started)?,
        operation: "identity",
        started_unix_milliseconds: unix_milliseconds(started)?,
        elapsed_milliseconds: elapsed.as_millis(),
        atomic_size: census.atomic_size,
        noether_version: noether::VERSION,
        producer: poset_census::source_identity(),
        noether_revision: poset_census::NOETHER_REVISION,
        mapping_model_algorithm: "full-cell-skeleton-bijection/v1",
        generator: Some(census.generator.id()),
        candidate_cap: Some(census.candidate_cap),
        enumeration: Some(&census.stats),
        street_nerve_degree: None,
        inputs: Vec::new(),
        outputs,
        validation_sha256,
        reconstruction_command: format!(
            "steiner-census identity --atomic-size {} --candidate-cap {} --data-root <explicit-data-root>",
            census.atomic_size, census.candidate_cap
        ),
    };
    write_json(&staging, "run.json", "run", &report)?;
    publish_staging(&staging, &final_directory)
}

pub fn emit_light_shell(
    data_root: &Path,
    census: &LightCensus,
    validation: &EnrichmentValidationReport,
    identity: &LoadedIdentityShell,
    started: SystemTime,
    elapsed: Duration,
) -> Result<PathBuf> {
    let version_root = data_root.join("enrichments/light-version=1");
    let final_directory = version_root.join(format!("atomic-size={}", census.atomic_size));
    let staging = staging_directory(&version_root, &final_directory, census.atomic_size)?;
    let mut outputs = vec![write_jsonl(
        &staging,
        "light-enrichments.jsonl.zst",
        "light-enrichments",
        &census.records,
        true,
    )?];
    let upstream = vec![CoverageUpstream {
        contract: IDENTITY_CONTRACT.to_owned(),
        atomic_size: census.atomic_size,
        validation_sha256: identity.validation_sha256.clone(),
    }];
    outputs.push(write_json(
        &staging,
        "coverage.json",
        "coverage",
        &light_coverage(census.atomic_size, census.records.len(), upstream),
    )?);
    let validation_output = write_json(&staging, "validation.json", "validation", validation)?;
    let validation_sha256 = validation_output.sha256.clone();
    outputs.push(validation_output);
    let report = LayerRunReport {
        contract: LIGHT_ENRICHMENT_CONTRACT,
        run_id: layer_run_id("light", census.atomic_size, started)?,
        operation: "light-enrichment",
        started_unix_milliseconds: unix_milliseconds(started)?,
        elapsed_milliseconds: elapsed.as_millis(),
        atomic_size: census.atomic_size,
        noether_version: noether::VERSION,
        producer: poset_census::source_identity(),
        noether_revision: poset_census::NOETHER_REVISION,
        mapping_model_algorithm: "full-cell-skeleton-bijection/v1",
        generator: None,
        candidate_cap: None,
        enumeration: None,
        street_nerve_degree: None,
        inputs: vec![LayerInputReport {
            contract: IDENTITY_CONTRACT.to_owned(),
            atomic_size: census.atomic_size,
            records_sha256: identity.records_sha256.clone(),
            validation_sha256: identity.validation_sha256.clone(),
        }],
        outputs,
        validation_sha256,
        reconstruction_command: format!(
            "steiner-census light --atomic-size {} --data-root <explicit-data-root>",
            census.atomic_size
        ),
    };
    write_json(&staging, "run.json", "run", &report)?;
    publish_staging(&staging, &final_directory)
}

pub fn emit_mapping_shell(
    data_root: &Path,
    census: &MappingCensus,
    validation: &MappingValidationReport,
    identity: &LoadedIdentityShell,
    started: SystemTime,
    elapsed: Duration,
) -> Result<PathBuf> {
    let version_root = data_root.join("enrichments/mapping-version=1");
    let final_directory = version_root.join(format!("atomic-size={}", census.atomic_size));
    let staging = staging_directory(&version_root, &final_directory, census.atomic_size)?;
    let mut outputs = vec![
        write_jsonl(
            &staging,
            "mapping-enrichments.jsonl.zst",
            "mapping-enrichments",
            &census.records,
            true,
        )?,
        write_jsonl(
            &staging,
            "mapping-categories.jsonl.zst",
            "mapping-categories",
            &census.mappings,
            true,
        )?,
    ];
    let upstream = vec![CoverageUpstream {
        contract: IDENTITY_CONTRACT.to_owned(),
        atomic_size: census.atomic_size,
        validation_sha256: identity.validation_sha256.clone(),
    }];
    outputs.push(write_json(
        &staging,
        "coverage.json",
        "coverage",
        &mapping_coverage(census.atomic_size, census.records.len(), upstream),
    )?);
    let validation_output = write_json(&staging, "validation.json", "validation", validation)?;
    let validation_sha256 = validation_output.sha256.clone();
    outputs.push(validation_output);
    let report = LayerRunReport {
        contract: MAPPING_ENRICHMENT_CONTRACT,
        run_id: layer_run_id("mapping", census.atomic_size, started)?,
        operation: "mapping-enrichment",
        started_unix_milliseconds: unix_milliseconds(started)?,
        elapsed_milliseconds: elapsed.as_millis(),
        atomic_size: census.atomic_size,
        noether_version: noether::VERSION,
        producer: poset_census::source_identity(),
        noether_revision: poset_census::NOETHER_REVISION,
        mapping_model_algorithm: "full-cell-skeleton-bijection/v1",
        generator: None,
        candidate_cap: None,
        enumeration: None,
        street_nerve_degree: None,
        inputs: vec![LayerInputReport {
            contract: IDENTITY_CONTRACT.to_owned(),
            atomic_size: census.atomic_size,
            records_sha256: identity.records_sha256.clone(),
            validation_sha256: identity.validation_sha256.clone(),
        }],
        outputs,
        validation_sha256,
        reconstruction_command: format!(
            "steiner-census mappings --atomic-size {} --data-root <explicit-data-root>",
            census.atomic_size
        ),
    };
    write_json(&staging, "run.json", "run", &report)?;
    publish_staging(&staging, &final_directory)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_rich_shell(
    data_root: &Path,
    census: &RichCensus,
    validation: &ValidationReport,
    identity: &LoadedIdentityShell,
    light: &LoadedLightShell,
    street_degree: usize,
    started: SystemTime,
    elapsed: Duration,
) -> Result<PathBuf> {
    let version_root = data_root.join("enrichments/rich-version=1");
    let final_directory = version_root.join(format!("atomic-size={}", census.atomic_size));
    let staging = staging_directory(&version_root, &final_directory, census.atomic_size)?;
    let mut outputs = vec![
        write_jsonl(
            &staging,
            "rich-enrichments.jsonl.zst",
            "rich-enrichments",
            &census.records,
            true,
        )?,
        write_jsonl(
            &staging,
            "mapping-categories.jsonl.zst",
            "mapping-categories",
            &census.mappings,
            true,
        )?,
        write_jsonl(
            &staging,
            "monomorphic-basepoints.jsonl.zst",
            "monomorphic-basepoints",
            &census.basepoints,
            true,
        )?,
        write_jsonl(
            &staging,
            "homotopy-posets.jsonl.zst",
            "homotopy-posets",
            &census.homotopy_posets,
            true,
        )?,
        write_jsonl(
            &staging,
            "categorical-homology.jsonl.zst",
            "categorical-homology",
            &census.categorical_homology,
            true,
        )?,
        write_jsonl(
            &staging,
            "categorical-homotopy-posets.jsonl.zst",
            "categorical-homotopy-posets",
            &census.categorical_homotopy,
            true,
        )?,
    ];
    let upstream = vec![
        CoverageUpstream {
            contract: IDENTITY_CONTRACT.to_owned(),
            atomic_size: census.atomic_size,
            validation_sha256: identity.validation_sha256.clone(),
        },
        CoverageUpstream {
            contract: LIGHT_ENRICHMENT_CONTRACT.to_owned(),
            atomic_size: census.atomic_size,
            validation_sha256: light.validation_sha256.clone(),
        },
    ];
    outputs.push(write_json(
        &staging,
        "coverage.json",
        "coverage",
        &rich_coverage(
            census.atomic_size,
            census.records.len(),
            street_degree,
            upstream,
        ),
    )?);
    let validation_output = write_json(&staging, "validation.json", "validation", validation)?;
    let validation_sha256 = validation_output.sha256.clone();
    outputs.push(validation_output);
    let report = LayerRunReport {
        contract: RICH_ENRICHMENT_CONTRACT,
        run_id: layer_run_id("rich", census.atomic_size, started)?,
        operation: "rich-enrichment",
        started_unix_milliseconds: unix_milliseconds(started)?,
        elapsed_milliseconds: elapsed.as_millis(),
        atomic_size: census.atomic_size,
        noether_version: noether::VERSION,
        producer: poset_census::source_identity(),
        noether_revision: poset_census::NOETHER_REVISION,
        mapping_model_algorithm: "full-cell-skeleton-bijection/v1",
        generator: None,
        candidate_cap: None,
        enumeration: None,
        street_nerve_degree: Some(street_degree),
        inputs: vec![
            LayerInputReport {
                contract: IDENTITY_CONTRACT.to_owned(),
                atomic_size: census.atomic_size,
                records_sha256: identity.records_sha256.clone(),
                validation_sha256: identity.validation_sha256.clone(),
            },
            LayerInputReport {
                contract: LIGHT_ENRICHMENT_CONTRACT.to_owned(),
                atomic_size: census.atomic_size,
                records_sha256: light.records_sha256.clone(),
                validation_sha256: light.validation_sha256.clone(),
            },
        ],
        outputs,
        validation_sha256,
        reconstruction_command: format!(
            "steiner-census rich --atomic-size {} --candidate-cap <identity-candidate-cap> --street-degree {} --data-root <explicit-data-root>",
            census.atomic_size, street_degree
        ),
    };
    write_json(&staging, "run.json", "run", &report)?;
    publish_staging(&staging, &final_directory)
}

pub fn load_identity_shell(data_root: &Path, atomic_size: usize) -> Result<LoadedIdentityShell> {
    let directory = data_root
        .join("census/identity-version=1")
        .join(format!("atomic-size={atomic_size}"));
    verify_producer_outputs(&directory)?;
    let records_path = directory.join("identities.jsonl");
    let validation_path = directory.join("validation.json");
    let validation: IdentityValidationReport = read_json(&validation_path)?;
    if validation.status != "passed"
        || validation.contract != IDENTITY_CONTRACT
        || validation.atomic_size != atomic_size
    {
        bail!("identity shell {} is not validated", directory.display());
    }
    let records = read_jsonl(&records_path, false)?;
    if records.len() != validation.category_count {
        bail!("identity shell row count does not match validation");
    }
    Ok(LoadedIdentityShell {
        records,
        records_sha256: sha256_file(&records_path)?,
        validation_sha256: sha256_file(&validation_path)?,
    })
}

pub fn load_light_shell(data_root: &Path, atomic_size: usize) -> Result<LoadedLightShell> {
    let directory = data_root
        .join("enrichments/light-version=1")
        .join(format!("atomic-size={atomic_size}"));
    verify_producer_outputs(&directory)?;
    let records_path = directory.join("light-enrichments.jsonl.zst");
    let validation_path = directory.join("validation.json");
    let validation: EnrichmentValidationReport = read_json(&validation_path)?;
    if validation.status != "passed"
        || validation.contract != LIGHT_ENRICHMENT_CONTRACT
        || validation.atomic_size != atomic_size
    {
        bail!(
            "light-enrichment shell {} is not validated",
            directory.display()
        );
    }
    let records = read_jsonl(&records_path, true)?;
    if records.len() != validation.enrichment_count {
        bail!("light-enrichment row count does not match validation");
    }
    Ok(LoadedLightShell {
        records,
        records_sha256: sha256_file(&records_path)?,
        validation_sha256: sha256_file(&validation_path)?,
    })
}

pub fn load_mapping_shell(data_root: &Path, atomic_size: usize) -> Result<LoadedMappingShell> {
    let directory = data_root
        .join("enrichments/mapping-version=1")
        .join(format!("atomic-size={atomic_size}"));
    verify_producer_outputs(&directory)?;
    let records_path = directory.join("mapping-enrichments.jsonl.zst");
    let mappings_path = directory.join("mapping-categories.jsonl.zst");
    let validation_path = directory.join("validation.json");
    let validation: MappingValidationReport = read_json(&validation_path)?;
    if validation.status != "passed"
        || validation.contract != MAPPING_ENRICHMENT_CONTRACT
        || validation.atomic_size != atomic_size
    {
        bail!(
            "mapping-enrichment shell {} is not validated",
            directory.display()
        );
    }
    let records = read_jsonl(&records_path, true)?;
    let mappings = read_jsonl(&mappings_path, true)?;
    if records.len() != validation.enrichment_count || mappings.len() != validation.mapping_count {
        bail!("mapping-enrichment shell row count does not match validation");
    }
    Ok(LoadedMappingShell {
        records,
        mappings,
        records_sha256: sha256_file(&records_path)?,
        mappings_sha256: sha256_file(&mappings_path)?,
        validation_sha256: sha256_file(&validation_path)?,
    })
}

fn staging_directory(
    version_root: &Path,
    final_directory: &Path,
    atomic_size: usize,
) -> Result<PathBuf> {
    if final_directory.exists() {
        bail!(
            "completed layer directory {} already exists; completed layers are immutable",
            final_directory.display()
        );
    }
    fs::create_dir_all(version_root)
        .with_context(|| format!("creating {}", version_root.display()))?;
    let staging = version_root.join(format!(
        ".atomic-size={atomic_size}.tmp-{}-{}",
        process::id(),
        unix_milliseconds(SystemTime::now())?
    ));
    fs::create_dir(&staging).with_context(|| format!("creating {}", staging.display()))?;
    Ok(staging)
}

fn publish_staging(staging: &Path, final_directory: &Path) -> Result<PathBuf> {
    fs::rename(staging, final_directory).with_context(|| {
        format!(
            "publishing {} as {}",
            staging.display(),
            final_directory.display()
        )
    })?;
    Ok(final_directory.to_path_buf())
}

fn layer_run_id(operation: &str, atomic_size: usize, started: SystemTime) -> Result<String> {
    Ok(format!(
        "{operation}-atomic-size-{atomic_size}-{}-{}",
        unix_milliseconds(started)?,
        process::id()
    ))
}

fn coverage_claim(invariant: &str, status: &str, scope: impl Into<String>) -> InvariantCoverage {
    InvariantCoverage {
        invariant: invariant.to_owned(),
        status: status.to_owned(),
        scope: scope.into(),
    }
}

fn identity_coverage(atomic_size: usize, category_count: usize) -> CoverageManifest {
    let shell = format!("all identities of total atomic size {atomic_size}");
    CoverageManifest {
        contract: IDENTITY_CONTRACT.to_owned(),
        status: "complete".to_owned(),
        atomic_size,
        category_count,
        upstream: Vec::new(),
        invariants: vec![
            coverage_claim("canonical-boundary-data", "complete", &shell),
            coverage_claim("strict-isomorphism-class", "complete", &shell),
            coverage_claim("atomic-dimension-vector", "complete", &shell),
            coverage_claim(
                "generating-skeleton-connectivity",
                "not-materialized",
                &shell,
            ),
            coverage_claim("integral-polygraphic-homology", "not-materialized", &shell),
            coverage_claim("natural-automorphism-group", "not-materialized", &shell),
            coverage_claim("dualities", "not-materialized", &shell),
            coverage_claim("exact-cell-counts", "not-materialized", &shell),
            coverage_claim("mapping-categories", "not-materialized", &shell),
            coverage_claim("monomorphic-basepoints", "not-materialized", &shell),
            coverage_claim("homotopy-posets", "not-materialized", &shell),
            coverage_claim("street-nerve-skeleton", "not-materialized", &shell),
            coverage_claim(
                "categorical-homotopy-component-posets",
                "not-materialized",
                &shell,
            ),
        ],
    }
}

fn light_coverage(
    atomic_size: usize,
    category_count: usize,
    upstream: Vec<CoverageUpstream>,
) -> CoverageManifest {
    let shell = format!("all identities of total atomic size {atomic_size}");
    CoverageManifest {
        contract: LIGHT_ENRICHMENT_CONTRACT.to_owned(),
        status: "complete".to_owned(),
        atomic_size,
        category_count,
        upstream,
        invariants: vec![
            coverage_claim("canonical-boundary-data", "upstream-complete", &shell),
            coverage_claim("strict-isomorphism-class", "upstream-complete", &shell),
            coverage_claim("atomic-dimension-vector", "upstream-complete", &shell),
            coverage_claim("generating-skeleton-connectivity", "complete", &shell),
            coverage_claim("integral-polygraphic-homology", "complete", &shell),
            coverage_claim("natural-automorphism-group", "complete", &shell),
            coverage_claim("dualities", "complete", &shell),
            coverage_claim("recognized-families", "complete", &shell),
            coverage_claim("exact-cell-counts", "not-materialized", &shell),
            coverage_claim("mapping-categories", "not-materialized", &shell),
            coverage_claim("monomorphic-basepoints", "not-materialized", &shell),
            coverage_claim("homotopy-posets", "not-materialized", &shell),
            coverage_claim("street-nerve-skeleton", "not-materialized", &shell),
            coverage_claim(
                "categorical-homotopy-component-posets",
                "not-materialized",
                &shell,
            ),
        ],
    }
}

fn mapping_coverage(
    atomic_size: usize,
    category_count: usize,
    upstream: Vec<CoverageUpstream>,
) -> CoverageManifest {
    let shell = format!("all identities of total atomic size {atomic_size}");
    CoverageManifest {
        contract: MAPPING_ENRICHMENT_CONTRACT.to_owned(),
        status: "complete".to_owned(),
        atomic_size,
        category_count,
        upstream,
        invariants: vec![
            coverage_claim("canonical-boundary-data", "upstream-complete", &shell),
            coverage_claim("strict-isomorphism-class", "upstream-complete", &shell),
            coverage_claim("atomic-dimension-vector", "upstream-complete", &shell),
            coverage_claim("exact-cell-counts", "complete", &shell),
            coverage_claim("exact-cell-register", "complete", &shell),
            coverage_claim("mapping-categories", "complete", &shell),
            coverage_claim("monomorphic-basepoints", "not-materialized", &shell),
            coverage_claim("homotopy-posets", "not-materialized", &shell),
            coverage_claim("street-nerve-skeleton", "not-materialized", &shell),
            coverage_claim(
                "categorical-homotopy-component-posets",
                "not-materialized",
                &shell,
            ),
        ],
    }
}

fn rich_coverage(
    atomic_size: usize,
    category_count: usize,
    street_degree: usize,
    upstream: Vec<CoverageUpstream>,
) -> CoverageManifest {
    let shell = format!("all identities of total atomic size {atomic_size}");
    CoverageManifest {
        contract: RICH_ENRICHMENT_CONTRACT.to_owned(),
        status: "complete".to_owned(),
        atomic_size,
        category_count,
        upstream,
        invariants: vec![
            coverage_claim("canonical-boundary-data", "upstream-complete", &shell),
            coverage_claim("strict-isomorphism-class", "upstream-complete", &shell),
            coverage_claim("atomic-dimension-vector", "upstream-complete", &shell),
            coverage_claim(
                "generating-skeleton-connectivity",
                "upstream-complete",
                &shell,
            ),
            coverage_claim("integral-polygraphic-homology", "upstream-complete", &shell),
            coverage_claim("natural-automorphism-group", "upstream-complete", &shell),
            coverage_claim("dualities", "upstream-complete", &shell),
            coverage_claim("exact-cell-counts", "complete", &shell),
            coverage_claim("mapping-categories", "complete", &shell),
            coverage_claim("monomorphic-basepoints", "complete", &shell),
            coverage_claim("homotopy-posets", "complete", &shell),
            coverage_claim(
                "street-nerve-skeleton",
                "complete",
                format!("{shell}; simplicial degrees 0 through {street_degree}"),
            ),
            coverage_claim(
                "categorical-homotopy-component-posets",
                "complete",
                format!("{shell}; particle numbers 0 through 2"),
            ),
        ],
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("parsing {}", path.display()))
}

fn read_jsonl<T: DeserializeOwned>(path: &Path, compressed: bool) -> Result<Vec<T>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader: Box<dyn BufRead> = if compressed {
        Box::new(BufReader::new(zstd::stream::read::Decoder::new(file)?))
    } else {
        Box::new(BufReader::new(file))
    };
    reader
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(&line?)
                .with_context(|| format!("parsing {} line {}", path.display(), index + 1))
        })
        .collect()
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(hex::encode(digest.finalize()))
}

pub fn emit_completed_shell(
    data_root: &Path,
    census: &Census,
    validation: &ValidationReport,
    street_degree: usize,
    started: SystemTime,
    elapsed: Duration,
) -> Result<PathBuf> {
    let version_root = data_root.join("census/record-version=1");
    let final_directory = version_root.join(format!("atomic-size={}", census.atomic_size));
    if final_directory.exists() {
        bail!(
            "completed census directory {} already exists; completed shells are immutable",
            final_directory.display()
        );
    }
    fs::create_dir_all(&version_root)
        .with_context(|| format!("creating {}", version_root.display()))?;
    let staging = version_root.join(format!(
        ".atomic-size={}.tmp-{}-{}",
        census.atomic_size,
        process::id(),
        unix_milliseconds(SystemTime::now())?
    ));
    fs::create_dir(&staging).with_context(|| format!("creating {}", staging.display()))?;

    let mut outputs = vec![
        write_jsonl(
            &staging,
            "categories.jsonl",
            "categories",
            &census.categories,
            false,
        )?,
        write_jsonl(
            &staging,
            "mapping-categories.jsonl.zst",
            "mapping-categories",
            &census.mappings,
            true,
        )?,
        write_jsonl(
            &staging,
            "monomorphic-basepoints.jsonl.zst",
            "monomorphic-basepoints",
            &census.basepoints,
            true,
        )?,
        write_jsonl(
            &staging,
            "homotopy-posets.jsonl.zst",
            "homotopy-posets",
            &census.homotopy_posets,
            true,
        )?,
        write_jsonl(
            &staging,
            "categorical-homology.jsonl.zst",
            "categorical-homology",
            &census.categorical_homology,
            true,
        )?,
        write_jsonl(
            &staging,
            "categorical-homotopy-posets.jsonl.zst",
            "categorical-homotopy-posets",
            &census.categorical_homotopy,
            true,
        )?,
    ];
    let validation_output = write_json(&staging, "validation.json", "validation", validation)?;
    let validation_sha256 = validation_output.sha256.clone();
    outputs.push(validation_output);

    let report = RunReport {
        contract: CONTRACT,
        run_id: format!(
            "{}-atomic-size-{}-{}-{}",
            census.generator.id(),
            census.atomic_size,
            unix_milliseconds(started)?,
            process::id()
        ),
        operation: "emit",
        started_unix_milliseconds: unix_milliseconds(started)?,
        elapsed_milliseconds: elapsed.as_millis(),
        generator: census.generator.id(),
        atomic_size: census.atomic_size,
        candidate_cap: census.candidate_cap,
        raw_candidate_count: census.raw_candidate_count,
        noether_version: noether::VERSION,
        producer: poset_census::source_identity(),
        noether_revision: poset_census::NOETHER_REVISION,
        mapping_model_algorithm: "full-cell-skeleton-bijection/v1",
        street_nerve_degree: street_degree,
        outputs,
        validation_sha256,
        reconstruction_command: format!(
            "steiner-census {} --atomic-size {} --candidate-cap {} --street-degree {} --data-root <explicit-data-root>",
            match census.generator {
                crate::Generator::Reference => "reference",
                crate::Generator::CellAttachment => "orderly",
            },
            census.atomic_size,
            census.candidate_cap,
            street_degree
        ),
    };
    write_json(&staging, "run.json", "run", &report)?;
    fs::rename(&staging, &final_directory).with_context(|| {
        format!(
            "publishing {} as {}",
            staging.display(),
            final_directory.display()
        )
    })?;
    Ok(final_directory)
}

pub fn emit_gray_table<T: Serialize>(path: &Path, records: &[T]) -> Result<()> {
    if path.exists() {
        bail!("Gray table {} already exists", path.display());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for record in records {
        serde_json::to_writer(&mut writer, record)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn write_jsonl<T: Serialize>(
    directory: &Path,
    filename: &str,
    kind: &str,
    rows: &[T],
    compressed: bool,
) -> Result<FileReport> {
    let path = directory.join(filename);
    if compressed {
        let file = File::create(&path)?;
        let mut encoder = zstd::stream::write::Encoder::new(file, 9)?;
        for row in rows {
            serde_json::to_writer(&mut encoder, row)?;
            encoder.write_all(b"\n")?;
        }
        encoder.finish()?;
    } else {
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        for row in rows {
            serde_json::to_writer(&mut writer, row)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
    }
    file_report(&path, filename, kind, rows.len())
}

fn write_json<T: Serialize>(
    directory: &Path,
    filename: &str,
    kind: &str,
    value: &T,
) -> Result<FileReport> {
    let path = directory.join(filename);
    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    file_report(&path, filename, kind, 1)
}

fn file_report(path: &Path, filename: &str, kind: &str, rows: usize) -> Result<FileReport> {
    let bytes = fs::read(path)?;
    Ok(FileReport {
        kind: kind.to_owned(),
        path: filename.to_owned(),
        row_count: rows,
        byte_count: bytes.len() as u64,
        sha256: hex::encode(Sha256::digest(&bytes)),
    })
}

fn unix_milliseconds(time: SystemTime) -> Result<u128> {
    Ok(time
        .duration_since(UNIX_EPOCH)
        .context("system time precedes Unix epoch")?
        .as_millis())
}

/// Reject modified sidecars before any downstream lane consumes them.
pub fn verify_producer_outputs(directory: &Path) -> Result<()> {
    let run: serde_json::Value = read_json(&directory.join("run.json"))?;
    let outputs = run["outputs"]
        .as_array()
        .context("producer has no output inventory")?;
    let required: &[&str] = match run["contract"].as_str() {
        Some(IDENTITY_CONTRACT) => &["identities.jsonl", "validation.json", "coverage.json"],
        Some(LIGHT_ENRICHMENT_CONTRACT) => &[
            "light-enrichments.jsonl.zst",
            "validation.json",
            "coverage.json",
        ],
        Some(MAPPING_ENRICHMENT_CONTRACT) => &[
            "mapping-enrichments.jsonl.zst",
            "mapping-categories.jsonl.zst",
            "validation.json",
            "coverage.json",
        ],
        Some(CONTRACT) => &[
            "categories.jsonl",
            "mapping-categories.jsonl.zst",
            "monomorphic-basepoints.jsonl.zst",
            "homotopy-posets.jsonl.zst",
            "categorical-homology.jsonl.zst",
            "categorical-homotopy-posets.jsonl.zst",
            "validation.json",
        ],
        _ => bail!("unsupported producer contract"),
    };
    for name in required {
        anyhow::ensure!(
            outputs
                .iter()
                .filter(|output| output["path"] == *name)
                .count()
                == 1,
            "producer inventory must name {name} exactly once"
        );
    }
    if let Some(revision) = run["noether_revision"].as_str() {
        anyhow::ensure!(
            revision == poset_census::NOETHER_REVISION,
            "unsupported producer revision"
        );
    }
    for output in outputs {
        let name = output["path"].as_str().context("output has no path")?;
        let path = Path::new(name);
        anyhow::ensure!(
            path.components()
                .all(|part| matches!(part, std::path::Component::Normal(_))),
            "unsafe producer artifact path"
        );
        let path = directory.join(path);
        anyhow::ensure!(
            output["byte_count"] == fs::metadata(&path)?.len()
                && output["sha256"] == sha256_file(&path)?,
            "producer artifact {name} hash mismatch"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Generator, identity_census, light_census, mapping_census,
        validate::{validate_identity_census, validate_light_census, validate_mapping_census},
    };

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(
                std::env::temp_dir()
                    .join(format!("steiner-census-layers-{}-{nonce}", process::id())),
            )
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn emitted_layers_are_loadable_linked_and_immutable() {
        let scratch = Scratch::new();
        let atomic_size = 3;
        let candidate_cap = 100_000;
        let (stats, representatives) =
            crate::enumerate::orderly_representatives(atomic_size, candidate_cap).unwrap();
        let identities = identity_census(
            Generator::CellAttachment,
            atomic_size,
            candidate_cap,
            stats,
            representatives,
        )
        .unwrap();
        let identity_validation = validate_identity_census(&identities).unwrap();
        emit_identity_shell(
            &scratch.0,
            &identities,
            &identity_validation,
            UNIX_EPOCH + Duration::from_secs(1),
            Duration::from_millis(10),
        )
        .unwrap();
        let loaded_identity = load_identity_shell(&scratch.0, atomic_size).unwrap();
        assert_eq!(loaded_identity.records, identities.identities);
        assert!(
            emit_identity_shell(
                &scratch.0,
                &identities,
                &identity_validation,
                UNIX_EPOCH + Duration::from_secs(2),
                Duration::from_millis(10),
            )
            .unwrap_err()
            .to_string()
            .contains("immutable")
        );

        let light = light_census(atomic_size, &loaded_identity.records).unwrap();
        let light_validation = validate_light_census(&light, &loaded_identity.records).unwrap();
        emit_light_shell(
            &scratch.0,
            &light,
            &light_validation,
            &loaded_identity,
            UNIX_EPOCH + Duration::from_secs(3),
            Duration::from_millis(10),
        )
        .unwrap();
        let loaded_light = load_light_shell(&scratch.0, atomic_size).unwrap();
        assert_eq!(loaded_light.records, light.records);

        let coverage: CoverageManifest = read_json(
            &scratch
                .0
                .join("enrichments/light-version=1/atomic-size=3/coverage.json"),
        )
        .unwrap();
        assert_eq!(coverage.upstream.len(), 1);
        assert_eq!(
            coverage.upstream[0].validation_sha256,
            loaded_identity.validation_sha256
        );
        assert!(coverage.invariants.iter().any(|claim| {
            claim.invariant == "mapping-categories" && claim.status == "not-materialized"
        }));
        let mappings = mapping_census(atomic_size, &loaded_identity.records).unwrap();
        let mapping_validation =
            validate_mapping_census(&mappings, &loaded_identity.records).unwrap();
        emit_mapping_shell(
            &scratch.0,
            &mappings,
            &mapping_validation,
            &loaded_identity,
            UNIX_EPOCH + Duration::from_secs(4),
            Duration::from_millis(10),
        )
        .unwrap();
        let loaded_mappings = load_mapping_shell(&scratch.0, atomic_size).unwrap();
        assert_eq!(loaded_mappings.records, mappings.records);
        assert_eq!(loaded_mappings.mappings, mappings.mappings);
        let coverage: CoverageManifest = read_json(
            &scratch
                .0
                .join("enrichments/mapping-version=1/atomic-size=3/coverage.json"),
        )
        .unwrap();
        assert!(coverage.invariants.iter().any(|claim| {
            claim.invariant == "mapping-categories" && claim.status == "complete"
        }));
        let mappings_path = scratch
            .0
            .join("enrichments/mapping-version=1/atomic-size=3/mapping-categories.jsonl.zst");
        let mut bytes = fs::read(&mappings_path).unwrap();
        bytes.push(b'!');
        fs::write(mappings_path, bytes).unwrap();
        assert!(load_mapping_shell(&scratch.0, 3).is_err());
    }
}
