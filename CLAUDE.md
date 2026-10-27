# CLAUDE.md

Guidance for agents working in this repo. The README covers how it works.

## Design docs

The design, goals and work sequence live in the maintainer's private notes,
not in this repo or in GitHub issues. On the maintainer's machine:
`~/repo/notes/10-projects/Koda/Koda - Eval Harness and Rust Tooling Design (Oct 2026).md`,
with status and open decisions in `Koda.md` next to it. That folder is an
Obsidian vault with its own `CLAUDE.md`; follow it when you write there. Don't
copy the notes into commits or public docs.

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
- **`results/` is not committed.** Conclusions go into the vault notes.
- **Style:** edition 2024, the toolchain pinned in `rust-toolchain.toml`,
  `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt`. Keep it small:
  this is a tool for one maintainer.
