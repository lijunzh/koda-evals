//! A task's project on disk: a fresh single-commit git repo.
//!
//! History tasks are unpacked with `git archive`, never cloned or checked
//! out as a worktree. A clone keeps later commits reachable, and the agent
//! could `git log` its way to the real fix.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::SystemTime;

use anyhow::{Context, Result, bail};

use crate::task::{Source, Task};

#[derive(Clone, Copy, Debug)]
pub enum Stage {
    /// What the agent gets: the base commit, or the fixture.
    Start,
    /// A correct answer: the fix commit, or the fixture plus its solution.
    Solved,
}

/// The instructions file a run's project starts with: the run file's
/// `memory`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Memory {
    /// Whatever the checkout or fixture has.
    Checkout,
    /// None at all.
    Without,
    /// This text as `CLAUDE.md`, in place of whatever the checkout has.
    Pinned(String),
}

/// The files at a project's root that Koda or Claude Code read as the
/// project's instructions. The task checkouts have none deeper down.
pub const MEMORY_FILES: &[&str] = &[
    "MEMORY.md",
    "CLAUDE.md",
    "AGENTS.md",
    "CLAUDE.local.md",
    ".claude/CLAUDE.md",
];

/// Write the task's project for `stage` into `dest` (which must not exist)
/// and commit it, so the agent's changes show up as a diff.
pub fn materialize(task: &Task, stage: Stage, koda_repo: &Path, dest: &Path) -> Result<()> {
    materialize_with(task, stage, koda_repo, dest, &Memory::Checkout)
}

/// [`materialize`], with `memory` as the project's instructions. The change
/// is in the start commit, so it stays out of the diff.
pub fn materialize_with(
    task: &Task,
    stage: Stage,
    koda_repo: &Path,
    dest: &Path,
    memory: &Memory,
) -> Result<()> {
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
    if *memory != Memory::Checkout {
        for name in MEMORY_FILES {
            let path = dest.join(name);
            if path.is_file() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("remove {}", path.display()))?;
            }
        }
    }
    if let Memory::Pinned(text) = memory {
        std::fs::write(dest.join("CLAUDE.md"), text).context("write the pinned CLAUDE.md")?;
    }
    stamp(dest)?;
    git(dest, &["init", "-q", "-b", "main"])?;
    git(dest, &["add", "-A"])?;
    git(dest, &["commit", "-q", "--no-verify", "-m", "task start"])?;
    Ok(())
}

/// Give every file under `dir` the current time, so cargo compiles it.
///
/// Cargo calls a package up to date when its sources are older than its
/// last build in the target dir, wherever on disk that build's project
/// was. `git archive` writes commit times and `fs::copy` can keep the
/// source's, so a new project looks older than the last build: cargo prints
/// `Finished`, or runs the solved stage's test binary, for code it never
/// compiled.
pub(crate) fn stamp(dir: &Path) -> Result<()> {
    stamp_tree(dir, SystemTime::now())
}

