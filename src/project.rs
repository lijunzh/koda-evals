//! A task's project on disk: a fresh single-commit git repo.
//!
//! History tasks are unpacked with `git archive`, never cloned or checked
//! out as a worktree. A clone keeps later commits reachable, and the agent
//! could `git log` its way to the real fix.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::task::{Source, Task};

#[derive(Clone, Copy, Debug)]
pub enum Stage {
    /// What the agent gets: the base commit, or the fixture.
    Start,
    /// A correct answer: the fix commit, or the fixture plus its solution.
    Solved,
}

/// Write the task's project for `stage` into `dest` (which must not exist)
/// and commit it, so the agent's changes show up as a diff.
pub fn materialize(task: &Task, stage: Stage, koda_repo: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        bail!("{} already exists", dest.display());
    }
    std::fs::create_dir_all(dest)?;
    match (&task.source, stage) {
        (Source::History { base, .. }, Stage::Start) => archive(koda_repo, base, dest)?,
        (Source::History { fix, .. }, Stage::Solved) => archive(koda_repo, fix, dest)?,
        (Source::Fixture { dir, .. }, Stage::Start) => copy_dir(dir, dest)?,
        (Source::Fixture { dir, solution }, Stage::Solved) => {
            copy_dir(dir, dest)?;
            copy_dir(solution, dest)?;
        }
    }
    // Worker agent configs go in `<project>/agents/`, koda's project-level
    // agents directory, and into the start commit so they stay out of the
    // diff. (Koda's user-level agents live under `$HOME/.config/koda`,
    // not under the run's `XDG_CONFIG_HOME`, so the fresh config directory
    // can't carry them.)
    if let Some(agents) = task.agents_dir() {
        let to = dest.join("agents");
        std::fs::create_dir_all(&to)?;
        copy_dir(&agents, &to).with_context(|| format!("copy agents from {}", agents.display()))?;
    }
    git(dest, &["init", "-q", "-b", "main"])?;
    git(dest, &["add", "-A"])?;
    git(dest, &["commit", "-q", "--no-verify", "-m", "task start"])?;
    Ok(())
}

