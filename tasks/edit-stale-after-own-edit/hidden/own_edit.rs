use crate::tools::ToolRegistry;
use serde_json::json;

fn edit_args(old: &str, new: &str) -> serde_json::Value {
    json!({
        "file_path": "f.txt",
        "replacements": [{"old_str": old, "new_str": new}]
    })
}

/// Pre-flight validation as the dispatcher runs it, then the Edit itself.
/// Ok when the Edit went through, Err with the rejection when it didn't.
async fn edit(
    registry: &ToolRegistry,
    root: &std::path::Path,
    old: &str,
    new: &str,
) -> Result<(), String> {
    let args = edit_args(old, new);
    if let Some(rejected) = validate_with_registry(registry, "Edit", &args, root).await {
        return Err(rejected);
    }
    let result = registry
        .execute("Edit", &args.to_string(), None, None)
        .await;
    if result.success {
        Ok(())
    } else {
        Err(result.output)
    }
}

/// A registry over a temp dir holding `f.txt`, already fully read.
async fn after_full_read(content: &str) -> (ToolRegistry, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("f.txt"), content).unwrap();
    let registry = ToolRegistry::new(dir.path().to_path_buf(), 100_000);
    let read = registry
        .execute("Read", r#"{"file_path":"f.txt"}"#, None, None)
        .await;
    assert!(read.success, "{}", read.output);
    // Filesystem mtimes can be coarse; make sure later writes get a new one.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    (registry, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edits_in_a_row_are_not_rejected() {
    let (registry, dir) = after_full_read("one\ntwo\nthree\n").await;
    edit(&registry, dir.path(), "one", "1").await.unwrap();
    edit(&registry, dir.path(), "two", "2").await.unwrap();
    edit(&registry, dir.path(), "three", "3").await.unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "1\n2\n3\n"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn outside_change_after_read_is_rejected() {
    let (registry, dir) = after_full_read("one\ntwo\nthree\n").await;
    std::fs::write(dir.path().join("f.txt"), "one\nTWO\nthree\n").unwrap();

    let result = edit(&registry, dir.path(), "three", "3").await;

    assert!(result.is_err(), "edit went through over an outside change");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "one\nTWO\nthree\n"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn outside_change_after_own_edit_is_rejected() {
    let (registry, dir) = after_full_read("one\ntwo\nthree\n").await;
    edit(&registry, dir.path(), "one", "1").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    std::fs::write(dir.path().join("f.txt"), "1\nTWO\nthree\n").unwrap();

    let result = edit(&registry, dir.path(), "three", "3").await;

    assert!(result.is_err(), "edit went through over an outside change");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "1\nTWO\nthree\n"
    );
}
