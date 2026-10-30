//! Building `koda` from a git ref, cached by commit.
//!
//! A variant named by ref (`main`, `exp/auto-cargo-check`, a tag, a SHA)
//! is resolved in the koda checkout, unpacked with `git archive` into a
//! scratch dir, built in release mode, and the binary kept at
//! `~/.cache/koda-eval/bin/<sha>/koda`. The next run with the same commit
//! skips the build. All refs share one `CARGO_TARGET_DIR`, so dependencies
//! compile once.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::{project, util};

pub struct Built {
    pub sha: String,
    pub bin: PathBuf,
}

/// The full commit SHA a ref names in `koda_repo`.
pub fn resolve(koda_repo: &Path, git_ref: &str) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(koda_repo)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{git_ref}^{{commit}}"),
        ])
        .output()
        .context("run git rev-parse")?;
    if !out.status.success() {
        bail!(
            "{git_ref:?} is not a commit in {} (fetch it first?)",
            koda_repo.display()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The release `koda` for `git_ref`, building it if this commit hasn't
/// been built before.
pub fn ensure(koda_repo: &Path, git_ref: &str) -> Result<Built> {
    let sha = resolve(koda_repo, git_ref)?;
    let bin = util::cache_dir().join("bin").join(&sha).join("koda");
    if bin.is_file() {
        return Ok(Built { sha, bin });
    }

    let short = &sha[..sha.len().min(12)];
    eprintln!("building koda at {git_ref} ({short}) in release mode ...");
    let build = util::cache_dir().join("build").join(&sha);
    let _ = std::fs::remove_dir_all(&build);
    std::fs::create_dir_all(&build)?;
    project::archive(koda_repo, &sha, &build)?;
    project::stamp(&build)?;

    let target = util::cache_dir().join("koda-target");
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "koda-cli"])
        .current_dir(&build)
        .env("CARGO_TARGET_DIR", &target)
        // The harness's own lint settings must not leak into koda's build.
        .env_remove("RUSTFLAGS")
        .status()
        .context("run cargo build")?;
    if !status.success() {
        bail!(
            "cargo build failed for {git_ref} ({short}); the source is kept at {}",
            build.display()
        );
    }

    std::fs::create_dir_all(bin.parent().context("bin dir")?)?;
    std::fs::copy(target.join("release").join("koda"), &bin)
        .with_context(|| format!("copy the built koda to {}", bin.display()))?;
    let _ = std::fs::remove_dir_all(&build);
    Ok(Built { sha, bin })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_names_a_commit_or_fails_clearly() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        let git = |args: &[&str]| {
            let ok = Command::new("git")
                .arg("-C")
                .arg(repo)
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@localhost",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        std::fs::write(repo.join("a"), "a").unwrap();
        git(&["add", "a"]);
        git(&["commit", "-q", "-m", "one"]);

        let sha = resolve(repo, "main").unwrap();
        assert_eq!(sha.len(), 40);
        assert_eq!(resolve(repo, "HEAD").unwrap(), sha);
        let err = resolve(repo, "no-such-branch").unwrap_err();
        assert!(format!("{err:#}").contains("not a commit"), "{err:#}");
    }
}
