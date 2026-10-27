//! Task definitions: `tasks/<id>/task.toml`.
//!
//! A task is a coding problem with a known answer. It names where the
//! project comes from (an old koda commit, or a fixture directory), the
//! prompt the agent gets, and hidden tests the agent never sees. The
//! hidden tests are appended to a source file after the agent finishes,
//! then the check command decides pass or fail.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskFile {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub repo: RepoSpec,
    pub check: CheckSpec,
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoSpec {
    /// History task: the koda commit the agent starts from.
    pub koda_base: Option<String>,
    /// History task: the koda commit with the real fix (fail-first check).
    pub koda_fix: Option<String>,
    /// Kata: directory (relative to the task) the agent starts from.
    pub fixture: Option<String>,
    /// Kata: directory copied over the fixture to get a correct answer.
    pub solution: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckSpec {
    pub hidden: Vec<HiddenTest>,
    pub command: String,
    #[serde(default = "default_check_secs")]
    pub timeout_secs: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HiddenTest {
    /// Source file, relative to the project root, the tests are appended to.
    pub file: String,
    /// File holding the test functions, relative to the task directory.
    pub test: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_wall_secs")]
    pub wall_secs: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            wall_secs: default_wall_secs(),
        }
    }
}

fn default_check_secs() -> u64 {
    600
}

fn default_wall_secs() -> u64 {
    900
}

/// Where a task's project comes from.
#[derive(Debug)]
pub enum Source {
    History { base: String, fix: String },
    Fixture { dir: PathBuf, solution: PathBuf },
}

#[derive(Debug)]
pub struct Task {
    pub spec: TaskFile,
    pub dir: PathBuf,
    pub source: Source,
}

impl Task {
    pub fn load(dir: &Path) -> Result<Task> {
        let path = dir.join("task.toml");
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let spec: TaskFile =
            toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;

        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if spec.id != dir_name {
            bail!(
                "{}: id {:?} must match its directory name",
                path.display(),
                spec.id
            );
        }

        let source = match (&spec.repo.koda_base, &spec.repo.fixture) {
            (Some(base), None) => {
                let Some(fix) = spec.repo.koda_fix.clone() else {
                    bail!("{}: a koda_base task needs koda_fix", path.display());
                };
                Source::History {
                    base: base.clone(),
                    fix,
                }
            }
            (None, Some(fixture)) => {
                let Some(solution) = &spec.repo.solution else {
                    bail!(
                        "{}: a fixture task needs a solution directory",
                        path.display()
                    );
                };
                let (fixture, solution) = (dir.join(fixture), dir.join(solution));
                for d in [&fixture, &solution] {
                    if !d.is_dir() {
                        bail!("{}: {} is not a directory", path.display(), d.display());
                    }
                }
                Source::Fixture {
                    dir: fixture,
                    solution,
                }
            }
            _ => bail!(
                "{}: [repo] needs exactly one of koda_base or fixture",
                path.display()
            ),
        };

        if spec.check.hidden.is_empty() {
            bail!("{}: [check] needs at least one hidden test", path.display());
        }
        for hidden in &spec.check.hidden {
            let test = dir.join(&hidden.test);
            if !test.is_file() {
                bail!(
                    "{}: hidden test {} not found",
                    path.display(),
                    test.display()
                );
            }
        }

        Ok(Task {
            spec,
            dir: dir.to_path_buf(),
            source,
        })
    }

    pub fn id(&self) -> &str {
        &self.spec.id
    }
}

/// Load every task under `tasks_dir`, sorted by id.
pub fn load_all(tasks_dir: &Path) -> Result<Vec<Task>> {
    let entries = std::fs::read_dir(tasks_dir)
        .with_context(|| format!("read tasks directory {}", tasks_dir.display()))?;
    let mut tasks = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.join("task.toml").is_file() {
            tasks.push(Task::load(&path)?);
        }
    }
    tasks.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(tasks)
}

/// Pick tasks by id, or by tags (a task must carry every listed tag).
/// No ids and no tags selects everything.
pub fn select<'a>(all: &'a [Task], ids: &[String], tags: &[String]) -> Result<Vec<&'a Task>> {
    if !ids.is_empty() {
        return ids
            .iter()
            .map(|id| {
                all.iter()
                    .find(|t| t.id() == id)
                    .with_context(|| format!("no task with id {id:?}"))
            })
            .collect();
    }
    Ok(all
        .iter()
        .filter(|t| tags.iter().all(|tag| t.spec.tags.contains(tag)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn kata(root: &Path, id: &str, tags: &str) -> PathBuf {
        let dir = root.join(id);
        write(
            &dir,
            "task.toml",
            &format!(
                r#"id = "{id}"
prompt = "fix it"
tags = [{tags}]
[repo]
fixture = "fixture"
solution = "solution"
[check]
hidden = [{{ file = "src/lib.rs", test = "hidden/t.rs" }}]
command = "cargo test eval_hidden"
"#
            ),
        );
        write(&dir, "fixture/src/lib.rs", "");
        write(&dir, "solution/src/lib.rs", "");
        write(&dir, "hidden/t.rs", "#[test] fn t() {}");
        dir
    }

    #[test]
    fn loads_a_fixture_task_with_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = kata(tmp.path(), "k1", r#""kata""#);
        let task = Task::load(&dir).unwrap();
        assert_eq!(task.id(), "k1");
        assert_eq!(task.spec.limits.wall_secs, 900);
        assert_eq!(task.spec.check.timeout_secs, 600);
        assert!(matches!(task.source, Source::Fixture { .. }));
    }

    #[test]
    fn rejects_an_id_that_differs_from_the_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = kata(tmp.path(), "k1", "");
        std::fs::rename(&dir, tmp.path().join("other")).unwrap();
        let err = Task::load(&tmp.path().join("other")).unwrap_err();
        assert!(format!("{err:#}").contains("must match"), "{err:#}");
    }

    #[test]
    fn history_task_needs_a_fix_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("h1");
        write(
            &dir,
            "task.toml",
            r#"id = "h1"
prompt = "fix it"
[repo]
koda_base = "abc^"
[check]
hidden = [{ file = "src/lib.rs", test = "hidden/t.rs" }]
command = "cargo test eval_hidden"
"#,
        );
        write(&dir, "hidden/t.rs", "");
        let err = Task::load(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("koda_fix"), "{err:#}");
    }

    #[test]
    fn selects_by_id_or_by_every_tag() {
        let tmp = tempfile::tempdir().unwrap();
        kata(tmp.path(), "a", r#""rust", "kata""#);
        kata(tmp.path(), "b", r#""rust""#);
        let all = load_all(tmp.path()).unwrap();

        let ids = |v: Vec<&Task>| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>();
        assert_eq!(ids(select(&all, &[], &[]).unwrap()), ["a", "b"]);
        assert_eq!(ids(select(&all, &[], &["kata".into()]).unwrap()), ["a"]);
        assert_eq!(ids(select(&all, &["b".into()], &[]).unwrap()), ["b"]);
        assert!(select(&all, &["zzz".into()], &[]).is_err());
    }
}
