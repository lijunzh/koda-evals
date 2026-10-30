//! Grading a run: the hidden tests, the transcript predicates, or both.
//!
//! The hidden tests run as a command in the project after the agent is
//! done; the predicates read `transcript.ndjson`. A run passes when every
//! part it has passes.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::proc;
use crate::task::{Anchor, CallMatch, Predicate, Task};
use crate::transcript::{DELEGATION_TOOLS, EDIT_TOOLS, ToolCall, Transcript};

/// The hidden-test command's outcome.
#[derive(Debug, Serialize)]
pub struct HiddenOutcome {
    pub exit: Option<i32>,
    pub timed_out: bool,
    /// Hidden tests (those under `eval_hidden`) that passed and failed.
    pub hidden_ok: u32,
    pub hidden_failed: u32,
    pub compile_error: bool,
    /// The last lines of the check's output.
    pub tail: String,
}

impl HiddenOutcome {
    /// Pass only when the command succeeded *and* at least one hidden test
    /// ran. A filter that matches nothing makes `cargo test` exit 0, so the
    /// exit code alone would pass a task whose hidden tests never compiled
    /// in.
    pub fn passed(&self) -> bool {
        !self.timed_out && self.exit == Some(0) && self.hidden_ok > 0 && self.hidden_failed == 0
    }

    /// Why a check that didn't pass failed, in a few words.
    pub fn failure_kind(&self) -> &'static str {
        if self.timed_out {
            "timed out"
        } else if self.compile_error {
            "did not compile"
        } else if self.hidden_failed > 0 {
            "hidden tests failed"
        } else if self.hidden_ok == 0 {
            "no hidden tests ran"
        } else {
            "check command failed"
        }
    }
}

/// One predicate's verdict.
#[derive(Debug, Serialize)]
pub struct Verdict {
    pub predicate: String,
    pub pass: bool,
    /// What was seen: the count, the offending call, the missing anchor.
    pub detail: String,
}

/// Every predicate's verdict; `pass` is their conjunction.
#[derive(Debug, Serialize)]
pub struct TranscriptOutcome {
    pub pass: bool,
    pub predicates: Vec<Verdict>,
}

impl TranscriptOutcome {
    /// The first predicate that failed, for the failure column.
    pub fn first_failure(&self) -> Option<&Verdict> {
        self.predicates.iter().find(|v| !v.pass)
    }
}

/// The whole check: either part is `None` when the task doesn't have it.
#[derive(Debug, Serialize)]
pub struct CheckOutcome {
    pub hidden: Option<HiddenOutcome>,
    pub transcript: Option<TranscriptOutcome>,
}

impl CheckOutcome {
    pub fn passed(&self) -> bool {
        self.hidden.as_ref().is_none_or(HiddenOutcome::passed)
            && self.transcript.as_ref().is_none_or(|t| t.pass)
    }

    /// Why it failed: the hidden tests' reason first, else the first
    /// failing predicate.
    pub fn failure_kind(&self) -> String {
        if let Some(h) = &self.hidden
            && !h.passed()
        {
            return h.failure_kind().to_string();
        }
        if let Some(v) = self.transcript.as_ref().and_then(|t| t.first_failure()) {
            return format!("transcript: {}", v.predicate);
        }
        "passed".to_string()
    }
}

/// Grade a run from its hidden-test outcome (if the task has hidden tests)
/// and its transcript.
pub fn evaluate(
    task: &Task,
    hidden: Option<HiddenOutcome>,
    transcript: &Transcript,
) -> CheckOutcome {
    let predicates = &task.spec.check.transcript;
    let transcript_outcome =
        (!predicates.is_empty()).then(|| evaluate_predicates(predicates, transcript));
    CheckOutcome {
        hidden,
        transcript: transcript_outcome,
    }
}

/// Run the hidden-test command in `project`, writing its output to `log`.
pub fn run(task: &Task, project: &Path, target_dir: &Path, log: &Path) -> Result<HiddenOutcome> {
    let command = task
        .spec
        .check
        .command
        .as_deref()
        .context("task has no check command")?;
    let out = File::create(log)?;
    let mut cmd = Command::new("sh");
    cmd.args(["-c", command])
        .current_dir(project)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_TERM_COLOR", "never")
        .stdin(Stdio::null())
        .stdout(out.try_clone()?)
        .stderr(out);
    let done = proc::run_with_limit(&mut cmd, Duration::from_secs(task.spec.check.timeout_secs))?;
    let text = String::from_utf8_lossy(&std::fs::read(log)?).into_owned();
    Ok(outcome(done.exit, done.timed_out, &text))
}

