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

/// How many times the end of a run looks for what's left, and how long it
/// gives a kill to land between looks.
const SWEEPS: u32 = 20;
const SWEEP_PAUSE: Duration = Duration::from_millis(50);

/// Run `cmd` in its own session and wait for it. Once `limit` passes its
/// process group gets SIGINT, so the run can write its summary. Then, on a
/// normal exit too, the whole session is killed. koda runs each Bash command
/// in a group of its own, and a job left behind there stays running when koda
/// exits, so killing koda's group isn't enough: the job could still change the
/// project under the hidden tests, or hold a port into the next run. Only a
/// job that starts its own session (`setsid`) gets past this.
pub fn run_with_limit(cmd: &mut Command, limit: Duration) -> Result<Finished> {
    run_with_grace(cmd, limit, INTERRUPT_GRACE)
}

fn run_with_grace(cmd: &mut Command, limit: Duration, grace: Duration) -> Result<Finished> {
    // SAFETY: `setsid` is async-signal-safe and touches no memory. It also
    // makes the child a group leader, so `signal_group` reaches it.
    unsafe {
        cmd.pre_exec(|| match libc::setsid() {
            -1 => Err(std::io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
    let start = Instant::now();
    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawn {:?}", cmd.get_program()))?;
    let pid = child.id();
    loop {
        if let Some(status) = child.try_wait()? {
            kill_session(pid);
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
            signal_group(pid, "KILL");
            let status = child.wait()?;
            kill_session(pid);
            return Ok(Finished {
                exit: status.code(),
                timed_out: true,
                elapsed: start.elapsed(),
            });
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Kill every process in session `sid`, the id of the process that led it.
/// `pkill -s` isn't on macOS. Jobs can fork while this runs, so look again.
fn kill_session(sid: u32) {
    for _ in 0..SWEEPS {
        let members = session_members(sid as libc::pid_t);
        if members.is_empty() {
            return;
        }
        for pid in members {
            // SAFETY: `kill` takes plain integers and touches no memory. It
            // fails harmlessly when the process is already gone.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
        std::thread::sleep(SWEEP_PAUSE);
    }
}

fn session_members(sid: libc::pid_t) -> Vec<libc::pid_t> {
    let Ok(listed) = Command::new("ps").args(["-A", "-o", "pid="]).output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&listed.stdout)
        .split_whitespace()
        .filter_map(|pid| pid.parse().ok())
        // SAFETY: `getsid` takes a plain integer and touches no memory.
        .filter(|&pid| unsafe { libc::getsid(pid) } == sid)
        .collect()
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

    /// A job in a group of its own, as `koda` leaves one: it wrote nothing by
    /// the time `run_with_limit` returned, and won't write later.
    fn leaves_no_job(script: &str, limit: Duration) {
        let tmp = tempfile::tempdir().unwrap();
        let written = tmp.path().join("written");
        let job = format!("set -m; (sleep 1; touch '{}') & ", written.display());
        let mut cmd = Command::new("sh");
        cmd.args(["-c", &format!("{job}{script}")]);
        run_with_limit(&mut cmd, limit).unwrap();
        std::thread::sleep(Duration::from_millis(1500));
        assert!(!written.exists());
    }

    #[test]
    fn a_job_in_its_own_group_ends_with_a_normal_exit() {
        leaves_no_job("exit 0", Duration::from_secs(10));
    }

    #[test]
    fn a_job_in_its_own_group_ends_with_a_timed_out_run() {
        leaves_no_job("sleep 30", Duration::from_millis(300));
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
