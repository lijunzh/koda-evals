//! Task definitions: `tasks/<id>/task.toml`.
//!
//! A task is a coding problem with a known answer. It names where the
//! project comes from (an old koda commit, or a fixture directory), the
//! prompt the agent gets, and hidden tests the agent never sees. After the
//! agent finishes, Rust hidden tests are appended to a source file and
//! hidden files (tests or checkers in any language) are copied in, then the
//! check command decides pass or fail.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

pub const CLUSTERS: [&str; 4] = ["investigation", "panic", "judgement", "kata"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskFile {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Suite cluster: `investigation`, `panic`, `judgement` or `kata`.
    #[serde(default)]
    pub cluster: String,
    /// Demand coordinate: how deep the task reaches into the code. Nothing
    /// reads it yet; the report will.
    #[serde(default)]
    #[allow(dead_code)]
    pub depth: String,
    /// Demand coordinate: the judgement the task asks for. Nothing reads it
    /// yet; the report will.
    #[serde(default)]
    #[allow(dead_code)]
    pub judgement: String,
    /// The tier the repeats earned. Empty until they have run.
    #[serde(default)]
    pub grade: String,
    pub repo: RepoSpec,
    pub check: CheckSpec,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub run: RunSettings,
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
    /// Kata: keep the fixture's own `AGENTS.md`/`CLAUDE.md` whatever the run
    /// file's `memory` says, for a task about following them.
    #[serde(default)]
    pub keep_memory: bool,
    /// Kata: directory copied in after the start commit, so the agent finds
    /// it as someone else's uncommitted work.
    pub uncommitted: Option<String>,
    /// Kata: on the solved side, commit the solution's files (and only
    /// those) with this message, for a check that reads `git log`.
    pub solution_commit: Option<String>,
}

/// How a run is graded: hidden tests, transcript predicates, or both.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckSpec {
    #[serde(default)]
    pub hidden: Vec<HiddenTest>,
    /// Whole files copied into the project before the check: tests or
    /// checkers in any language.
    #[serde(default)]
    pub hidden_files: Vec<HiddenFile>,
    /// Runs the hidden tests; required when there are any.
    pub command: Option<String>,
    #[serde(default = "default_check_secs")]
    pub timeout_secs: u64,
    /// Predicates over `transcript.ndjson`, all of which must hold.
    #[serde(default)]
    pub transcript: Vec<Predicate>,
}

/// A test the agent's choices must pass, read from the event stream.
/// Tool predicates look at top-level calls (`is_sub_agent: false`) unless
/// stated; `no_edit` looks at every agent, because a file a child edited
/// is edited all the same.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    /// Calls of `name` (any tool when absent) number at least `min` and
    /// at most `max`.
    ToolCount {
        name: Option<String>,
        min: Option<u64>,
        max: Option<u64>,
    },
    /// Every call of `name` carries `key` (`present`, the default), or
    /// doesn't (`present = false`), or carries it matching a regex. With
    /// `file`, only calls whose `file_path` ends with it count.
    /// `max_whole_file_reads` calls may break the rule.
    ToolArgs {
        name: String,
        key: String,
        present: Option<bool>,
        matches: Option<String>,
        file: Option<String>,
        #[serde(default)]
        max_whole_file_reads: u64,
    },
    /// Some call matching `a` (one matcher, or any of a list) happened
    /// before `b`: the first call matching `b`, or `"final_message"`, the
    /// agent's last reply.
    Before { a: CallMatches, b: Anchor },
    /// No `Edit` or `Write` call, by any agent.
    NoEdit,
    /// Calls of `InvokeAgent` or `SpawnAgent` number within `min..=max`.
    Delegated { min: Option<u64>, max: Option<u64> },
    /// The agent's last reply (`run_summary.response`): matches a regex,
    /// doesn't match one, and/or is a question (its last line ends with
    /// `?`, or the agent called AskUser).
    FinalMessage {
        matches: Option<String>,
        not_matches: Option<String>,
        is_question: Option<bool>,
    },
    /// The turn ended with this `reason.kind`: `complete`, `error` or
    /// `cancelled`.
    TurnEnd { kind: String },
    /// No destructive Bash attempt: no `approval_request` for Bash with
    /// effect `Destructive` (headless koda auto-rejects these, so an
    /// attempt is what the stream shows) and no `action_blocked` for Bash.
    NoDestructiveBash,
    /// `approval_request` events number within `min..=max`. With `matches`,
    /// only requests whose detail (the command, for Bash) matches the regex
    /// count, so a task about one thing isn't failed by an unrelated prompt.
    Approvals {
        min: Option<u64>,
        max: Option<u64>,
        matches: Option<String>,
    },
    /// Compactions that produced a summary number within `min..=max`. A
    /// task about life after a compaction says `min = 1`, so a run that
    /// never compacted doesn't pass for the wrong reason.
    Compactions { min: Option<u64>, max: Option<u64> },
    /// A conditional claim: the final message may match `claim` only if a
    /// call matching `call` came before it. With `exit_ok`, the last such
    /// call's result must show success (no nonzero exit code and no cargo
    /// failure line). A message that doesn't make the claim passes.
    ClaimRequires {
        claim: String,
        call: CallMatches,
        #[serde(default)]
        exit_ok: bool,
    },
}

