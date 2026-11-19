use sha2::{Digest, Sha256};
use std::{env, fs, path::Path, process::Command};

fn command_output(program: &str, args: &[&str], directory: &Path) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(directory)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR");
    let repository = Path::new(&manifest).join("../..");
    let revision = command_output("git", &["rev-parse", "HEAD"], &repository)
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = command_output("git", &["status", "--porcelain"], &repository)
        .is_some_and(|status| !status.is_empty());
    let rustc = env::var("RUSTC")
        .ok()
        .and_then(|rustc| command_output(&rustc, &["--version"], &repository))
        .unwrap_or_else(|| "unknown".to_owned());
    let flint = command_output("pkg-config", &["--modversion", "flint"], &repository)
        .unwrap_or_else(|| "unknown".to_owned());
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());

    let mut source = Sha256::new();
    if let Some(paths) = command_output(
        "git",
        &["ls-files", "--cached", "--others", "--exclude-standard"],
        &repository,
    ) {
        for path in paths.lines() {
            let file = repository.join(path);
            if let Ok(bytes) = fs::read(&file) {
                source.update(path.as_bytes());
                source.update([0]);
                source.update((bytes.len() as u64).to_le_bytes());
                source.update(bytes);
                println!("cargo:rerun-if-changed={}", file.display());
            }
        }
    }
    println!(
        "cargo:rustc-env=TYPELAKE_SOURCE_SHA256={:x}",
        source.finalize()
    );
    println!("cargo:rustc-env=POSET_CENSUS_REVISION={revision}");
    println!("cargo:rustc-env=POSET_CENSUS_DIRTY={dirty}");
    println!("cargo:rustc-env=POSET_CENSUS_RUSTC={rustc}");
    println!("cargo:rustc-env=POSET_CENSUS_FLINT_VERSION={flint}");
    println!("cargo:rustc-env=POSET_CENSUS_TARGET={target}");
    println!("cargo:rustc-env=POSET_CENSUS_PROFILE={profile}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
}
