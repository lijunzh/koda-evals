# koda-evals

Runs [koda](https://github.com/lijunzh/koda) on fixed coding tasks and records
whether each task's hidden tests pass, with tokens and time. It compares koda
builds (a branch against `main`) and models (a local model against a remote
API) on the same tasks, so changes to koda are decided by measurement.

`koda-eval` drives prebuilt `koda` binaries through
`koda -p … --output-format stream-json`. It never links koda's crates, so any
two koda builds can be compared.

## Commands

```bash
cargo run -- list                    # tasks and their tags
cargo run -- verify [TASK...]        # fail-first check (all tasks by default)
cargo run -- run runs/<name>.toml    # run a run file, then print its report
cargo run -- report <name|dir>       # the report again: a run name (latest) or a results dir
```

History tasks and `ref` variants come from a koda checkout: `--koda-repo`
(or `KODA_REPO`), default `~/repo/koda`.

## Tasks

`tasks/<id>/task.toml`, plus the task's hidden tests and, for a kata, its
`fixture/` and `solution/` directories.

- **History tasks** (`koda_base` / `koda_fix`): a real koda bug. The agent
  starts at the commit before the fix.
- **Katas** (`fixture` / `solution`): a small Cargo project. `solution/` is
  copied over `fixture/` to get a correct answer.

After the agent finishes, the hidden tests are appended to the named source
file inside `#[cfg(test)] mod eval_hidden { use super::*; … }`, and the check
command runs. A task passes only when the command succeeds **and** at least
one hidden test ran.

**Every task must pass `verify` before it joins a suite.** Its hidden tests
must fail where the agent starts and pass on the solution. Otherwise the task
is free or impossible. `verify` also warms the task's build cache.

## Run files

`runs/<name>.toml` names the tasks (ids, or tags), the koda builds, the models
and the repeats:

```toml
name = "local-smoke"
tags = ["kata"]
repeats = 3

[[variants]]
name = "main"
ref = "main"                        # a branch, tag or commit in the koda checkout

[[variants]]
name = "auto-check"
ref = "exp/auto-cargo-check"

# or a prebuilt binary:
# [[variants]]
# name = "local-build"
# bin = "~/repo/koda/target/release/koda"

[[models]]
name = "local"
provider = "lm-studio"
base_url = "http://localhost:1234/v1"
model = "<model id>"
```

A `ref` variant is resolved to its commit, unpacked with `git archive`,
built in release mode and cached at `~/.cache/koda-eval/bin/<sha>/koda`; the
same commit is never built twice. All refs share one
`~/.cache/koda-eval/koda-target`, so dependencies compile once.

**API keys never go in a run file.** Each run gets an empty koda config
directory, so its koda has no keystore of its own. The harness hands keys to
it as environment variables, from two sources:

1. the harness's own environment (`ANTHROPIC_API_KEY` and so on), which wins;
2. the `apikey:*` rows of your real koda keystore
   (`~/.config/koda/db/koda.db`), so a key saved with koda's `/key` command
   works without exporting it anywhere. `--keystore <path>` points elsewhere.

The run log names the variables it passed, never their values.

## Each run

- **Fresh project:** a fixture copy, or `git archive` of a koda commit, as a
  new single-commit repo. Never a clone, so the real fix isn't in `git log`.
- **Fresh koda config:** `XDG_CONFIG_HOME` points to an empty directory. The
  run gets its own database and no global memory, skills or stored keys.
- **Trust mode `auto`**, inside koda's kernel sandbox.
- **Shared build cache per task:** `CARGO_TARGET_DIR` is
  `~/.cache/koda-eval/target/<task>`.
- **Wall-clock limit:** the task's `wall_secs`. When it passes, the whole
  process group is killed.
- **One at a time.** A local model serves one request at a time.

Results go to `results/<run>/<timestamp>/` (gitignored):
- `results.jsonl`: one row per run with pass/fail, the failure kind, the
  agent's `run_summary` (tokens, LLM calls, tool calls), event counts and
  diff size.
- Per run: `transcript.ndjson`, `stderr.log`, `diff.patch` and `check.log`.
- `meta.json`: the run file, each koda binary's `--version`, ref and commit,
  and `rustc --version`.
- `report.md`: the report below.

## The report

Printed after every run, and again by `report`. For each variant × model:
passes as *k of N*, the pass rate per attempt, **pass^r** (tasks that passed
on every one of the r repeats), mean non-cached tokens and mean wall time.
Then the tasks where variants disagree, per model, and a per-task grid. Raw
counts sit next to every percentage: with about ten tasks and three repeats,
a gap of a few points is noise.

## Tests

```bash
cargo test
# End to end through a real koda driven by its mock provider. The mock
# exists only under koda's test-support feature; build into a separate
# target dir so other koda builds don't replace it.
cargo build -p koda-cli --features koda-core/test-support \
  --manifest-path ~/repo/koda/Cargo.toml --target-dir ~/.cache/koda-eval/koda-test-support
KODA_EVAL_TEST_BIN=~/.cache/koda-eval/koda-test-support/debug/koda cargo test --test mock_e2e
```

A koda built without that feature treats `--provider mock` as unknown and
falls back to its default local server, so every run fails with "agent
failed" and no tokens.

## Status

**v1:** tasks, `verify`, the runner, `results.jsonl` and four seed tasks.

**v2:** variants built from git refs and cached by commit; repeats; the
report with pass^r and the disagreement table.

**Next:** a suite of about 10 tasks, growing to 20–30, with multi-file tasks
that can tell sub-agent designs apart.
