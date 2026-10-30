//! A run's first build compiles the run's own code. Driven by a stand-in
//! agent that only calls cargo, so no model or koda binary is involved.

use std::path::Path;
use std::process::Command;

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), to).unwrap();
        }
    }
}

fn eval(root: &Path, args: &[&str]) {
    let out = Command::new(env!("CARGO_BIN_EXE_koda-eval"))
        .arg("--root")
        .arg(root)
        .args(args)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "koda-eval {args:?}:\n{stderr}");
}

#[test]
fn the_first_cargo_test_after_verify_sees_the_real_error() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tasks/kata-largest"),
        &root.join("tasks/kata-largest"),
    );

    let agent = root.join("agent");
    std::fs::write(
        &agent,
        "#!/bin/sh\n\
         [ \"$1\" = --version ] && { echo stand-in; exit 0; }\n\
         cargo test --lib >>\"$PROBE\" 2>&1\n\
         echo \"target=$CARGO_TARGET_DIR\" >>\"$PROBE\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).unwrap();
    let probe = root.join("probe.log");
    let run_file = root.join("fresh.toml");
    std::fs::write(
        &run_file,
        format!(
            r#"name = "fresh"
tasks = ["kata-largest"]

[[variants]]
name = "agent"
bin = {agent:?}

[[models]]
name = "none"
provider = "mock"
env = {{ PROBE = {probe:?} }}
"#
        ),
    )
    .unwrap();

    // Cold and unverified: the run compiles, and leaves its target behind.
    let shared = root.join("cache/koda-eval/target/kata-largest");
    eval(root, &["run", run_file.to_str().unwrap()]);
    assert!(
        shared.join("debug").is_dir(),
        "no cache left for the next run"
    );
    std::fs::remove_file(&probe).unwrap();

    // `verify` leaves the solved stage's test binary in the task's cache.
    eval(root, &["verify", "kata-largest"]);
    eval(root, &["run", run_file.to_str().unwrap()]);
    let seen = std::fs::read_to_string(&probe).unwrap();
    assert!(seen.contains("error[E0369]"), "{seen}");
    assert!(!seen.contains("eval_hidden"), "{seen}");
    assert!(
        !seen.contains(&format!("target={}", shared.display())),
        "{seen}"
    );
}
