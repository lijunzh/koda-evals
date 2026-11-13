//! Run files (`runs/<name>.toml`) and the runner.
//!
//! A run is every task × variant × model × repeat, one at a time by default
//! or several at once, from the run file's `jobs` or `--jobs N`. Each one gets a fresh project and a fresh koda config
//! directory, runs `koda -p … --output-format stream-json` under a
//! wall-clock limit, and then the task's hidden tests decide pass or fail.
//! Runs share nothing: each one's cargo target directory is a clone of the
//! task's cache (see `project::seed_target`), so dependencies compile once
//! and no run sees another's build. Their wall times are contended.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::project::{self, Memory, Stage};
use crate::task::{self, Task};
use crate::transcript::Transcript;
use crate::{build, check, keys, klog, proc, util};

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
    /// Koda trust mode for every task, overriding each task's `[run]
    /// trust`: to run tasks written for `auto` read-only, as `plan`.
    #[serde(default)]
    pub trust: Option<String>,
    /// The project's instructions file. `"checkout"` (the default) keeps
    /// whatever the task's project has; `"none"` removes it; a path,
    /// relative to this repo, puts that file in its place as `CLAUDE.md`.
    /// See `project::MEMORY_FILES` for what counts.
    #[serde(default)]
    pub memory: Option<String>,
    /// Runs to execute at once. `--jobs` overrides it. Wall times in the
    /// report are contended above 1.
    #[serde(default)]
    pub jobs: Option<usize>,
    pub variants: Vec<Variant>,
    pub models: Vec<Model>,
}

fn one() -> u32 {
    1
}

/// The run file's `memory`, read: what the projects get, and the label the
/// rows and the report carry (`checkout`, `none`, or the file's path).
fn resolve_memory(value: Option<&str>, root: &Path) -> Result<(Memory, String)> {
    match value.unwrap_or("checkout") {
        "checkout" => Ok((Memory::Checkout, "checkout".into())),
        "none" => Ok((Memory::Without, "none".into())),
        path => {
            let text = std::fs::read_to_string(root.join(path))
                .with_context(|| format!("memory: read {path}"))?;
            if text.trim().is_empty() {
                bail!("memory: {path} is empty; use \"none\" for no instructions");
            }
            Ok((Memory::Pinned(text), path.to_string()))
        }
    }
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
    /// What serves the model, for the row's configuration identity.
    /// Default: the provider, plus the `base_url` host when there is one
    /// (`lm-studio@localhost:1234`). Set it when that isn't the engine,
    /// e.g. two servers behind one proxy.
    pub engine: Option<String>,
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
    /// `--jobs`: runs executed at once, over the run file's `jobs`.
    pub jobs: Option<usize>,
}

