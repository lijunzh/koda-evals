//! Running a child process under a wall-clock limit.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

pub struct Finished {
    /// `None` when the process was killed by a signal.
    pub exit: Option<i32>,
    pub timed_out: bool,
    pub elapsed: Duration,
}

/// How long a run gets after SIGINT to report what it did. koda waits up to
/// 5 seconds for its turn to stop, then writes its summary.
const INTERRUPT_GRACE: Duration = Duration::from_secs(10);

/// Run `cmd` in its own process group and wait for it. Once `limit` passes
/// the group gets SIGINT, so the run can write its summary, then SIGKILL.
/// The group is also killed after a normal exit, so nothing the child left
/// running in the background outlives the run.
pub fn run_with_limit(cmd: &mut Command, limit: Duration) -> Result<Finished> {
    run_with_grace(cmd, limit, INTERRUPT_GRACE)
}

fn run_with_grace(cmd: &mut Command, limit: Duration, grace: Duration) -> Result<Finished> {
    cmd.process_group(0);
    let start = Instant::now();
    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawn {:?}", cmd.get_program()))?;
    let pid = child.id();
    loop {
        if let Some(status) = child.try_wait()? {
            kill_group(pid);
            return Ok(Finished {
                exit: status.code(),
                timed_out: false,
                elapsed: start.elapsed(),
            });
        }
        if start.elapsed() >= limit {
            signal_group(pid, "INT");
            let stop_by = Instant::now() + grace;
            while child.try_wait()?.is_none() && Instant::now() < stop_by {
                std::thread::sleep(Duration::from_millis(50));
            }
            kill_group(pid);
            let status = child.wait()?;
            return Ok(Finished {
                exit: status.code(),
                timed_out: true,
                elapsed: start.elapsed(),
            });
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn kill_group(pid: u32) {
    signal_group(pid, "KILL");
}

fn signal_group(pid: u32, signal: &str) {
    // Fails harmlessly when the group is already gone.
    let _ = Command::new("kill")
        .args(["-s", signal, "--", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_exit_code() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "exit 3"]);
        let done = run_with_limit(&mut cmd, Duration::from_secs(10)).unwrap();
        assert_eq!(done.exit, Some(3));
        assert!(!done.timed_out);
    }

    #[test]
    fn an_overrunning_process_can_finish_on_sigint() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "trap 'exit 7' INT; sleep 30 & wait"]);
        let done = run_with_limit(&mut cmd, Duration::from_millis(300)).unwrap();
        assert!(done.timed_out);
        assert_eq!(done.exit, Some(7));
        assert!(done.elapsed < Duration::from_secs(10));
    }

    #[test]
    fn kills_a_process_that_ignores_sigint() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "trap '' INT; sleep 30"]);
        let done = run_with_grace(
            &mut cmd,
            Duration::from_millis(300),
            Duration::from_millis(300),
        )
        .unwrap();
        assert!(done.timed_out);
        assert_eq!(done.exit, None);
        assert!(done.elapsed < Duration::from_secs(10));
    }
}
