//! Run files (`runs/<name>.toml`) and the runner.
//!
//! A run is every task × variant × model × repeat, one at a time by default
//! (a local model serves one request at a time) or `--jobs N` at once for an
//! API model. Each one gets a fresh project and a fresh koda config
//! directory, runs `koda -p … --output-format stream-json` under a
//! wall-clock limit, and then the task's hidden tests decide pass or fail.
//! Runs share nothing but the per-task cargo target directory, which cargo
//! locks itself, so parallel runs are safe; their wall times are contended.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::project::{self, Stage};
use crate::task::{self, Task};
use crate::transcript::Transcript;
use crate::{build, check, keys, proc, util};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunFile {
    pub name: String,
    /// Task ids. Empty means every task carrying all of `tags`.
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "one")]
    pub repeats: u32,
    /// Wall-clock limit per run in seconds, overriding every task's
    /// `[limits] wall_secs`. For "worker conditions": a background model
    /// that nobody is waiting on gets longer than an interactive one.
    #[serde(default)]
    pub wall_secs: Option<u64>,
    pub variants: Vec<Variant>,
    pub models: Vec<Model>,
}

fn one() -> u32 {
    1
}

/// A koda to measure: either a prebuilt binary or a git ref in the koda
/// checkout, which is built once per commit (see `build.rs`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub name: String,
    /// Path to a built `koda` binary.
    pub bin: Option<String>,
    /// A branch, tag or commit in the koda checkout.
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
}

/// A variant with its binary in hand.
struct Built {
    name: String,
    bin: PathBuf,
    version: String,
    git_ref: Option<String>,
    sha: Option<String>,
}

/// API keys never go in a run file: koda reads them from the environment
/// (`ANTHROPIC_API_KEY` and so on), which every run inherits.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    /// Extra environment for this model's runs, e.g. `KODA_MOCK_RESPONSES`.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

pub struct Options {
    pub root: PathBuf,
    pub koda_repo: PathBuf,
    /// Koda keystore to copy API keys from (see `keys.rs`).
    pub keystore: PathBuf,
    pub keep_work: bool,
    /// Runs executed at once; 1 is sequential.
    pub jobs: usize,
}

