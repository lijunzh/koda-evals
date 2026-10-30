//! `verify` leaves no solution or hidden test where an agent could read it.

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

fn verify(root: &Path) -> bool {
    let out = Command::new(env!("CARGO_BIN_EXE_koda-eval"))
        .arg("--root")
        .arg(root)
        .args(["verify", "kata-largest"])
        .env("XDG_CACHE_HOME", root.join("cache"))
        .output()
        .unwrap();
    out.status.success()
}

#[test]
fn a_fair_task_leaves_nothing_and_an_unfair_one_only_logs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let task = root.join("tasks/kata-largest");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tasks/kata-largest"),
        &task,
    );
    let work = root.join("cache/koda-eval/verify/kata-largest");

    assert!(verify(root), "kata-largest should be fair");
    assert!(!work.exists());

    // A solution that still doesn't compile makes the task unfair.
    std::fs::copy(
        task.join("fixture/src/lib.rs"),
        task.join("solution/src/lib.rs"),
    )
    .unwrap();
    assert!(!verify(root), "a broken solution should be unfair");
    let mut left: Vec<String> = std::fs::read_dir(&work)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(left, ["solved.log", "start.log"]);
}