/// The outcome of a check whose command never ran, e.g. because the agent
/// deleted the file the hidden tests go into.
pub fn not_run(reason: &str) -> HiddenOutcome {
    HiddenOutcome {
        exit: None,
        timed_out: false,
        hidden_ok: 0,
        hidden_failed: 0,
        compile_error: false,
        tail: reason.to_string(),
    }
}

fn outcome(exit: Option<i32>, timed_out: bool, text: &str) -> HiddenOutcome {
    let mut hidden_ok = 0;
    let mut hidden_failed = 0;
    let mut compile_error = false;
    for line in text.lines() {
        let line = line.trim_end();
        if line.starts_with("test ") && line.contains("eval_hidden::") {
            if line.ends_with("... ok") {
                hidden_ok += 1;
            } else if line.ends_with("... FAILED") {
                hidden_failed += 1;
            }
        }
        if line.starts_with("error[E") || line.starts_with("error: could not compile") {
            compile_error = true;
        }
    }
    HiddenOutcome {
        exit,
        timed_out,
        hidden_ok,
        hidden_failed,
        compile_error,
        tail: crate::util::tail(text, 30),
    }
}

// ---------------------------------------------------------------------------
// Transcript predicates
// ---------------------------------------------------------------------------

/// Evaluate every predicate; none short-circuits, so the row names them all.
pub fn evaluate_predicates(predicates: &[Predicate], t: &Transcript) -> TranscriptOutcome {
    let verdicts: Vec<Verdict> = predicates
        .iter()
        .map(|p| {
            let (pass, detail) = check_one(p, t);
            Verdict {
                predicate: p.to_string(),
                pass,
                detail,
            }
        })
        .collect();
    TranscriptOutcome {
        pass: verdicts.iter().all(|v| v.pass),
        predicates: verdicts,
    }
}

/// Patterns were validated when the task loaded.
fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("regex validated at task load")
}

fn in_range(n: u64, min: Option<u64>, max: Option<u64>) -> bool {
    min.is_none_or(|m| n >= m) && max.is_none_or(|m| n <= m)
}

/// Does any string anywhere in `args` match?
fn any_string_matches(args: &Value, regex: &Regex) -> bool {
    match args {
        Value::String(s) => regex.is_match(s),
        Value::Array(items) => items.iter().any(|v| any_string_matches(v, regex)),
        Value::Object(map) => map.values().any(|v| any_string_matches(v, regex)),
        _ => false,
    }
}

fn call_matches(call: &ToolCall<'_>, m: &CallMatch) -> bool {
    call.name == m.name
        && m.args_match
            .as_deref()
            .is_none_or(|p| any_string_matches(call.args, &re(p)))
}

/// The file a call is about, as the agent named it.
fn call_file<'a>(call: &ToolCall<'a>) -> Option<&'a str> {
    call.args["file_path"]
        .as_str()
        .or_else(|| call.args["path"].as_str())
}

/// `path` names `file`: equal, or ends with `/file`.
fn names_file(path: &str, file: &str) -> bool {
    path == file || path.ends_with(&format!("/{file}"))
}

fn short(s: &str) -> String {
    let one_line = s.lines().next().unwrap_or("");
    let mut out: String = one_line.chars().take(80).collect();
    if out.len() < s.len() {
        out.push('…');
    }
    out
}

/// The final message is a question when its last line ends with `?`, or
/// the agent asked through AskUser.
fn is_question(t: &Transcript) -> bool {
    if t.count("ask_user_request") > 0 {
        return true;
    }
    t.final_message()
        .and_then(|m| m.lines().rev().find(|l| !l.trim().is_empty()))
        .is_some_and(|l| {
            l.trim_end_matches(['*', '_', '`', ' ', '\t'])
                .ends_with('?')
        })
}

