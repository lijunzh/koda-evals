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

/// How many times the end of a run looks for what's left, how long it gives a
/// kill to land between looks, and how many empty looks in a row it takes to
/// believe them: `ps` isn't atomic, and a process can fork between being
/// listed and being asked about.
const SWEEPS: u32 = 20;
const SWEEP_PAUSE: Duration = Duration::from_millis(50);
const EMPTY_SWEEPS: u32 = 2;

/// A run's processes couldn't be shown to be gone. Whatever is left may touch
/// the project or the next run, so nothing after this can be trusted.
#[derive(Debug)]
pub struct CleanupFailed(String);

impl std::fmt::Display for CleanupFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cleanup failed: {}", self.0)
    }
}

impl std::error::Error for CleanupFailed {}

/// Run `cmd` in its own session and wait for it. Once `limit` passes its
/// process group gets SIGINT, so the run can write its summary. Then, on a
/// normal exit too, the whole session is killed. koda runs each Bash command
/// in a group of its own, and a job left behind there stays running when koda
/// exits, so killing koda's group isn't enough: the job could still change the
/// project under the hidden tests, or hold a port into the next run. Fails
/// with [`CleanupFailed`] if the session can't be shown to be empty. This is
/// best effort, not containment: a job that starts its own session (`setsid`)
/// gets past it, and so could one forked at the wrong moment.
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
            kill_session(pid)?;
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
            kill_session(pid)?;
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
/// `pkill -s` isn't on macOS.
fn kill_session(sid: u32) -> Result<()> {
    sweep(|| session_members(sid as libc::pid_t))
}

/// Kill what `list` finds until it has found nothing often enough to believe.
fn sweep(mut list: impl FnMut() -> Result<Vec<libc::pid_t>, CleanupFailed>) -> Result<()> {
    let mut empty = 0;
    let mut left = Vec::new();
    for _ in 0..SWEEPS {
        left = list()?;
        if left.is_empty() {
            empty += 1;
            if empty == EMPTY_SWEEPS {
                return Ok(());
            }
        } else {
            empty = 0;
        }
        for &pid in &left {
            // SAFETY: `kill` takes plain integers and touches no memory. A
            // kill that fails shows up as a process still listed.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
        std::thread::sleep(SWEEP_PAUSE);
    }
    Err(CleanupFailed(format!("still running after {SWEEPS} sweeps: {left:?}")).into())
}

/// The live processes in session `sid`. A zombie has already stopped, and
/// isn't ours to reap.
fn session_members(sid: libc::pid_t) -> Result<Vec<libc::pid_t>, CleanupFailed> {
    let listed = Command::new("ps")
        .args(["-A", "-o", "pid=,stat="])
        .output()
        .map_err(|e| CleanupFailed(format!("can't run ps: {e}")))?;
    if !listed.status.success() {
        let stderr = String::from_utf8_lossy(&listed.stderr);
        return Err(CleanupFailed(format!(
            "ps failed: {} {}",
            listed.status,
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let stat = fields.next()?;
            (!stat.starts_with('Z')).then_some(pid)
        })
        // SAFETY: `getsid` takes a plain integer and touches no memory.
        .filter(|&pid| unsafe { libc::getsid(pid) } == sid)
        .collect())
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

    fn leaves_no_job_after(script: &str) {
        let tmp = tempfile::tempdir().unwrap();
        let written = tmp.path().join("written");
        let script = script.replace("{written}", written.to_str().unwrap());
        let mut cmd = Command::new("sh");
        cmd.args(["-c", &script]);
        run_with_limit(&mut cmd, Duration::from_secs(10)).unwrap();
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
    fn a_job_a_short_lived_wrapper_left_ends_too() {
        leaves_no_job_after("set -m; (sh -c '(sleep 1; touch \"$0\") &' '{written}') & exit 0");
    }

    #[test]
    fn a_session_that_cant_be_listed_is_a_failed_cleanup() {
        let err = sweep(|| Err(CleanupFailed("ps failed".into()))).unwrap_err();
        assert!(err.is::<CleanupFailed>(), "{err:#}");
    }

    #[test]
    fn a_process_that_wont_die_is_a_failed_cleanup() {
        let mut stuck = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = stuck.id() as libc::pid_t;
        let err = sweep(|| Ok(vec![pid])).unwrap_err();
        let _ = stuck.wait();
        assert!(err.is::<CleanupFailed>(), "{err:#}");
        assert!(format!("{err:#}").contains(&pid.to_string()), "{err:#}");
    }

    #[test]
    fn one_empty_look_is_not_enough() {
        let mut looks = 0;
        sweep(|| {
            looks += 1;
            Ok(Vec::new())
        })
        .unwrap();
        assert_eq!(looks, EMPTY_SWEEPS);
    }

    #[test]
    fn a_process_seen_after_an_empty_look_starts_the_count_again() {
        let mut stuck = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = stuck.id() as libc::pid_t;
        let mut looks = 0;
        sweep(|| {
            looks += 1;
            Ok(if looks == 2 { vec![pid] } else { Vec::new() })
        })
        .unwrap();
        let _ = stuck.wait();
        assert_eq!(looks, 2 + EMPTY_SWEEPS);
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