/// Execute `run_path`; returns the results directory.
pub fn run(run_path: &Path, opts: &Options) -> Result<PathBuf> {
    let text = std::fs::read_to_string(run_path)
        .with_context(|| format!("read {}", run_path.display()))?;
    let run: RunFile =
        toml::from_str(&text).with_context(|| format!("parse {}", run_path.display()))?;
    validate(&run)?;

    let all = task::load_all(&opts.root.join("tasks"))?;
    let tasks = task::select(&all, &run.tasks, &run.tags)?;
    if tasks.is_empty() {
        bail!("run {:?} selects no tasks", run.name);
    }

    let mut variants = Vec::new();
    for variant in &run.variants {
        let (bin, sha) = match (&variant.bin, &variant.git_ref) {
            (Some(bin), None) => (util::expand(bin), None),
            (None, Some(git_ref)) => {
                let built = build::ensure(&opts.koda_repo, git_ref)
                    .with_context(|| format!("variant {:?}", variant.name))?;
                (built.bin, Some(built.sha))
            }
            _ => bail!("variant {:?} needs exactly one of bin or ref", variant.name),
        };
        let version = command_line(Command::new(&bin).arg("--version")).with_context(|| {
            format!(
                "variant {:?}: run {} --version",
                variant.name,
                bin.display()
            )
        })?;
        variants.push(Built {
            name: variant.name.clone(),
            bin,
            version,
            git_ref: variant.git_ref.clone(),
            sha,
        });
    }

    // Keys the runs inherit from koda's keystore. Names only ever reach
    // the log and meta.json; the values go straight into the child's
    // environment.
    let inherited = keys::keys_for_child(&opts.keystore)?;
    let key_names: Vec<&String> = inherited.keys().collect();
    if !key_names.is_empty() {
        eprintln!(
            "keys from koda keystore {}: {}",
            opts.keystore.display(),
            key_names
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for (name, value) in &inherited {
        if keys::looks_wrong(name, value) {
            eprintln!(
                "warning: the keystore value for {name} doesn't look like an API key \
                 (whitespace, too short, or the wrong prefix); fix it with koda's /key command"
            );
        }
    }

    let stamp = util::utc_stamp();
    let out = opts.root.join("results").join(&run.name).join(&stamp);
    std::fs::create_dir_all(&out)?;
    let meta = json!({
        "run": run.name,
        "started": stamp,
        "keys_from_keystore": key_names,
        "tasks": tasks.iter().map(|t| t.id()).collect::<Vec<_>>(),
        "variants": variants.iter().map(|v| json!({
            "name": v.name, "bin": v.bin, "version": v.version,
            "ref": v.git_ref, "sha": v.sha,
        })).collect::<Vec<_>>(),
        // Task projects have no toolchain pin of their own, so they build
        // with rustup's default.
        "rustc": command_line(Command::new("rustc").arg("--version").current_dir(util::home())).ok(),
        "jobs": opts.jobs,
        "run_file": text,
    });
    std::fs::write(out.join("meta.json"), serde_json::to_string_pretty(&meta)?)?;

    let work_root = util::cache_dir()
        .join("work")
        .join(format!("{}-{stamp}", run.name));
    let mut results = File::create(out.join("results.jsonl"))?;

    // Every run, in the order a sequential pass would take them.
    let mut jobs = Vec::new();
    for task in &tasks {
        for variant in &variants {
            for model in &run.models {
                for repeat in 1..=run.repeats {
                    let slot = format!("{}.{}.r{repeat}", variant.name, model.name);
                    let art = out.join(task.id()).join(&slot);
                    std::fs::create_dir_all(&art)?;
                    let work = work_root.join(format!("{}.{slot}", task.id()));
                    jobs.push(Job {
                        n: jobs.len() + 1,
                        task,
                        variant,
                        model,
                        repeat,
                        slot,
                        art,
                        work,
                        wall_secs: run.wall_secs.unwrap_or(task.spec.limits.wall_secs),
                    });
                }
            }
        }
    }
    let total = jobs.len();
    let mut tally: BTreeMap<(String, String), (u32, u32)> = BTreeMap::new();

    let mut finish = |job: &Job, mut row: Value| -> Result<()> {
        row["artifacts"] = json!(job.art.strip_prefix(&out).unwrap_or(&job.art));
        writeln!(results, "{row}")?;
        results.flush()?;
        if let Some(pass) = row["pass"].as_bool() {
            let entry = tally
                .entry((job.variant.name.clone(), job.model.name.clone()))
                .or_default();
            entry.0 += u32::from(pass);
            entry.1 += 1;
        }
        if !opts.keep_work {
            let _ = std::fs::remove_dir_all(&job.work);
        }
        Ok(())
    };

    if opts.jobs <= 1 {
        for job in &jobs {
            eprint!("[{}/{total}] {} {} ... ", job.n, job.task.id(), job.slot);
            let row = job.execute(opts, &inherited);
            eprintln!("{}", describe(&row));
            finish(job, row)?;
        }
    } else {
        // A pool of `jobs` workers pulling from one queue; the main thread
        // records rows as they complete, so results.jsonl is in completion
        // order. Rows carry their own task/variant/model/repeat, and the
        // report never assumes an order.
        let workers = opts.jobs.min(total);
        eprintln!("running {total} runs, {workers} at a time");
        let next = std::sync::atomic::AtomicUsize::new(0);
        let (tx, rx) = std::sync::mpsc::channel::<(usize, Value)>();
        std::thread::scope(|scope| -> Result<()> {
            for _ in 0..workers {
                let tx = tx.clone();
                let (jobs, next, inherited) = (&jobs, &next, &inherited);
                scope.spawn(move || {
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let Some(job) = jobs.get(i) else { break };
                        eprintln!(
                            "[{}/{total}] {} {} ... started",
                            job.n,
                            job.task.id(),
                            job.slot
                        );
                        let row = job.execute(opts, inherited);
                        if tx.send((i, row)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(tx);
            for (i, row) in rx {
                let job = &jobs[i];
                eprintln!(
                    "[{}/{total}] {} {} ... {}",
                    job.n,
                    job.task.id(),
                    job.slot,
                    describe(&row)
                );
                finish(job, row)?;
            }
            Ok(())
        })?;
    }

    for ((variant, model), (passed, ran)) in &tally {
        println!("{variant} / {model}: {passed} of {ran} passed");
    }
    Ok(out)
}

/// One task × variant × model × repeat, with its directories decided.
struct Job<'a> {
    /// 1-based position in the sequential order, for the progress lines.
    n: usize,
    task: &'a Task,
    variant: &'a Built,
    model: &'a Model,
    repeat: u32,
    slot: String,
    art: PathBuf,
    work: PathBuf,
    /// The effective wall-clock limit: the run file's override, else the
    /// task's own.
    wall_secs: u64,
}

impl Job<'_> {
    /// Run it and return its results row. A harness failure (not the
    /// agent's) becomes a row with `pass: null`, so it stays out of the
    /// pass counts but is not lost.
    fn execute(&self, opts: &Options, keys: &BTreeMap<String, String>) -> Value {
        let one = RunOne {
            task: self.task,
            variant: &self.variant.name,
            bin: &self.variant.bin,
            model: self.model,
            repeat: self.repeat,
            art: &self.art,
            work: &self.work,
            koda_repo: &opts.koda_repo,
            keys,
            wall_secs: self.wall_secs,
        };
        match one.run() {
            Ok(row) => row,
            Err(e) => json!({
                "task": self.task.id(), "variant": self.variant.name,
                "model": self.model.name, "repeat": self.repeat,
                "pass": null, "harness_error": format!("{e:#}"),
            }),
        }
    }
}

fn validate(run: &RunFile) -> Result<()> {
    if run.variants.is_empty() || run.models.is_empty() {
        bail!(
            "run {:?} needs at least one variant and one model",
            run.name
        );
    }
    if run.repeats == 0 {
        bail!("run {:?}: repeats must be at least 1", run.name);
    }
    let mut names: Vec<&str> = run.variants.iter().map(|v| v.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    if names.len() != run.variants.len() {
        bail!("run {:?}: variant names must be unique", run.name);
    }
    let mut names: Vec<&str> = run.models.iter().map(|m| m.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    if names.len() != run.models.len() {
        bail!("run {:?}: model names must be unique", run.name);
    }
    Ok(())
}

struct RunOne<'a> {
    task: &'a Task,
    variant: &'a str,
    bin: &'a Path,
    model: &'a Model,
    repeat: u32,
    art: &'a Path,
    work: &'a Path,
    koda_repo: &'a Path,
    /// API keys copied from koda's keystore, as `ENV_NAME -> value`.
    keys: &'a BTreeMap<String, String>,
    /// The effective wall-clock limit.
    wall_secs: u64,
}

impl RunOne<'_> {
    fn run(&self) -> Result<Value> {
        let task = self.task;
        let project = self.work.join("project");
        let config = self.work.join("config");
        std::fs::create_dir_all(&config)?;
        project::materialize(task, Stage::Start, self.koda_repo, &project)?;
        let target = util::cache_dir().join("target").join(task.id());

        let transcript_path = self.art.join("transcript.ndjson");
        let mut cmd = Command::new(self.bin);
        cmd.arg("-p")
            .arg(&task.spec.prompt)
            .args(["--output-format", "stream-json", "--mode", "auto"])
            .args(["--provider", &self.model.provider])
            .arg("--project-root")
            .arg(&project)
            .current_dir(&project)
            // A fresh config dir: its own database (so no stored keys and
            // no session history), no global memory, no user skills.
            .env("XDG_CONFIG_HOME", &config)
            .env("CARGO_TARGET_DIR", &target)
            // The run file, not the shell, picks the model and mode.
            .env_remove("KODA_PROVIDER")
            .env_remove("KODA_MODEL")
            .env_remove("KODA_BASE_URL")
            .env_remove("KODA_MODE")
            .envs(self.keys)
            .envs(&self.model.env)
            .stdin(Stdio::null())
            .stdout(File::create(&transcript_path)?)
            .stderr(File::create(self.art.join("stderr.log"))?);
        if let Some(model) = &self.model.model {
            cmd.args(["--model", model]);
        }
        if let Some(url) = &self.model.base_url {
            cmd.args(["--base-url", url]);
        }
        let agent = proc::run_with_limit(&mut cmd, Duration::from_secs(self.wall_secs))?;
        // koda's own log says why a turn failed ("LLM inference failed" on
        // stderr doesn't). Keep it with the run's other evidence.
        keep_koda_log(&config, self.art);

        let transcript = Transcript::read(&transcript_path)?;
        let diff = project::capture_diff(&project, &self.art.join("diff.patch"))?;
        let hidden = if task.has_hidden() {
            Some(match project::inject_hidden(task, &project) {
                Ok(()) => check::run(task, &project, &target, &self.art.join("check.log"))?,
                Err(e) => check::not_run(&format!("hidden tests not added: {e:#}")),
            })
        } else {
            None
        };
        let check = check::evaluate(task, hidden, &transcript);

        // When koda itself failed (no model call, a provider error, a
        // crash), say so instead of blaming the untouched code.
        let agent_failed =
            agent.timed_out || agent.exit != Some(0) || transcript.turn_end_kind() == Some("error");
        let failure = if check.passed() {
            None
        } else if agent_failed && !agent.timed_out {
            Some("agent failed".to_string())
        } else {
            Some(check.failure_kind())
        };

        Ok(json!({
            "task": task.id(),
            "variant": self.variant,
            "model": self.model.name,
            "repeat": self.repeat,
            "pass": check.passed(),
            "failure": failure,
            "agent": {
                "exit": agent.exit,
                "timed_out": agent.timed_out,
                "wall_ms": agent.elapsed.as_millis() as u64,
            },
            "summary": transcript.summary,
            "events": transcript.counts,
            "diff": diff,
            "check": check,
        }))
    }
}

/// Copy koda's log file(s) from the run's config dir into `art/koda.log`.
/// Best effort: a missing log is not an error.
fn keep_koda_log(config: &Path, art: &Path) {
    let logs = config.join("koda").join("logs");
    let Ok(entries) = std::fs::read_dir(&logs) else {
        return;
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("koda-")
            && name.ends_with(".log")
            && let Ok(bytes) = std::fs::read(entry.path())
        {
            out.extend_from_slice(&bytes);
        }
    }
    if !out.is_empty() {
        let _ = std::fs::write(art.join("koda.log"), out);
    }
}

/// One line for the terminal: outcome, time, tokens.
fn describe(row: &Value) -> String {
    if let Some(err) = row["harness_error"].as_str() {
        return format!("HARNESS ERROR: {err}");
    }
    let outcome = match row["pass"].as_bool() {
        Some(true) => "PASS".to_string(),
        _ => format!("FAIL ({})", row["failure"].as_str().unwrap_or("?")),
    };
    let secs = row["agent"]["wall_ms"].as_u64().unwrap_or(0) / 1000;
    let usage = &row["summary"]["usage"];
    let tokens = usage["prompt_tokens"].as_u64().unwrap_or(0)
        + usage["completion_tokens"].as_u64().unwrap_or(0);
    let timed_out = if row["agent"]["timed_out"].as_bool() == Some(true) {
        ", agent hit the time limit"
    } else {
        ""
    };
    format!(
        "{outcome}  {}m{:02}s  {tokens} tokens{timed_out}",
        secs / 60,
        secs % 60
    )
}

fn command_line(cmd: &mut Command) -> Result<String> {
    let out = cmd.output()?;
    if !out.status.success() {
        bail!("exited with {}", out.status);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_file_parses_with_defaults() {
        let run: RunFile = toml::from_str(
            r#"name = "smoke"
tags = ["kata"]
[[variants]]
name = "main"
bin = "~/repo/koda/target/release/koda"
[[models]]
name = "local"
provider = "lm-studio"
base_url = "http://localhost:1234/v1"
"#,
        )
        .unwrap();
        assert_eq!(run.repeats, 1);
        assert!(run.models[0].env.is_empty());
        validate(&run).unwrap();
    }

    #[test]
    fn run_file_wall_secs_defaults_to_none_and_parses() {
        let run: RunFile = toml::from_str(
            r#"
name = "x"
[[variants]]
name = "main"
[[models]]
name = "m"
provider = "anthropic"
"#,
        )
        .unwrap();
        assert_eq!(run.wall_secs, None);
        let run: RunFile = toml::from_str(
            r#"
name = "x"
wall_secs = 3600
[[variants]]
name = "main"
[[models]]
name = "m"
provider = "anthropic"
"#,
        )
        .unwrap();
        assert_eq!(run.wall_secs, Some(3600));
    }

    #[test]
    fn duplicate_variant_names_are_rejected() {
        let run: RunFile = toml::from_str(
            r#"name = "x"
[[variants]]
name = "a"
bin = "k"
[[variants]]
name = "a"
bin = "k2"
[[models]]
name = "m"
provider = "mock"
"#,
        )
        .unwrap();
        assert!(validate(&run).is_err());
    }

    #[test]
    fn describes_a_row() {
        let row = json!({
            "pass": false, "failure": "hidden tests failed",
            "agent": {"wall_ms": 125_000, "timed_out": false},
            "summary": {"usage": {"prompt_tokens": 1000, "completion_tokens": 200}},
        });
        assert_eq!(
            describe(&row),
            "FAIL (hidden tests failed)  2m05s  1200 tokens"
        );
    }
}
