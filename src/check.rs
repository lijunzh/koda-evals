//! Running a task's check command and reading the result.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::Result;
use serde::Serialize;

use crate::proc;
use crate::task::Task;

#[derive(Debug, Serialize)]
pub struct CheckOutcome {
    pub exit: Option<i32>,
    pub timed_out: bool,
    /// Hidden tests (those under `eval_hidden`) that passed and failed.
    pub hidden_ok: u32,
    pub hidden_failed: u32,
    pub compile_error: bool,
    /// The last lines of the check's output.
    pub tail: String,
}

impl CheckOutcome {
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

/// Run the check command in `project`, writing its output to `log`.
pub fn run(task: &Task, project: &Path, target_dir: &Path, log: &Path) -> Result<CheckOutcome> {
    let out = File::create(log)?;
    let mut cmd = Command::new("sh");
    cmd.args(["-c", &task.spec.check.command])
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
pub fn not_run(reason: &str) -> CheckOutcome {
    CheckOutcome {
        exit: None,
        timed_out: false,
        hidden_ok: 0,
        hidden_failed: 0,
        compile_error: false,
        tail: reason.to_string(),
    }
}

fn outcome(exit: Option<i32>, timed_out: bool, text: &str) -> CheckOutcome {
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
    CheckOutcome {
        exit,
        timed_out,
        hidden_ok,
        hidden_failed,
        compile_error,
        tail: crate::util::tail(text, 30),
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
}
