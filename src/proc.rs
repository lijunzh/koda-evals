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

/// Run `cmd` in its own process group and wait for it, killing the whole
/// group once `limit` passes. The group is also killed after a normal exit,
/// so nothing the child left running in the background outlives the run.
pub fn run_with_limit(cmd: &mut Command, limit: Duration) -> Result<Finished> {
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
    // Fails harmlessly when the group is already gone.
    let _ = Command::new("kill")
        .args(["-s", "KILL", "--", &format!("-{pid}")])
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
    fn kills_a_process_that_overruns() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 30"]);
        let done = run_with_limit(&mut cmd, Duration::from_millis(300)).unwrap();
        assert!(done.timed_out);
        assert!(done.elapsed < Duration::from_secs(10));
    }
}
