//! Opt-in HAP parity for family and resolution paths plus named hand examples.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use finite_group_cohomology_census::{
    jsonl::load_modular_census,
    records::{FiniteGroupModularCohomologyRecord, ModularComputationRecord},
};

const MAX_DEGREE: usize = 3;

#[test]
#[ignore = "requires explicit GAP/HAP validation and the complete modular census"]
fn deterministic_order_prime_sample_matches_hap() {
    assert_eq!(
        env::var("TYPELAKE_VALIDATE_HAP").as_deref(),
        Ok("1"),
        "set TYPELAKE_VALIDATE_HAP=1 and provide the complete data root"
    );

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data_root = env::var_os("FINITE_GROUP_COHOMOLOGY_DATA_ROOT")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        })
        .unwrap_or_else(|| workspace_root.join("data"));
    let (_, census) = load_modular_census(&data_root, 32, 8, 12).unwrap();
    let mut sampled_paths = BTreeSet::new();
    let samples = census
        .records
        .iter()
        .fold(BTreeMap::new(), |mut samples, record| {
            let (order, _) = parse_label(&record.group_label);
            let prime = record
                .coefficient_context
                .characteristic
                .parse::<u64>()
                .unwrap();
            let resolution = matches!(
                record.computation,
                ModularComputationRecord::Resolution { .. }
            );
            let (_, group_index) = parse_label(&record.group_label);
            if sampled_paths.insert((order, prime, resolution))
                || ["2.1", "4.1", "4.2", "8.3", "8.4"].contains(&record.group_label.as_str())
            {
                samples.insert((order, prime, group_index), record);
            }
            samples
        });

    let script = hap_script(&samples);
    let mut child = Command::new(env::var_os("GAP").unwrap_or_else(|| "gap".into()))
        .args(["-q", "-b"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start GAP for HAP parity");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "GAP/HAP failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let observed = parse_hap_output(&String::from_utf8(output.stdout).unwrap());
    assert_eq!(observed.len(), samples.len(), "HAP omitted a sample bucket");
    for (&key, record) in &samples {
        assert_eq!(
            observed.get(&key),
            Some(&expected_dimensions(record)),
            "HAP parity mismatch for {} over F_{}",
            record.group_label,
            key.1
        );
    }
    eprintln!(
        "HAP parity passed for {} deterministic group/prime cases through degree {}",
        samples.len(),
        MAX_DEGREE
    );
}

fn expected_dimensions(record: &FiniteGroupModularCohomologyRecord) -> Vec<usize> {
    match &record.computation {
        ModularComputationRecord::FiniteAbelianClassifyingSpace { certificate } => {
            certificate.hilbert_function[..=MAX_DEGREE].to_vec()
        }
        ModularComputationRecord::Resolution { basis, .. } => {
            let mut dimensions = vec![0; MAX_DEGREE + 1];
            for element in basis.iter().filter(|element| element.degree <= MAX_DEGREE) {
                dimensions[element.degree] += 1;
            }
            dimensions
        }
    }
}

fn hap_script(
    samples: &BTreeMap<(usize, u64, usize), &FiniteGroupModularCohomologyRecord>,
) -> String {
    let mut script = String::from(
        "if LoadPackage(\"smallgrp\") <> true then Error(\"SmallGrp unavailable\"); fi;\n\
         if LoadPackage(\"hap\") <> true then Error(\"HAP unavailable\"); fi;\n",
    );
    for (&(order, prime, group_index), record) in samples {
        let (_, index) = parse_label(&record.group_label);
        script.push_str(&format!(
            "A := ModPCohomologyRing(SmallGroup({order},{index}), {prime}, {MAX_DEGREE});;\n\
             B := BasisVectors(Basis(A));;\n\
             Print(\"TYPELAKE|{order}|{prime}|{group_index}|\", List([0..{MAX_DEGREE}], d -> Number(B, x -> A!.degree(x) = d)), \"\\n\");\n\
             Unbind(A);; Unbind(B);; GASMAN(\"collect\");;\n"
        ));
    }
    script.push_str("QUIT;\n");
    script
}

fn parse_hap_output(output: &str) -> BTreeMap<(usize, u64, usize), Vec<usize>> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("TYPELAKE|"))
        .map(|line| {
            let mut fields = line.splitn(4, '|');
            let order = fields.next().unwrap().parse().unwrap();
            let prime = fields.next().unwrap().parse().unwrap();
            let group_index = fields.next().unwrap().parse().unwrap();
            let dimensions = fields
                .next()
                .unwrap()
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|value| value.trim().parse().unwrap())
                .collect();
            ((order, prime, group_index), dimensions)
        })
        .collect()
}

fn parse_label(label: &str) -> (usize, usize) {
    let (order, index) = label.split_once('.').unwrap();
    (order.parse().unwrap(), index.parse().unwrap())
}