/// A tool call by name (one, or any of a list), optionally with a regex
/// that some string in its arguments must match.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallMatch {
    pub name: Names,
    pub args_match: Option<String>,
}

/// A tool name, or a list of names any of which counts.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Names {
    One(String),
    Any(Vec<String>),
}

impl Names {
    pub fn contains(&self, name: &str) -> bool {
        match self {
            Names::One(n) => n == name,
            Names::Any(ns) => ns.iter().any(|n| n == name),
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self, Names::Any(ns) if ns.is_empty())
    }
}

impl std::fmt::Display for Names {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Names::One(n) => write!(f, "{n}"),
            Names::Any(ns) => write!(f, "{}", ns.join("|")),
        }
    }
}

impl std::fmt::Display for CallMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(m) = &self.args_match {
            write!(f, "~{m:?}")?;
        }
        Ok(())
    }
}

/// One matcher, or a list of alternatives any of which counts (so Bash
/// `cargo test` and the `Cargo` tool's test subcommand can be one side of
/// a predicate).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CallMatches {
    One(CallMatch),
    Any(Vec<CallMatch>),
}

impl CallMatches {
    pub fn iter(&self) -> impl Iterator<Item = &CallMatch> {
        match self {
            CallMatches::One(m) => std::slice::from_ref(m).iter(),
            CallMatches::Any(ms) => ms.iter(),
        }
    }

    fn validate(&self, what: &str) -> Result<()> {
        if self.iter().next().is_none() {
            bail!("{what} needs at least one matcher");
        }
        for m in self.iter() {
            if m.name.is_empty() {
                bail!("{what}: name needs at least one tool");
            }
            regex_ok(what, m.args_match.as_deref())?;
        }
        Ok(())
    }
}

impl std::fmt::Display for CallMatches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallMatches::One(m) => write!(f, "{m}"),
            CallMatches::Any(ms) => {
                write!(f, "[")?;
                for (i, m) in ms.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{m}")?;
                }
                write!(f, "]")
            }
        }
    }
}

/// The `b` side of `before`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Anchor {
    /// Only `"final_message"` is accepted.
    Named(String),
    Call(CallMatch),
}

fn regex_ok(what: &str, pattern: Option<&str>) -> Result<()> {
    if let Some(p) = pattern {
        regex::Regex::new(p).with_context(|| format!("{what}: bad regex {p:?}"))?;
    }
    Ok(())
}

fn range_ok(what: &str, min: Option<u64>, max: Option<u64>) -> Result<()> {
    match (min, max) {
        (None, None) => bail!("{what} needs min or max"),
        (Some(lo), Some(hi)) if lo > hi => bail!("{what}: min {lo} is above max {hi}"),
        _ => Ok(()),
    }
}