fn stamp_tree(dir: &Path, now: SystemTime) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            stamp_tree(&entry.path(), now)?;
        } else if kind.is_file() {
            File::open(entry.path())
                .and_then(|f| f.set_modified(now))
                .with_context(|| format!("stamp {}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// Clone the task's shared cargo target dir to `target`, the run's own.
///
/// `stamp` alone isn't enough at `--jobs N`: a build another run of the
/// task starts after this project was written is newer than its files
/// (and than the file the hidden tests went into), so cargo would hand this
/// run that run's output. A clone keeps every run's packages to itself and
/// still starts it with the task's compiled dependencies, whose fingerprints
/// survive because `cp` keeps mtimes. Missing `shared` means a cold start.
pub fn seed_target(shared: &Path, target: &Path) -> Result<()> {
    if !shared.is_dir() {
        return Ok(());
    }
    let mut cp = Command::new("cp");
    if cfg!(target_os = "macos") {
        cp.arg("-cRp");
    } else {
        cp.args(["-a", "--reflink=auto"]);
    }
    let out = cp.arg(shared).arg(target).output().context("run cp")?;
    if !out.status.success() {
        bail!(
            "clone {}: {}",
            shared.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
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
    fn memory_is_kept_removed_or_pinned_in_the_start_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let task_dir = tmp.path().join("k");
        for (rel, text) in [
            ("fixture/src/lib.rs", "pub fn f() -> u8 { 1 }\n"),
            ("fixture/CLAUDE.md", "long checkout file\n"),
            ("fixture/AGENTS.md", "agents file\n"),
            ("fixture/.claude/CLAUDE.md", "nested\n"),
            ("solution/src/lib.rs", "pub fn f() -> u8 { 2 }\n"),
            ("hidden/t.rs", "#[test]\nfn two() { assert_eq!(f(), 2); }\n"),
        ] {
            let path = task_dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        std::fs::write(
            task_dir.join("task.toml"),
            "id = \"k\"\nprompt = \"p\"\n[repo]\nfixture = \"fixture\"\nsolution = \"solution\"\n\
             [check]\nhidden = [{ file = \"src/lib.rs\", test = \"hidden/t.rs\" }]\ncommand = \"true\"\n",
        )
        .unwrap();
        let task = Task::load(&task_dir).unwrap();
        let read = |dir: &Path, name: &str| std::fs::read_to_string(dir.join(name)).ok();

        let kept = tmp.path().join("kept");
        materialize_with(&task, Stage::Start, tmp.path(), &kept, &Memory::Checkout).unwrap();
        assert_eq!(
            read(&kept, "CLAUDE.md").as_deref(),
            Some("long checkout file\n")
        );

        let without = tmp.path().join("without");
        materialize_with(&task, Stage::Start, tmp.path(), &without, &Memory::Without).unwrap();
        for name in MEMORY_FILES {
            assert_eq!(read(&without, name), None, "{name}");
        }

        let pinned = tmp.path().join("pinned");
        let memory = Memory::Pinned("short file\n".into());
        materialize_with(&task, Stage::Start, tmp.path(), &pinned, &memory).unwrap();
        assert_eq!(read(&pinned, "CLAUDE.md").as_deref(), Some("short file\n"));
        assert_eq!(read(&pinned, "AGENTS.md"), None);
        assert_eq!(read(&pinned, ".claude/CLAUDE.md"), None);
        // In the start commit, so the agent's diff doesn't show it.
        assert_eq!(git(&pinned, &["status", "--porcelain"]).unwrap(), "");
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

    fn cargo(dir: &Path, target: &Path, args: &[&str]) -> (bool, String) {
        let out = Command::new("cargo")
            .args(args)
            .current_dir(dir)
            .env("CARGO_TARGET_DIR", target)
            .env("CARGO_TERM_COLOR", "never")
            .env_remove("RUSTFLAGS")
            .output()
            .unwrap();
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.success(), text)
    }

    const MANIFEST: &str = "[package]\nname = \"k\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";
    const BROKEN: &str = "pub fn f() -> u8 { \"one\" }\n";
    const FIXED: &str = "pub fn f() -> u8 { 2 }\n";

    /// Whatever `Stage::Solved` builds must not stand in for `Stage::Start`:
    /// the first build of the new project has to fail with the real error.
    fn assert_start_is_compiled_after_solved(task: &Task, koda_repo: &Path, tmp: &Path) {
        let target = tmp.join("target");
        let solved = tmp.join("solved");
        materialize(task, Stage::Solved, koda_repo, &solved).unwrap();
        inject_hidden(task, &solved).unwrap();
        // What `verify` leaves, then what a run that fixed the bug leaves.
        let (ok, text) = cargo(&solved, &target, &["test", "--lib"]);
        assert!(ok && text.contains("eval_hidden::two ... ok"), "{text}");
        let (ok, text) = cargo(&solved, &target, &["build"]);
        assert!(ok, "{text}");

        let start = tmp.join("start");
        materialize(task, Stage::Start, koda_repo, &start).unwrap();
        for args in [&["build"][..], &["test", "--lib"][..]] {
            let (ok, text) = cargo(&start, &target, args);
            assert!(!ok, "cargo {args:?} reused the solved build:\n{text}");
            assert!(text.contains("error[E0308]"), "{text}");
            assert!(!text.contains("eval_hidden"), "{text}");
        }
    }

    fn write_task_files(task_dir: &Path, toml: &str) {
        std::fs::create_dir_all(task_dir.join("hidden")).unwrap();
        std::fs::write(
            task_dir.join("hidden/t.rs"),
            "#[test]\nfn two() { assert_eq!(f(), 2); }\n",
        )
        .unwrap();
        std::fs::write(task_dir.join("task.toml"), toml).unwrap();
    }

    #[test]
    fn a_fixture_project_cannot_reuse_the_last_build_in_its_target() {
        let tmp = tempfile::tempdir().unwrap();
        let task_dir = tmp.path().join("k");
        for (rel, text) in [
            ("fixture/Cargo.toml", MANIFEST),
            ("fixture/src/lib.rs", BROKEN),
            ("solution/src/lib.rs", FIXED),
        ] {
            let path = task_dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        write_task_files(
            &task_dir,
            r#"id = "k"
prompt = "p"
[repo]
fixture = "fixture"
solution = "solution"
[check]
hidden = [{ file = "src/lib.rs", test = "hidden/t.rs" }]
command = "true"
"#,
        );
        let task = Task::load(&task_dir).unwrap();
        assert_start_is_compiled_after_solved(&task, tmp.path(), tmp.path());
    }

    #[test]
    fn a_history_project_cannot_reuse_the_last_build_in_its_target() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), MANIFEST).unwrap();
        std::fs::write(repo.join("src/lib.rs"), BROKEN).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]).unwrap();
        git(&repo, &["add", "-A"]).unwrap();
        git(&repo, &["commit", "-q", "--no-verify", "-m", "base"]).unwrap();
        git(&repo, &["tag", "base"]).unwrap();
        std::fs::write(repo.join("src/lib.rs"), FIXED).unwrap();
        git(&repo, &["commit", "-qam", "fix", "--no-verify"]).unwrap();

        let task_dir = tmp.path().join("k");
        write_task_files(
            &task_dir,
            r#"id = "k"
prompt = "p"
[repo]
koda_base = "base"
koda_fix = "main"
[check]
hidden = [{ file = "src/lib.rs", test = "hidden/t.rs" }]
command = "true"
"#,
        );
        let task = Task::load(&task_dir).unwrap();
        assert_start_is_compiled_after_solved(&task, &repo, tmp.path());
    }

    #[test]
    fn a_seeded_target_keeps_its_cached_builds_and_stays_apart() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join("src")).unwrap();
        std::fs::write(project.join("Cargo.toml"), MANIFEST).unwrap();
        std::fs::write(project.join("src/lib.rs"), FIXED).unwrap();
        let shared = tmp.path().join("shared");
        assert!(cargo(&project, &shared, &["build"]).0);

        let (one, two) = (tmp.path().join("one"), tmp.path().join("two"));
        seed_target(&shared, &one).unwrap();
        seed_target(&shared, &two).unwrap();
        let (ok, text) = cargo(&project, &one, &["build"]);
        assert!(ok && !text.contains("Compiling"), "cache lost:\n{text}");

        // A newer build in one clone can't make the other look up to date.
        stamp(&project).unwrap();
        let (ok, text) = cargo(&project, &one, &["build"]);
        assert!(ok && text.contains("Compiling"), "{text}");
        let (ok, text) = cargo(&project, &two, &["build"]);
        assert!(ok && text.contains("Compiling"), "{text}");

        let cold = tmp.path().join("cold");
        seed_target(&tmp.path().join("missing"), &cold).unwrap();
        assert!(!cold.exists());
    }

    #[test]
    fn stamp_renews_read_only_files_and_leaves_links_alone() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        use std::time::Duration;

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("p");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let file = dir.join("sub/f.rs");
        std::fs::write(&file, "x").unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        File::open(&file).unwrap().set_modified(old).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444)).unwrap();
        symlink("no-such-file", dir.join("dangling")).unwrap();

        stamp(&dir).unwrap();
        let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        assert!(mtime > old + Duration::from_secs(3000));
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