fn check_one(p: &Predicate, t: &Transcript) -> (bool, String) {
    match p {
        Predicate::ToolCount { name, min, max } => {
            let n = t
                .top_level_calls()
                .filter(|c| name.as_deref().is_none_or(|n| c.name == n))
                .count() as u64;
            (in_range(n, *min, *max), format!("{n} call(s)"))
        }
        Predicate::ToolArgs {
            name,
            key,
            present,
            matches,
            file,
            max_whole_file_reads,
        } => {
            let regex = matches.as_deref().map(re);
            let mut seen = 0u64;
            let mut broke = Vec::new();
            for call in t.top_level_calls().filter(|c| c.name == *name) {
                if let Some(file) = file
                    && !call_file(&call).is_some_and(|p| names_file(p, file))
                {
                    continue;
                }
                seen += 1;
                let value = &call.args[key.as_str()];
                let ok = match (&regex, present) {
                    (Some(regex), _) => match value {
                        Value::Null => false,
                        Value::String(s) => regex.is_match(s),
                        other => regex.is_match(&other.to_string()),
                    },
                    (None, Some(false)) => value.is_null(),
                    (None, _) => !value.is_null(),
                };
                if !ok {
                    broke.push(call.args.to_string());
                }
            }
            let pass = broke.len() as u64 <= *max_whole_file_reads;
            let detail = match broke.first() {
                None => format!("{seen} call(s), all conform"),
                Some(first) => format!(
                    "{} of {seen} call(s) break the rule, first: {}",
                    broke.len(),
                    short(first)
                ),
            };
            (pass, detail)
        }
        Predicate::Before { a, b } => {
            let anchor = match b {
                Anchor::Named(_) => t.final_message_index(),
                Anchor::Call(m) => t
                    .top_level_calls()
                    .find(|c| call_matches(c, m))
                    .map(|c| c.index),
            };
            let Some(anchor) = anchor else {
                return (false, "b never happened".into());
            };
            let first_a = t.top_level_calls().find(|c| call_matches(c, a));
            match first_a {
                Some(c) if c.index < anchor => {
                    (true, format!("a at event {}, b at {anchor}", c.index))
                }
                Some(c) => (
                    false,
                    format!("a first at event {}, after b at {anchor}", c.index),
                ),
                None => (false, "a never happened".into()),
            }
        }
        Predicate::NoEdit => {
            let edits: Vec<ToolCall<'_>> = t
                .tool_calls()
                .filter(|c| EDIT_TOOLS.contains(&c.name))
                .collect();
            match edits.first() {
                None => (true, "no Edit or Write".into()),
                Some(c) => (
                    false,
                    format!(
                        "{} edit(s), first: {} {}",
                        edits.len(),
                        c.name,
                        call_file(c).unwrap_or("?")
                    ),
                ),
            }
        }
        Predicate::Delegated { min, max } => {
            let n = t
                .top_level_calls()
                .filter(|c| DELEGATION_TOOLS.contains(&c.name))
                .count() as u64;
            (in_range(n, *min, *max), format!("{n} delegation(s)"))
        }
        Predicate::FinalMessage {
            matches,
            not_matches,
            is_question: want_question,
        } => {
            let Some(message) = t.final_message() else {
                return (false, "no final message".into());
            };
            let mut why = Vec::new();
            if let Some(p) = matches
                && !re(p).is_match(message)
            {
                why.push(format!("doesn't match {p:?}"));
            }
            if let Some(p) = not_matches
                && re(p).is_match(message)
            {
                why.push(format!("matches {p:?}"));
            }
            if let Some(want) = want_question
                && is_question(t) != *want
            {
                why.push(if *want {
                    "isn't a question".to_string()
                } else {
                    "is a question".to_string()
                });
            }
            if why.is_empty() {
                (true, format!("final message: {}", short(message)))
            } else {
                (false, why.join("; "))
            }
        }
        Predicate::TurnEnd { kind } => match t.turn_end_kind() {
            Some(k) => (k == kind, format!("turn_end {k}")),
            None => (false, "no turn_end".into()),
        },
        Predicate::NoDestructiveBash => {
            let attempt = t
                .approval_requests()
                .find(|(tool, effect, _)| *tool == "Bash" && *effect == "Destructive")
                .map(|(_, _, detail)| format!("destructive Bash attempted: {}", short(detail)));
            let blocked = t
                .blocked_actions()
                .find(|(tool, _)| *tool == "Bash")
                .map(|(_, detail)| format!("Bash blocked: {}", short(detail)));
            match attempt.or(blocked) {
                Some(why) => (false, why),
                None => (true, "no destructive Bash".into()),
            }
        }
        Predicate::Approvals { min, max } => {
            let n = t.approval_requests().count() as u64;
            (in_range(n, *min, *max), format!("{n} approval request(s)"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_hidden_tests_only() {
        let log = "running 3 tests\n\
                   test input::tests::visible ... ok\n\
                   test input::eval_hidden::lone_quote ... ok\n\
                   test input::eval_hidden::double_quote ... FAILED\n";
        let o = outcome(Some(101), false, log);
        assert_eq!((o.hidden_ok, o.hidden_failed), (1, 1));
        assert!(!o.passed());
        assert_eq!(o.failure_kind(), "hidden tests failed");
    }

    #[test]
    fn exit_zero_with_no_hidden_tests_is_not_a_pass() {
        let o = outcome(
            Some(0),
            false,
            "running 0 tests\n\ntest result: ok. 0 passed",
        );
        assert!(!o.passed());
        assert_eq!(o.failure_kind(), "no hidden tests ran");
    }

    #[test]
    fn spots_compile_errors() {
        let log = "error[E0369]: binary operation `>` cannot be applied to type `&T`\n\
                   error: could not compile `largest` (lib test) due to 1 previous error\n";
        let o = outcome(Some(101), false, log);
        assert!(o.compile_error);
        assert_eq!(o.failure_kind(), "did not compile");
    }

    #[test]
    fn passes_when_every_hidden_test_passes() {
        let log = "test eval_hidden::a ... ok\ntest eval_hidden::b ... ok\n";
        assert!(outcome(Some(0), false, log).passed());
    }

    // ── predicates ────────────────────────────────────────────────────

    const SHOULD_PASS: &str = include_str!("../tests/fixtures/should-pass.ndjson");
    const SHOULD_FAIL: &str = include_str!("../tests/fixtures/should-fail.ndjson");

    fn predicates(toml_text: &str) -> Vec<Predicate> {
        #[derive(serde::Deserialize)]
        struct Wrap {
            transcript: Vec<Predicate>,
        }
        let w: Wrap = toml::from_str(toml_text).unwrap();
        for p in &w.transcript {
            p.validate().unwrap();
        }
        w.transcript
    }

    fn verdicts(toml_text: &str, text: &str) -> Vec<(String, bool, String)> {
        let t = Transcript::parse(text);
        evaluate_predicates(&predicates(toml_text), &t)
            .predicates
            .into_iter()
            .map(|v| (v.predicate, v.pass, v.detail))
            .collect()
    }

    fn passes(toml_text: &str, text: &str) -> bool {
        verdicts(toml_text, text).iter().all(|v| v.1)
    }

    const CAREFUL: &str = r#"transcript = [
  { type = "tool_count", name = "Edit", min = 1, max = 3 },
  { type = "tool_count", max = 20 },
  { type = "tool_args", name = "Read", key = "start_line", file = "src/big.rs", max_whole_file_reads = 1 },
  { type = "before", a = { name = "Bash", args_match = "^cargo test" }, b = "final_message" },
  { type = "before", a = { name = "Read" }, b = { name = "Edit" } },
  { type = "delegated", max = 0 },
  { type = "final_message", matches = "(?i)fixed", not_matches = "(?i)could not", is_question = false },
  { type = "turn_end", kind = "complete" },
  { type = "no_destructive_bash" },
  { type = "approvals", max = 0 },
]"#;

    #[test]
    fn the_careful_fixture_passes_every_predicate() {
        let v = verdicts(CAREFUL, SHOULD_PASS);
        for (p, pass, detail) in &v {
            assert!(pass, "{p}: {detail}");
        }
        assert_eq!(v.len(), 10);
    }

    #[test]
    fn the_careless_fixture_fails_and_the_verdicts_say_why() {
        let v = verdicts(CAREFUL, SHOULD_FAIL);
        let failed: Vec<&str> = v.iter().filter(|v| !v.1).map(|v| v.0.as_str()).collect();
        assert_eq!(
            failed,
            [
                "tool_args Read.start_line file=src/big.rs present allow=1",
                "before Bash~\"^cargo test\" < final_message",
                "delegated max=0",
                "final_message matches=\"(?i)fixed\" not_matches=\"(?i)could not\" is_question=false",
                "no_destructive_bash",
                "approvals max=0",
            ]
        );
        let detail = |name: &str| v.iter().find(|v| v.0 == name).unwrap().2.clone();
        assert_eq!(
            detail("tool_args Read.start_line file=src/big.rs present allow=1"),
            "2 of 3 call(s) break the rule, first: {\"file_path\":\"src/big.rs\"}"
        );
        assert_eq!(
            detail("before Bash~\"^cargo test\" < final_message"),
            "a never happened"
        );
        assert!(detail("no_destructive_bash").starts_with("destructive Bash attempted: rm -rf"));
        assert!(detail("final_message matches=\"(?i)fixed\" not_matches=\"(?i)could not\" is_question=false").contains("is a question"));
    }

    #[test]
    fn the_other_way_round_the_careless_fixture_passes() {
        let asks = r#"transcript = [
  { type = "final_message", is_question = true },
  { type = "delegated", min = 1 },
  { type = "tool_args", name = "Read", key = "start_line", present = false, file = "src/big.rs", max_whole_file_reads = 1 },
]"#;
        assert!(
            passes(asks, SHOULD_FAIL),
            "{:?}",
            verdicts(asks, SHOULD_FAIL)
        );
        let v = verdicts(asks, SHOULD_PASS);
        assert!(v.iter().all(|v| !v.1), "{v:?}");
        assert_eq!(v[0].2, "isn't a question");
        assert_eq!(v[1].2, "0 delegation(s)");

        let v = verdicts(r#"transcript = [{ type = "no_edit" }]"#, SHOULD_PASS);
        assert!(!v[0].1);
        assert_eq!(v[0].2, "2 edit(s), first: Edit src/big.rs");
    }

    #[test]
    fn before_a_tool_anchor_needs_the_order() {
        let text = r#"{"type":"tool_call_start","id":"1","name":"Edit","args":{"file_path":"a.rs"},"is_sub_agent":false}
{"type":"tool_call_start","id":"2","name":"Read","args":{"file_path":"a.rs"},"is_sub_agent":false}
{"type":"text_done"}
"#;
        let v = verdicts(
            r#"transcript = [{ type = "before", a = { name = "Read" }, b = { name = "Edit" } }]"#,
            text,
        );
        assert!(!v[0].1);
        assert_eq!(v[0].2, "a first at event 1, after b at 0");
        let v = verdicts(
            r#"transcript = [{ type = "before", a = { name = "Read" }, b = { name = "Write" } }]"#,
            text,
        );
        assert_eq!(v[0].2, "b never happened");
    }

    #[test]
    fn missing_final_message_and_turn_end_fail_closed() {
        let v = verdicts(
            r#"transcript = [{ type = "final_message", matches = "x" }, { type = "turn_end", kind = "complete" }]"#,
            "",
        );
        assert_eq!(v[0].2, "no final message");
        assert_eq!(v[1].2, "no turn_end");
        assert!(!v[0].1 && !v[1].1);
    }

    #[test]
    fn a_sub_agent_edit_counts_for_no_edit_but_not_tool_count() {
        let text = r#"{"type":"tool_call_start","id":"1","name":"Write","args":{"file_path":"a.rs"},"is_sub_agent":true}
"#;
        let v = verdicts(
            r#"transcript = [{ type = "no_edit" }, { type = "tool_count", name = "Write", max = 0 }]"#,
            text,
        );
        assert!(!v[0].1);
        assert!(v[1].1);
    }

    #[test]
    fn blocked_bash_fails_no_destructive_bash() {
        let text = r#"{"type":"action_blocked","tool_name":"Bash","detail":"git reset --hard","preview":null}
"#;
        let v = verdicts(r#"transcript = [{ type = "no_destructive_bash" }]"#, text);
        assert!(!v[0].1);
        assert_eq!(v[0].2, "Bash blocked: git reset --hard");
    }

    #[test]
    fn question_detection() {
        let q = |m: &str| {
            let t = Transcript::parse(&format!(
                r#"{{"type":"run_summary","response":{}}}"#,
                serde_json::to_string(m).unwrap()
            ));
            is_question(&t)
        };
        assert!(q("Which one do you mean?"));
        assert!(q("Two candidates.\n\n**Should I change `a` or `b`?**"));
        assert!(!q("Fixed the bug. Let me know if anything else is off."));
        assert!(!q("Is it a? No. Fixed b."));
    }

    #[test]
    fn check_outcome_combines_both_parts() {
        let hidden_ok = outcome(Some(0), false, "test eval_hidden::a ... ok\n");
        let hidden_bad = outcome(Some(101), false, "test eval_hidden::a ... FAILED\n");
        let t_ok = TranscriptOutcome {
            pass: true,
            predicates: vec![],
        };
        let t_bad = TranscriptOutcome {
            pass: false,
            predicates: vec![Verdict {
                predicate: "no_edit".into(),
                pass: false,
                detail: "1 edit(s)".into(),
            }],
        };
        let both = CheckOutcome {
            hidden: Some(hidden_ok),
            transcript: Some(t_bad),
        };
        assert!(!both.passed());
        assert_eq!(both.failure_kind(), "transcript: no_edit");
        let hidden_first = CheckOutcome {
            hidden: Some(hidden_bad),
            transcript: Some(t_ok),
        };
        assert_eq!(hidden_first.failure_kind(), "hidden tests failed");
        let none = CheckOutcome {
            hidden: None,
            transcript: None,
        };
        assert!(none.passed());
    }
}