impl Predicate {
    /// Reject what can never be evaluated: bad regexes, empty ranges.
    pub fn validate(&self) -> Result<()> {
        match self {
            Predicate::ToolCount { min, max, .. } => range_ok("tool_count", *min, *max),
            Predicate::Delegated { min, max } => range_ok("delegated", *min, *max),
            Predicate::Approvals { min, max, matches } => {
                regex_ok("approvals", matches.as_deref())?;
                range_ok("approvals", *min, *max)
            }
            Predicate::Compactions { min, max } => range_ok("compactions", *min, *max),
            Predicate::ToolArgs {
                present, matches, ..
            } => {
                if *present == Some(false) && matches.is_some() {
                    bail!("tool_args: present = false can't go with matches");
                }
                regex_ok("tool_args", matches.as_deref())
            }
            Predicate::Before { a, b } => {
                a.validate("before.a")?;
                match b {
                    Anchor::Named(n) if n != "final_message" => {
                        bail!("before: b must be \"final_message\" or a tool call, not {n:?}")
                    }
                    Anchor::Named(_) => Ok(()),
                    Anchor::Call(c) => regex_ok("before.b", c.args_match.as_deref()),
                }
            }
            Predicate::FinalMessage {
                matches,
                not_matches,
                is_question,
            } => {
                if matches.is_none() && not_matches.is_none() && is_question.is_none() {
                    bail!("final_message needs matches, not_matches or is_question");
                }
                regex_ok("final_message", matches.as_deref())?;
                regex_ok("final_message", not_matches.as_deref())
            }
            Predicate::TurnEnd { kind } => {
                if !["complete", "error", "cancelled"].contains(&kind.as_str()) {
                    bail!("turn_end: kind {kind:?} must be complete, error or cancelled");
                }
                Ok(())
            }
            Predicate::ClaimRequires { claim, call, .. } => {
                regex_ok("claim_requires", Some(claim))?;
                call.validate("claim_requires.call")
            }
            Predicate::NoEdit | Predicate::NoDestructiveBash => Ok(()),
        }
    }
}

impl std::fmt::Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn range(
            f: &mut std::fmt::Formatter<'_>,
            min: Option<u64>,
            max: Option<u64>,
        ) -> std::fmt::Result {
            if let Some(m) = min {
                write!(f, " min={m}")?;
            }
            if let Some(m) = max {
                write!(f, " max={m}")?;
            }
            Ok(())
        }
        match self {
            Predicate::ToolCount { name, min, max } => {
                write!(f, "tool_count {}", name.as_deref().unwrap_or("*"))?;
                range(f, *min, *max)
            }
            Predicate::ToolArgs {
                name,
                key,
                present,
                matches,
                file,
                max_whole_file_reads,
            } => {
                write!(f, "tool_args {name}.{key}")?;
                if let Some(file) = file {
                    write!(f, " file={file}")?;
                }
                match (present, matches) {
                    (_, Some(m)) => write!(f, " matches={m:?}")?,
                    (Some(false), None) => write!(f, " absent")?,
                    _ => write!(f, " present")?,
                }
                if *max_whole_file_reads > 0 {
                    write!(f, " allow={max_whole_file_reads}")?;
                }
                Ok(())
            }
            Predicate::Before { a, b } => {
                write!(f, "before {a}")?;
                match b {
                    Anchor::Named(n) => write!(f, " < {n}"),
                    Anchor::Call(c) => write!(f, " < {c}"),
                }
            }
            Predicate::NoEdit => write!(f, "no_edit"),
            Predicate::Delegated { min, max } => {
                write!(f, "delegated")?;
                range(f, *min, *max)
            }
            Predicate::FinalMessage {
                matches,
                not_matches,
                is_question,
            } => {
                write!(f, "final_message")?;
                if let Some(m) = matches {
                    write!(f, " matches={m:?}")?;
                }
                if let Some(m) = not_matches {
                    write!(f, " not_matches={m:?}")?;
                }
                if let Some(q) = is_question {
                    write!(f, " is_question={q}")?;
                }
                Ok(())
            }
            Predicate::TurnEnd { kind } => write!(f, "turn_end {kind}"),
            Predicate::NoDestructiveBash => write!(f, "no_destructive_bash"),
            Predicate::Approvals { min, max, matches } => {
                write!(f, "approvals")?;
                if let Some(m) = matches {
                    write!(f, " matches={m:?}")?;
                }
                range(f, *min, *max)
            }
            Predicate::Compactions { min, max } => {
                write!(f, "compactions")?;
                range(f, *min, *max)
            }
            Predicate::ClaimRequires {
                claim,
                call,
                exit_ok,
            } => {
                write!(f, "claim_requires {claim:?} needs {call}")?;
                if *exit_ok {
                    write!(f, " exit_ok")?;
                }
                Ok(())
            }
        }
    }
}

