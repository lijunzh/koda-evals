# Contributing

Contributions to harness correctness, fair tasks, and reproducible comparisons
are welcome. For a larger change, open an issue describing the problem and
expected behavior before implementing it. Keep each PR focused on one change.

## Setup

Use macOS or Linux with Git, Python 3, a C compiler, and Rust installed through
rustup. `rust-toolchain.toml` selects the harness's Rust version and components
automatically. Windows is not currently supported: process cleanup and several
tests use Unix facilities.

```bash
git clone https://github.com/lijunzh/koda-evals.git
cd koda-evals
cargo run --locked -- list
cargo run --locked -- verify kata-largest
```

These commands list the task catalog and check one task against its broken and
solved stages. They need no Koda checkout, model server, API key, or subscription.
The first build downloads Rust components and Cargo dependencies as needed.

History tasks and variants built from Git refs also need a Koda checkout with
its history. Clone it once, then pass its location explicitly:

```bash
git clone https://github.com/lijunzh/koda.git ../koda
cargo run --locked -- --koda-repo ../koda verify bash-classifier-escapes
```

The [README](README.md) documents task fields, run files, grading, and reports.
All contribution requirements are in this repository; private maintainer notes
are not required.

## Local checks

Run the same checks as CI before submitting:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --locked -- list
```

The default tests use fixtures and stand-in programs, without calling models.
The optional `mock_e2e` test returns early unless `KODA_EVAL_TEST_BIN` points to
a Koda binary built with its mock provider. See [Tests](README.md#tests) for
that separate integration check. CI runs the default checks on Linux and macOS,
without model credentials or that optional binary.

## Harness changes

Koda is a black box: use its binary and its `koda exec --json` contract, without linking
its crates. The existing keystore reader in `src/keys.rs` is the only exception.
Wire-format changes need coordination with Koda and an updated sanitized
compatibility fixture. See [Tests](README.md#tests) for the capture command.

For a bug fix, describe a concrete before/after case and add a regression when
it can distinguish the bug from the corrected behavior. Report what you ran
and any checks you could not run. Keep changes to scoring and reporting
explicit, since they affect comparisons with earlier results.

## Tasks and grading

1. Put a new task under `tasks/<id>/`, using the format in the README. Include
   the prompt, metadata, limits, and either a fixture/solution pair or public
   Koda base/fix references. Name the source of adapted material and preserve
   any required license notices.
2. Test the behavior promised by the prompt, accepting alternative correct
   implementations. For transcript predicates, include both
   `hidden/should-pass.ndjson` and `hidden/should-fail.ndjson`.
3. Run `cargo run --locked -- verify <id>` (with `--koda-repo` for history
   tasks). Hidden checks must fail at the starting stage and pass on the
   solution; canned transcripts must produce their intended verdicts. Include
   the result in the PR.
4. Explain what the task measures and any overlap with existing tasks.
   Maintainers decide canonical `suite` membership and coordinate model
   validation. Changing suite membership or grading starts a new baseline.

**The suite is frozen for now.** Koda is being improved against the current 31
tasks, so a new task may wait, and it won't join `suite` until that work is
done. Fixes for a broken task or harness bug are welcome now.

You can submit a contribution without paid model runs. If you include your
own measurements, identify the task set, Koda commit, model/provider, context
limit, wall limit, repeats, and parallelism. Separate harness failures from
agent failures and passes. Official baseline runs are coordinated by the
maintainer; CI does not run them.

Tests and solutions in this repository are public. They are withheld from a
trial's initial project, rather than a private benchmark holdout. Treat scores
as public regression measurements.

## Running someone else's task

Read a task before you run it. `verify` and `run` execute its check command,
hidden tests and any build script on your machine, with your user's access and
no sandbox. Your API keys and other environment variables are kept from the
check command, but it can still read your files, including Koda's keystore. CI
is safe from this because it only runs `list`, not a task.

`run` also starts an agent on the task's prompt with your API keys, so a task
written to mislead the agent is another reason to read it first. For a task you
don't trust, use a disposable machine or container with no credentials.

## Sharing evidence

Keep API keys in your environment or Koda keystore, never in run files, issues,
PRs, or fixtures. Do not commit `results/`, conversation databases, private
notes, or raw run logs. Share a redacted summary and the smallest reproduction
needed; inspect transcripts and patches before sharing them. Small sanitized
fixtures belong beside the tests that use them.

Contributions are made under this repository's [MIT license](LICENSE).