/// Execute `run_path`; returns the results directory.
pub fn run(run_path: &Path, opts: &Options) -> Result<PathBuf> {
    let text = std::fs::read_to_string(run_path)
        .with_context(|| format!("read {}", run_path.display()))?;
    let run: RunFile =
        toml::from_str(&text).with_context(|| format!("parse {}", run_path.display()))?;
    validate(&run)?;
    let (memory, memory_label) = resolve_memory(run.memory.as_deref(), &opts.root)?;
    // Koda reads its global memory from `$HOME`, not from the run's
    // `XDG_CONFIG_HOME`, so this file would reach every run unseen.
    let global = util::home().join(".config/koda/memory.md");
    if global.exists() {
        bail!(
            "{} exists, and every run's koda would load it as global memory; \
             move it aside while the run goes",
            global.display()
        );
    }
    let concurrency = opts.jobs.or(run.jobs).unwrap_or(1);

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
        "jobs": concurrency,
        "memory": memory_label,
        // The pinned file itself, since it can change after the run.
        "memory_text": match &memory {
            Memory::Pinned(text) => Some(text),
            _ => None,
        },
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
                        trust: run.trust.as_deref().unwrap_or(&task.spec.run.trust),
                        memory: &memory,
                        memory_label: &memory_label,
                        concurrency,
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
    // Set by the first failed cleanup. What that run left may still be
    // running, so nothing starts or is graded after it.
    let stop = AtomicBool::new(false);

    if concurrency <= 1 {
        for job in &jobs {
            eprint!("[{}/{total}] {} {} ... ", job.n, job.task.id(), job.slot);
            let row = job.execute(opts, &inherited, &stop);
            eprintln!("{}", describe(&row));
            finish(job, row)?;
            if stop.load(Ordering::SeqCst) {
                break;
            }
        }
    } else {
        // A pool of `jobs` workers pulling from one queue; the main thread
        // records rows as they complete, so results.jsonl is in completion
        // order. Rows carry their own task/variant/model/repeat, and the
        // report never assumes an order.
        let workers = concurrency.min(total);
        eprintln!("running {total} runs, {workers} at a time");
        let next = std::sync::atomic::AtomicUsize::new(0);
        let (tx, rx) = std::sync::mpsc::channel::<(usize, Value)>();
        std::thread::scope(|scope| -> Result<()> {
            for _ in 0..workers {
                let tx = tx.clone();
                let (jobs, next, inherited, stop) = (&jobs, &next, &inherited, &stop);
                scope.spawn(move || {
                    while !stop.load(Ordering::SeqCst) {
                        let i = next.fetch_add(1, Ordering::SeqCst);
                        let Some(job) = jobs.get(i) else { break };
                        eprintln!(
                            "[{}/{total}] {} {} ... started",
                            job.n,
                            job.task.id(),
                            job.slot
                        );
                        let row = job.execute(opts, inherited, stop);
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
    if stop.load(Ordering::SeqCst) {
        bail!("stopping the run: a cleanup failed, see harness_error in results.jsonl");
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
    /// The effective trust mode: the run file's override, else the task's.
    trust: &'a str,
    memory: &'a Memory,
    memory_label: &'a str,
    /// Runs executed at once, recorded on the row.
    concurrency: usize,
}

impl Job<'_> {
    /// Run it and return its results row. A harness failure (not the
    /// agent's) becomes a row with `pass: null`, so it stays out of the
    /// pass counts but is not lost.
    fn execute(&self, opts: &Options, keys: &BTreeMap<String, String>, stop: &AtomicBool) -> Value {
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
            trust: self.trust,
            jobs: self.concurrency,
            memory: self.memory,
            memory_label: self.memory_label,
            stop,
        };
        match one.run() {
            // Grading can't be interrupted, so what a failed cleanup left may
            // have run alongside it. Keep the evidence, but not the score.
            Ok(mut row) if stop.load(Ordering::SeqCst) => {
                row["pass"] = Value::Null;
                row["harness_error"] = json!("not trusted: a cleanup failed during this run");
                row
            }
            Ok(row) => row,
            Err(e) => {
                if e.is::<proc::CleanupFailed>() {
                    stop.store(true, Ordering::SeqCst);
                }
                json!({
                    "task": self.task.id(), "variant": self.variant.name,
                    "model": self.model.name, "repeat": self.repeat,
                    "pass": null, "harness_error": format!("{e:#}"),
                })
            }
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
    if run.jobs == Some(0) {
        bail!("run {:?}: jobs must be at least 1", run.name);
    }
    if let Some(trust) = &run.trust
        && !crate::task::TRUST_MODES.contains(&trust.as_str())
    {
        bail!(
            "run {:?}: trust {trust:?} must be one of {}",
            run.name,
            crate::task::TRUST_MODES.join(", ")
        );
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
    /// The effective trust mode.
    trust: &'a str,
    /// Runs executed at once, for the configuration identity.
    jobs: usize,
    /// The project's instructions, and their label for the row.
    memory: &'a Memory,
    memory_label: &'a str,
    /// Set when any run's cleanup failed.
    stop: &'a AtomicBool,
}

impl RunOne<'_> {
    fn run(&self) -> Result<Value> {
        let task = self.task;
        let project = self.work.join("project");
        let config = self.work.join("config");
        std::fs::create_dir_all(&config)?;
        let shared = util::cache_dir().join("target").join(task.id());
        let target = self.work.join("target");
        project::seed_target(&shared, &target)?;
        project::materialize_with(task, Stage::Start, self.koda_repo, &project, self.memory)?;

        let transcript_path = self.art.join("transcript.ndjson");
        let trust = self.trust;
        let mut cmd = Command::new(self.bin);
        cmd.arg("-p")
            .arg(&task.spec.prompt)
            .args(["--output-format", "stream-json", "--mode", trust])
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
            // Debug text is the evidence for a failed row; a model's own
            // `env` can still override this.
            .env("RUST_LOG", "koda_core=debug,koda_cli=info")
            .envs(self.keys)
            .envs(&self.model.env)
            .envs(
                task.spec
                    .run
                    .max_context_tokens
                    .map(|n| ("KODA_MAX_CONTEXT_TOKENS", n.to_string())),
            )
            .stdin(Stdio::null())
            .stdout(File::create(&transcript_path)?)
            .stderr(File::create(self.art.join("stderr.log"))?);
        if let Some(model) = &self.model.model {
            cmd.args(["--model", model]);
        }
        if let Some(url) = &self.model.base_url {
            cmd.args(["--base-url", url]);
        }
        // Outside the project, so the agent's file tools never list it.
        if let Some(mcp) = task.mcp_config()? {
            let path = self.work.join("mcp.json");
            std::fs::write(&path, serde_json::to_string_pretty(&mcp)?)?;
            cmd.arg("--mcp-config").arg(&path);
        }
        let agent = proc::run_with_limit(&mut cmd, Duration::from_secs(self.wall_secs))?;
        // koda's own log says why a turn failed ("LLM inference failed" on
        // stderr doesn't). Keep it with the run's other evidence.
        keep_koda_log(&config, self.art);
        keep_koda_db(&config, self.art);

        // Koda's stream is read once, here; grading and regrading use the
        // record.
        let transcript = Transcript::read(&transcript_path)?;
        transcript.save(&self.art.join(crate::transcript::RECORD_FILE))?;
        let diff = project::capture_diff(&project, &self.art.join("diff.patch"))?;
        if self.stop.load(Ordering::SeqCst) {
            bail!("not graded: another run's cleanup failed");
        }
        let hidden = if task.has_hidden() {
            Some(match project::inject_hidden(task, &project) {
                Ok(()) => check::run(task, &project, &target, &self.art.join("check.log"))?,
                Err(e) => check::not_run(&format!("hidden tests not added: {e:#}")),
            })
        } else {
            None
        };
        let check = check::evaluate(task, hidden, &transcript, Some(&diff));

        // A task nobody verified starts cold; its first finished run leaves
        // the compiled dependencies for the others.
        if !shared.exists() {
            std::fs::create_dir_all(shared.parent().context("target cache dir")?)?;
            let _ = std::fs::rename(&target, &shared);
        }

        let (pass, failure) = verdict(&check, &transcript, agent.exit, agent.timed_out);

        Ok(json!({
            "task": task.id(),
            "variant": self.variant,
            "model": self.model.name,
            "repeat": self.repeat,
            "pass": pass,
            "failure": failure,
            // What this row was measured under, so rows from different
            // run files compare.
            "config": {
                "engine": engine_of(self.model),
                "window": task.spec.run.max_context_tokens.or(window_of(self.model)),
                "wall_secs": self.wall_secs,
                "trust": trust,
                "jobs": self.jobs,
                "memory": self.memory_label,
                "cluster": task.spec.cluster,
                "grade": task.spec.grade,
            },
            "agent": {
                "exit": agent.exit,
                "timed_out": agent.timed_out,
                "wall_ms": agent.elapsed.as_millis() as u64,
            },
            "behaviour": self.behaviour(&transcript, &agent),
            "summary": transcript.summary.clone().or_else(|| summary_from_db(self.art)),
            "events": transcript.counts,
            "diff": diff,
            "check": check,
        }))
    }
}

impl RunOne<'_> {
    /// The event stream's signals plus koda's log's. A log with nothing to
    /// read (none kept, or written before koda logged these lines) leaves
    /// its fields `null`.
    fn behaviour(&self, transcript: &Transcript, agent: &proc::Finished) -> Value {
        let mut behaviour = transcript.behaviour(agent.timed_out);
        let signals = klog::read(&self.art.join("koda.log")).unwrap_or_default();
        let wall_ms = agent.elapsed.as_millis() as u64;
        if let (Some(row), Value::Object(log)) = (
            behaviour.as_object_mut(),
            signals.to_json(
                self.task
                    .spec
                    .run
                    .max_context_tokens
                    .or(window_of(self.model)),
                wall_ms,
            ),
        ) {
            row.extend(log);
        }
        behaviour
    }
}

/// A graded run's pass and failure reason, from its check and how the agent
/// ended. Shared with `regrade`, so a regraded row reads like a fresh one.
pub(crate) fn verdict(
    check: &check::CheckOutcome,
    transcript: &Transcript,
    exit: Option<i32>,
    timed_out: bool,
) -> (bool, Option<String>) {
    // When koda itself failed (no model call, a provider error, a
    // crash), say so instead of blaming the untouched code.
    let agent_failed = timed_out || exit != Some(0) || transcript.turn_end_kind() == Some("error");
    // A protocol this reader doesn't know leaves the checks unreadable,
    // so a pass from them means nothing.
    let unsupported = transcript.unsupported_protocol();
    let pass = check.passed() && unsupported.is_none();
    let failure = if unsupported.is_some() {
        unsupported
    } else if pass {
        None
    } else if agent_failed && !timed_out {
        Some("agent failed".to_string())
    } else {
        Some(check.failure_kind())
    };
    (pass, failure)
}

/// The engine a model runs on: an explicit `engine`, else the provider
/// with the `base_url` host (`lm-studio@localhost:1234`), else the
/// provider alone (`anthropic`).
fn engine_of(model: &Model) -> String {
    if let Some(engine) = &model.engine {
        return engine.clone();
    }
    match model.base_url.as_deref().and_then(host_of) {
        Some(host) => format!("{}@{host}", model.provider),
        None => model.provider.clone(),
    }
}

/// `host[:port]` of a URL, without a URL parser.
fn host_of(url: &str) -> Option<&str> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    (!host.is_empty()).then_some(host)
}

/// The context window pinned through the model's environment, if any.
/// `null` means koda used the provider's own figure.
fn window_of(model: &Model) -> Option<u64> {
    model
        .env
        .get("KODA_MAX_CONTEXT_TOKENS")
        .and_then(|v| v.trim().parse().ok())
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

/// Copy koda's database, with any WAL files a killed run left, into `art`.
/// It holds the whole conversation, compacted rows included. Best effort,
/// like the log.
fn keep_koda_db(config: &Path, art: &Path) {
    let Ok(entries) = std::fs::read_dir(config.join("koda").join("db")) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("koda.db") {
            let _ = std::fs::copy(entry.path(), art.join(&name));
        }
    }
}

/// A stand-in summary for a run killed before it wrote its own, with the
/// usage koda stored. The run's config dir is fresh, so every row is this run's.
fn summary_from_db(art: &Path) -> Option<Value> {
    let conn = rusqlite::Connection::open_with_flags(
        art.join("koda.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .ok()?;
    conn.query_row(
        "SELECT COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0),
                COALESCE(SUM(cache_read_tokens), 0), COALESCE(SUM(cache_creation_tokens), 0),
                COALESCE(SUM(thinking_tokens), 0), COUNT(*)
         FROM messages
         WHERE prompt_tokens IS NOT NULL OR completion_tokens IS NOT NULL",
        [],
        |row| {
            Ok(json!({
                "usage_source": "database",
                "usage": {
                    "prompt_tokens": row.get::<_, i64>(0)?,
                    "completion_tokens": row.get::<_, i64>(1)?,
                    "cache_read_tokens": row.get::<_, i64>(2)?,
                    "cache_creation_tokens": row.get::<_, i64>(3)?,
                    "thinking_tokens": row.get::<_, i64>(4)?,
                    "api_calls": row.get::<_, i64>(5)?,
                },
            }))
        },
    )
    .ok()
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

    fn model(provider: &str, base_url: Option<&str>, engine: Option<&str>) -> Model {
        Model {
            name: "m".into(),
            provider: provider.into(),
            model: None,
            base_url: base_url.map(str::to_string),
            engine: engine.map(str::to_string),
            env: BTreeMap::new(),
        }
    }

    #[test]
    fn engine_is_the_provider_and_the_host_unless_named() {
        assert_eq!(engine_of(&model("anthropic", None, None)), "anthropic");
        assert_eq!(
            engine_of(&model("lm-studio", Some("http://localhost:1234/v1"), None)),
            "lm-studio@localhost:1234"
        );
        assert_eq!(
            engine_of(&model(
                "openai",
                Some("https://user:pw@api.example.com/v1/"),
                None
            )),
            "openai@api.example.com"
        );
        assert_eq!(
            engine_of(&model(
                "lm-studio",
                Some("http://localhost:1234/v1"),
                Some("mlx")
            )),
            "mlx"
        );
        assert_eq!(host_of("localhost:8080"), Some("localhost:8080"));
        assert_eq!(host_of("http:///v1"), None);
    }

    #[test]
    fn window_comes_from_the_model_env() {
        let mut m = model("lm-studio", None, None);
        assert_eq!(window_of(&m), None);
        m.env
            .insert("KODA_MAX_CONTEXT_TOKENS".into(), "131072".into());
        assert_eq!(window_of(&m), Some(131_072));
        m.env
            .insert("KODA_MAX_CONTEXT_TOKENS".into(), "lots".into());
        assert_eq!(window_of(&m), None);
    }

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
    fn run_file_jobs_is_optional_and_must_be_positive() {
        let parse = |jobs: &str| -> RunFile {
            toml::from_str(&format!(
                r#"
name = "x"
{jobs}
[[variants]]
name = "main"
[[models]]
name = "m"
provider = "anthropic"
"#
            ))
            .unwrap()
        };
        assert_eq!(parse("").jobs, None);
        assert_eq!(parse("jobs = 3").jobs, Some(3));
        validate(&parse("jobs = 3")).unwrap();
        assert!(validate(&parse("jobs = 0")).is_err());
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
    fn run_file_trust_overrides_and_is_checked() {
        let parse = |extra: &str| -> RunFile {
            toml::from_str(&format!(
                "name = \"x\"\n{extra}\n[[variants]]\nname = \"main\"\n[[models]]\nname = \"m\"\nprovider = \"anthropic\"\n"
            ))
            .unwrap()
        };
        assert_eq!(parse("").trust, None);
        let run = parse("trust = \"plan\"");
        assert_eq!(run.trust.as_deref(), Some("plan"));
        validate(&run).unwrap();
        assert!(validate(&parse("trust = \"yolo\"")).is_err());
    }

    #[test]
    fn memory_is_the_checkout_unless_the_run_file_says_otherwise() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("memory")).unwrap();
        std::fs::write(dir.path().join("memory/short.md"), "Build with cargo.\n").unwrap();
        std::fs::write(dir.path().join("memory/blank.md"), "\n").unwrap();

        let resolve = |v| resolve_memory(v, dir.path());
        assert_eq!(
            resolve(None).unwrap(),
            (Memory::Checkout, "checkout".into())
        );
        assert_eq!(
            resolve(Some("checkout")).unwrap(),
            (Memory::Checkout, "checkout".into())
        );
        assert_eq!(
            resolve(Some("none")).unwrap(),
            (Memory::Without, "none".into())
        );
        assert_eq!(
            resolve(Some("memory/short.md")).unwrap(),
            (
                Memory::Pinned("Build with cargo.\n".into()),
                "memory/short.md".into()
            )
        );
        assert!(resolve(Some("memory/missing.md")).is_err());
        assert!(resolve(Some("memory/blank.md")).is_err());

        let run: RunFile = toml::from_str(
            "name = \"x\"\nmemory = \"none\"\n[[variants]]\nname = \"v\"\n[[models]]\nname = \"m\"\nprovider = \"mock\"\n",
        )
        .unwrap();
        assert_eq!(run.memory.as_deref(), Some("none"));
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
    fn a_killed_run_gets_its_usage_from_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        let db_dir = config.join("koda").join("db");
        std::fs::create_dir_all(&db_dir).unwrap();
        let conn = rusqlite::Connection::open(db_dir.join("koda.db")).unwrap();
        conn.execute_batch(
            "CREATE TABLE messages (
                 prompt_tokens INTEGER, completion_tokens INTEGER,
                 cache_read_tokens INTEGER, cache_creation_tokens INTEGER,
                 thinking_tokens INTEGER);
             INSERT INTO messages VALUES (100, 10, 5, NULL, 2), (200, 20, NULL, NULL, NULL),
                                         (NULL, NULL, NULL, NULL, NULL);",
        )
        .unwrap();
        drop(conn);
        let art = dir.path().join("art");
        std::fs::create_dir_all(&art).unwrap();

        keep_koda_db(&config, &art);
        let summary = summary_from_db(&art).unwrap();

        assert_eq!(summary["usage_source"], "database");
        assert_eq!(summary["usage"]["prompt_tokens"], 300);
        assert_eq!(summary["usage"]["completion_tokens"], 30);
        assert_eq!(summary["usage"]["cache_read_tokens"], 5);
        assert_eq!(summary["usage"]["api_calls"], 2);
    }

    #[test]
    fn a_run_with_no_database_gets_no_summary() {
        let dir = tempfile::tempdir().unwrap();
        assert!(summary_from_db(dir.path()).is_none());
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