/// Per-task run settings, `[run]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSettings {
    /// Koda trust mode: `auto` (default) or `safe`.
    #[serde(default = "default_trust")]
    pub trust: String,
    /// Directory of agent JSON configs, relative to the task, copied to
    /// `<project>/agents/` in the start commit (koda reads project agents
    /// from there; its user agents live under `$HOME`, which the run
    /// doesn't own).
    pub agents_dir: Option<String>,
    /// A context window koda must run this task at, in tokens, whatever
    /// the run file pins for the model: for tasks about what happens when
    /// a session compacts, which at a 1M window never happens. Passed as
    /// `KODA_MAX_CONTEXT_TOKENS` and recorded as the row's window. Claude
    /// Code and Codex variants can't take it.
    pub max_context_tokens: Option<u64>,
}

impl Default for RunSettings {
    fn default() -> Self {
        Self {
            trust: default_trust(),
            agents_dir: None,
            max_context_tokens: None,
        }
    }
}

fn default_trust() -> String {
    "auto".into()
}

const TRUST_MODES: [&str; 2] = ["auto", "safe"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HiddenTest {
    /// Source file, relative to the project root, the tests are appended to.
    pub file: String,
    /// File holding the test functions, relative to the task directory.
    pub test: String,
}

/// A file copied into the project after the agent finishes, replacing any
/// file the agent left at `to`. For tests outside Rust (a Python
/// `unittest` module) or a checker script; the check counts it only if its
/// test ids contain `eval_hidden` (see `check::outcome`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HiddenFile {
    /// Relative to the task directory.
    pub from: String,
    /// Relative to the project root, inside it.
    pub to: String,
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

        if !spec.cluster.is_empty() && !CLUSTERS.contains(&spec.cluster.as_str()) {
            bail!(
                "{}: cluster {:?} must be one of {}",
                path.display(),
                spec.cluster,
                CLUSTERS.join(", ")
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
                let uncommitted = spec.repo.uncommitted.as_ref().map(|u| dir.join(u));
                for d in [&fixture, &solution].into_iter().chain(&uncommitted) {
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

        let has_hidden = !spec.check.hidden.is_empty() || !spec.check.hidden_files.is_empty();
        if !has_hidden && spec.check.transcript.is_empty() {
            bail!(
                "{}: [check] needs hidden tests, transcript predicates, or both",
                path.display()
            );
        }
        if has_hidden && spec.check.command.is_none() {
            bail!("{}: [check] hidden tests need a command", path.display());
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
        let mut targets = std::collections::BTreeSet::new();
        for file in &spec.check.hidden_files {
            let from = dir.join(&file.from);
            if !from.is_file() {
                bail!(
                    "{}: hidden file {} not found",
                    path.display(),
                    from.display()
                );
            }
            let to = Path::new(&file.to);
            let inside = !file.to.is_empty()
                && to
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)));
            if !inside {
                bail!(
                    "{}: hidden file target {:?} must be a relative path inside the project",
                    path.display(),
                    file.to
                );
            }
            if !targets.insert(to) {
                bail!("{}: two hidden files go to {:?}", path.display(), file.to);
            }
        }
        for (i, predicate) in spec.check.transcript.iter().enumerate() {
            predicate
                .validate()
                .with_context(|| format!("{}: check.transcript[{i}]", path.display()))?;
        }
        if !spec.check.transcript.is_empty() {
            // `verify` can't run a model: the predicates are checked
            // against two canned transcripts instead.
            for canned in [CANNED_PASS, CANNED_FAIL] {
                if !dir.join(canned).is_file() {
                    bail!(
                        "{}: transcript predicates need {canned} for verify",
                        path.display()
                    );
                }
            }
        }

        if !TRUST_MODES.contains(&spec.run.trust.as_str()) {
            bail!(
                "{}: run.trust {:?} must be one of {}",
                path.display(),
                spec.run.trust,
                TRUST_MODES.join(", ")
            );
        }
        if let Some(agents) = &spec.run.agents_dir {
            let agents = dir.join(agents);
            let has_json = std::fs::read_dir(&agents)
                .map(|d| {
                    d.flatten()
                        .any(|e| e.path().extension().is_some_and(|x| x == "json"))
                })
                .unwrap_or(false);
            if !has_json {
                bail!(
                    "{}: run.agents_dir {} holds no agent .json",
                    path.display(),
                    agents.display()
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

    pub fn has_hidden(&self) -> bool {
        !self.spec.check.hidden.is_empty() || !self.spec.check.hidden_files.is_empty()
    }

    pub fn has_predicates(&self) -> bool {
        !self.spec.check.transcript.is_empty()
    }

    /// The canned transcript the predicates must pass.
    pub fn canned_pass(&self) -> PathBuf {
        self.dir.join(CANNED_PASS)
    }

    /// The canned transcript the predicates must fail.
    pub fn canned_fail(&self) -> PathBuf {
        self.dir.join(CANNED_FAIL)
    }

    /// The directory copied in after the start commit, if any.
    pub fn uncommitted_dir(&self) -> Option<PathBuf> {
        self.spec
            .repo
            .uncommitted
            .as_ref()
            .map(|d| self.dir.join(d))
    }

    /// The agents directory to copy into the project, if any.
    pub fn agents_dir(&self) -> Option<PathBuf> {
        self.spec.run.agents_dir.as_ref().map(|d| self.dir.join(d))
    }
}

/// Canned transcripts, relative to the task directory.
pub const CANNED_PASS: &str = "hidden/should-pass.ndjson";
pub const CANNED_FAIL: &str = "hidden/should-fail.ndjson";

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
        assert_eq!(task.spec.cluster, "");
        assert_eq!(task.spec.grade, "");
    }

    fn with_cluster(root: &Path, cluster: &str) -> PathBuf {
        let dir = kata(root, "k1", "");
        let path = dir.join("task.toml");
        let text = std::fs::read_to_string(&path).unwrap();
        let line = format!("cluster = \"{cluster}\"\n[repo]\n");
        std::fs::write(&path, text.replace("[repo]\n", &line)).unwrap();
        dir
    }

    #[test]
    fn loads_a_known_cluster() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = with_cluster(tmp.path(), "panic");
        assert_eq!(Task::load(&dir).unwrap().spec.cluster, "panic");
    }

    #[test]
    fn rejects_an_unknown_cluster() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = with_cluster(tmp.path(), "panics");
        let err = Task::load(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("must be one of"), "{err:#}");
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

    fn with_check(root: &Path, check: &str, run: &str) -> PathBuf {
        let dir = kata(root, "k1", "");
        let path = dir.join("task.toml");
        let text = std::fs::read_to_string(&path).unwrap();
        let head = text.split("[check]").next().unwrap();
        std::fs::write(&path, format!("{head}[check]\n{check}\n{run}")).unwrap();
        dir
    }

    #[test]
    fn a_task_needs_hidden_tests_or_predicates() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = with_check(tmp.path(), "command = \"true\"", "");
        let err = Task::load(&dir).unwrap_err();
        assert!(
            format!("{err:#}").contains("hidden tests, transcript predicates"),
            "{err:#}"
        );
    }

    #[test]
    fn hidden_tests_need_a_command() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = with_check(
            tmp.path(),
            "hidden = [{ file = \"src/lib.rs\", test = \"hidden/t.rs\" }]",
            "",
        );
        let err = Task::load(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("need a command"), "{err:#}");
    }

    #[test]
    fn hidden_files_alone_grade_a_task_and_need_a_command() {
        let tmp = tempfile::tempdir().unwrap();
        let files = "hidden_files = [{ from = \"hidden/test_eval_hidden.py\", to = \"tests/test_eval_hidden.py\" }]";
        let dir = with_check(tmp.path(), files, "");
        write(&dir, "hidden/test_eval_hidden.py", "");
        let err = Task::load(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("need a command"), "{err:#}");

        let dir = with_check(
            &tmp.path().join("b"),
            &format!("{files}\ncommand = \"python3 -m unittest -v\""),
            "",
        );
        write(&dir, "hidden/test_eval_hidden.py", "");
        let task = Task::load(&dir).unwrap();
        assert!(task.has_hidden());
        assert_eq!(
            task.spec.check.hidden_files[0].to,
            "tests/test_eval_hidden.py"
        );
    }

    #[test]
    fn hidden_files_must_exist_and_land_inside_the_project() {
        let tmp = tempfile::tempdir().unwrap();
        let load = |n: &str, from: &str, to: &str| {
            let dir = with_check(
                &tmp.path().join(n),
                &format!(
                    "hidden_files = [{{ from = \"{from}\", to = \"{to}\" }}]\ncommand = \"true\""
                ),
                "",
            );
            write(&dir, "hidden/check.py", "");
            format!("{:#}", Task::load(&dir).unwrap_err())
        };
        assert!(load("a", "hidden/nope.py", "check.py").contains("not found"));
        for to in [
            "",
            "/tmp/check.py",
            "../check.py",
            "a/../../check.py",
            "./check.py",
        ] {
            let err = load("b", "hidden/check.py", to);
            assert!(err.contains("inside the project"), "{to:?}: {err}");
        }

        let dir = with_check(
            &tmp.path().join("c"),
            "hidden_files = [{ from = \"hidden/check.py\", to = \"c.py\" }, \
             { from = \"hidden/check.py\", to = \"c.py\" }]\ncommand = \"true\"",
            "",
        );
        write(&dir, "hidden/check.py", "");
        let err = format!("{:#}", Task::load(&dir).unwrap_err());
        assert!(err.contains("two hidden files"), "{err}");
    }

    #[test]
    fn predicates_parse_and_need_canned_transcripts() {
        let tmp = tempfile::tempdir().unwrap();
        let check = r#"
[[check.transcript]]
type = "tool_count"
name = "Edit"
min = 1

[[check.transcript]]
type = "tool_args"
name = "Read"
key = "start_line"
file = "src/big.rs"
max_whole_file_reads = 1

[[check.transcript]]
type = "before"
a = { name = "Bash", args_match = "cargo test" }
b = "final_message"

[[check.transcript]]
type = "before"
a = { name = "Read" }
b = { name = "Edit" }

[[check.transcript]]
type = "final_message"
not_matches = "(?i)tests pass"
is_question = false

[[check.transcript]]
type = "turn_end"
kind = "complete"

[[check.transcript]]
type = "no_edit"

[[check.transcript]]
type = "no_destructive_bash"

[[check.transcript]]
type = "delegated"
max = 0

[[check.transcript]]
type = "approvals"
max = 0

[[check.transcript]]
type = "claim_requires"
claim = "(?i)tests pass"
call = [{ name = "Bash", args_match = "cargo test" }, { name = "Cargo", args_match = "^test$" }]
exit_ok = true

[[check.transcript]]
type = "before"
a = { name = ["Bash", "Cargo"] }
b = { name = "Edit" }
"#;
        let dir = with_check(tmp.path(), check, "");
        let err = Task::load(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("should-pass.ndjson"), "{err:#}");
        write(&dir, CANNED_PASS, "");
        write(&dir, CANNED_FAIL, "");
        let task = Task::load(&dir).unwrap();
        assert!(!task.has_hidden());
        assert!(task.has_predicates());
        let shown: Vec<String> = task
            .spec
            .check
            .transcript
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            shown,
            [
                "tool_count Edit min=1",
                "tool_args Read.start_line file=src/big.rs present allow=1",
                "before Bash~\"cargo test\" < final_message",
                "before Read < Edit",
                "final_message not_matches=\"(?i)tests pass\" is_question=false",
                "turn_end complete",
                "no_edit",
                "no_destructive_bash",
                "delegated max=0",
                "approvals max=0",
                "claim_requires \"(?i)tests pass\" needs [Bash~\"cargo test\" | Cargo~\"^test$\"] exit_ok",
                "before Bash|Cargo < Edit",
            ]
        );
        assert_eq!(task.spec.run.trust, "auto");
        assert_eq!(task.agents_dir(), None);
    }

    #[test]
    fn predicate_validation_rejects_bad_regex_and_empty_ranges() {
        let tmp = tempfile::tempdir().unwrap();
        for (check, msg) in [
            (
                r#"transcript = [{ type = "tool_count", name = "Edit" }]"#,
                "needs min or max",
            ),
            (
                r#"transcript = [{ type = "delegated", min = 2, max = 1 }]"#,
                "above max",
            ),
            (
                r#"transcript = [{ type = "final_message", matches = "(" }]"#,
                "bad regex",
            ),
            (
                r#"transcript = [{ type = "before", a = { name = "Bash" }, b = "last" }]"#,
                "final_message",
            ),
            (
                r#"transcript = [{ type = "tool_args", name = "Read", key = "x", present = false, matches = "y" }]"#,
                "can't go with",
            ),
            (
                r#"transcript = [{ type = "final_message" }]"#,
                "needs matches",
            ),
            (
                r#"transcript = [{ type = "claim_requires", claim = "(", call = { name = "Bash" } }]"#,
                "bad regex",
            ),
            (
                r#"transcript = [{ type = "claim_requires", claim = "x", call = [] }]"#,
                "at least one matcher",
            ),
            (
                r#"transcript = [{ type = "claim_requires", claim = "x", call = { name = [] } }]"#,
                "at least one tool",
            ),
            (
                r#"transcript = [{ type = "before", a = [{ name = "Bash", args_match = "(" }], b = "final_message" }]"#,
                "bad regex",
            ),
        ] {
            let dir = with_check(tmp.path(), check, "");
            write(&dir, CANNED_PASS, "");
            write(&dir, CANNED_FAIL, "");
            let err = Task::load(&dir).unwrap_err();
            assert!(format!("{err:#}").contains(msg), "{check}: {err:#}");
            std::fs::remove_dir_all(&dir).unwrap();
        }
    }

    #[test]
    fn run_settings_parse_and_validate() {
        let tmp = tempfile::tempdir().unwrap();
        let hidden =
            "hidden = [{ file = \"src/lib.rs\", test = \"hidden/t.rs\" }]\ncommand = \"true\"";
        let dir = with_check(
            tmp.path(),
            hidden,
            "[run]\ntrust = \"safe\"\nagents_dir = \"agents\"",
        );
        let err = Task::load(&dir).unwrap_err();
        assert!(
            format!("{err:#}").contains("holds no agent .json"),
            "{err:#}"
        );
        write(&dir, "agents/worker.json", "{}");
        let task = Task::load(&dir).unwrap();
        assert_eq!(task.spec.run.trust, "safe");
        assert_eq!(task.agents_dir(), Some(dir.join("agents")));

        let dir = with_check(tmp.path(), hidden, "[run]\ntrust = \"plan\"");
        let err = Task::load(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("run.trust"), "{err:#}");
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

    #[test]
    fn the_suite_leaves_out_measurement_tasks() {
        let all = load_all(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tasks")).unwrap();
        let suite = select(&all, &[], &["suite".into()]).unwrap();
        assert_eq!(suite.len(), 31);
        for task in suite {
            assert!(
                !task.spec.tags.iter().any(|t| t == "measurement"),
                "{}",
                task.id()
            );
        }
    }
}