/// Unpack `rev` of `repo` into `dest` with `git archive` (no history).
pub(crate) fn archive(repo: &Path, rev: &str, dest: &Path) -> Result<()> {
    let mut archive = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["archive", "--format=tar", rev])
        .stdout(Stdio::piped())
        .spawn()
        .context("spawn git archive")?;
    let stdout = archive.stdout.take().context("git archive stdout")?;
    let untar = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(dest)
        .stdin(stdout)
        .status()
        .context("run tar")?;
    let archived = archive.wait()?;
    if !archived.success() || !untar.success() {
        bail!("git archive {rev} from {} failed", repo.display());
    }
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&to)?;
            copy_dir(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)
                .with_context(|| format!("copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// Run git in `dir` with a neutral identity, ignoring the user's global
/// and system config (hooks, templates, commit signing).
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=koda-eval",
            "-c",
            "user.email=koda-eval@localhost",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .context("run git")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub struct DiffStat {
    pub files: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// Save everything the agent changed (new files included) as a patch, and
/// count it.
pub fn capture_diff(dir: &Path, patch: &Path) -> Result<DiffStat> {
    git(dir, &["add", "-A"])?;
    // A fixture without a Cargo.lock gets one from the agent's first
    // `cargo` call; that's cargo's footprint, not the agent's.
    let scope = ["--", ".", ":!Cargo.lock"];
    let mut show = vec!["diff", "--cached", "--binary"];
    show.extend(scope);
    std::fs::write(patch, git(dir, &show)?)?;
    let mut stat = vec!["diff", "--cached", "--numstat"];
    stat.extend(scope);
    Ok(parse_numstat(&git(dir, &stat)?))
}

fn parse_numstat(text: &str) -> DiffStat {
    let mut stat = DiffStat::default();
    for line in text.lines() {
        let mut cols = line.split('\t');
        let (Some(added), Some(removed)) = (cols.next(), cols.next()) else {
            continue;
        };
        stat.files += 1;
        // Binary files show "-" in both columns.
        stat.insertions += added.parse().unwrap_or(0);
        stat.deletions += removed.parse().unwrap_or(0);
    }
    stat
}

/// Append the task's hidden tests to their source files, one
/// `mod eval_hidden` per file, so they can reach private items through
/// `use super::*`.
pub fn inject_hidden(task: &Task, dir: &Path) -> Result<()> {
    let mut by_file: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for hidden in &task.spec.check.hidden {
        let path = task.dir.join(&hidden.test);
        let body = std::fs::read_to_string(&path)
            .with_context(|| format!("read hidden test {}", path.display()))?;
        by_file.entry(&hidden.file).or_default().push(body);
    }
    for (file, bodies) in by_file {
        let path = dir.join(file);
        let mut text = std::fs::read_to_string(&path)
            .with_context(|| format!("read {} to append hidden tests", path.display()))?;
        text.push_str(&hidden_module(&bodies));
        std::fs::write(&path, text)?;
    }
    Ok(())
}

fn hidden_module(bodies: &[String]) -> String {
    let mut module = String::from(
        "\n\n// Appended by koda-eval: hidden tests.\n\
         #[cfg(test)]\n\
         mod eval_hidden {\n    \
         #[allow(unused_imports)]\n    \
         use super::*;\n\n",
    );
    for body in bodies {
        module.push_str(body.trim_end());
        module.push_str("\n\n");
    }
    module.push_str("}\n");
    module
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numstat_counts_text_and_binary_files() {
        let stat = parse_numstat("3\t1\tsrc/lib.rs\n-\t-\tlogo.png\n10\t0\tnew.rs\n");
        assert_eq!(
            stat,
            DiffStat {
                files: 3,
                insertions: 13,
                deletions: 1
            }
        );
    }

    #[test]
    fn hidden_module_wraps_every_body() {
        let module = hidden_module(&["#[test]\nfn a() {}\n".into(), "#[test]\nfn b() {}".into()]);
        assert!(module.contains("#[cfg(test)]\nmod eval_hidden {"));
        assert!(module.contains("use super::*;"));
        assert!(module.contains("fn a() {}\n\n#[test]\nfn b() {}\n\n}\n"));
    }

    #[test]
    fn materialized_fixture_is_a_clean_single_commit_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let task_dir = tmp.path().join("k");
        for (rel, text) in [
            ("task.toml", ""),
            ("fixture/src/lib.rs", "pub fn f() -> u8 { 1 }\n"),
            ("solution/src/lib.rs", "pub fn f() -> u8 { 2 }\n"),
            ("hidden/t.rs", "#[test]\nfn two() { assert_eq!(f(), 2); }\n"),
        ] {
            let path = task_dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        std::fs::write(
            task_dir.join("task.toml"),
            r#"id = "k"
prompt = "p"
[repo]
fixture = "fixture"
solution = "solution"
[check]
hidden = [{ file = "src/lib.rs", test = "hidden/t.rs" }]
command = "true"
"#,
        )
        .unwrap();
        let task = Task::load(&task_dir).unwrap();

        let start = tmp.path().join("start");
        materialize(&task, Stage::Start, tmp.path(), &start).unwrap();
        assert_eq!(git(&start, &["status", "--porcelain"]).unwrap(), "");
        assert_eq!(
            git(&start, &["rev-list", "--count", "HEAD"])
                .unwrap()
                .trim(),
            "1"
        );

        let solved = tmp.path().join("solved");
        materialize(&task, Stage::Solved, tmp.path(), &solved).unwrap();
        let lib = std::fs::read_to_string(solved.join("src/lib.rs")).unwrap();
        assert!(lib.contains("{ 2 }"));

        std::fs::write(start.join("src/lib.rs"), "pub fn f() -> u8 { 2 }\n").unwrap();
        std::fs::write(start.join("notes.txt"), "new\n").unwrap();
        let patch = tmp.path().join("diff.patch");
        let stat = capture_diff(&start, &patch).unwrap();
        assert_eq!(stat.files, 2);
        assert!(
            std::fs::read_to_string(patch)
                .unwrap()
                .contains("+pub fn f() -> u8 { 2 }")
        );

        inject_hidden(&task, &start).unwrap();
        let lib = std::fs::read_to_string(start.join("src/lib.rs")).unwrap();
        assert!(lib.contains("mod eval_hidden"));
        assert!(lib.contains("fn two()"));
    }

    #[test]
    fn agents_dir_is_committed_with_the_project() {
        let tmp = tempfile::tempdir().unwrap();
        let task_dir = tmp.path().join("k");
        for (rel, text) in [
            ("fixture/src/lib.rs", "pub fn f() {}\n"),
            ("solution/src/lib.rs", "pub fn f() {}\n"),
            ("hidden/t.rs", ""),
            ("agents/worker.json", "{\"system_prompt\":\"do the work\"}"),
        ] {
            let path = task_dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        std::fs::write(
            task_dir.join("task.toml"),
            r#"id = "k"
prompt = "p"
[repo]
fixture = "fixture"
solution = "solution"
[check]
hidden = [{ file = "src/lib.rs", test = "hidden/t.rs" }]
command = "true"
[run]
agents_dir = "agents"
"#,
        )
        .unwrap();
        let task = Task::load(&task_dir).unwrap();
        let start = tmp.path().join("start");
        materialize(&task, Stage::Start, tmp.path(), &start).unwrap();
        assert!(start.join("agents/worker.json").is_file());
        assert_eq!(git(&start, &["status", "--porcelain"]).unwrap(), "");
        let stat = capture_diff(&start, &tmp.path().join("d.patch")).unwrap();
        assert_eq!(stat.files, 0);
    }
}
