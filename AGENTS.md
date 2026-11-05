# AGENTS.md

Guidance for agents working in this repo. Read [README.md](README.md) for the
contract and [CONTRIBUTING.md](CONTRIBUTING.md) for setup, checks, and task
acceptance. Discuss design changes in GitHub issues or PRs; private maintainer
notes are not needed and must not be copied into commits or public docs.

## Rules

- **koda is a black box here.** Talk to it only through the `koda` binary and
  its `stream-json` output. Never depend on koda's crates. A change to the
  event JSON is a contract change between two repos. The one exception is
  `src/keys.rs`, which reads API keys from koda's keystore
  (`<config dir>/db/koda.db`, table `kv_store`, rows `apikey:<ENV_NAME>`) so
  isolated runs can authenticate; keep that the only place that looks inside.
- **A task joins a suite only after `verify` says it is fair.** Its hidden
  tests fail where the agent starts and pass on the solution.
- **Hidden tests must accept any correct answer.** Assert on behavior, not on
  one particular implementation, and keep the prompt or doc comment honest
  about what is expected. An unfair failure measures the task, not the agent.
- **No API keys in run files, results or logs.** Keys reach a run as
  environment variables, from the harness's environment or koda's keystore.
  Log variable names at most, never values.
- **`results/` is not committed.** Share redacted summaries and minimal
  reproductions in issues or PRs; keep raw logs and conversation databases local.
- **Style:** edition 2024, the toolchain pinned in `rust-toolchain.toml`,
  `cargo clippy --locked --all-targets -- -D warnings` clean, `cargo fmt`.
  Keep changes small and focused. Default checks and CI make no model calls;
  official model validation is coordinated by the maintainer.
