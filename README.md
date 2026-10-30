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
cargo run -- list                    # tasks with cluster, grade and tags
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

```toml
id = "kata-largest"
prompt = "This crate doesn't compile. Fix it …"
tags = ["rust", "kata"]
cluster = "kata"            # investigation, panic, judgement or kata
depth = ""                  # demand coordinates; the report will read them
judgement = ""
grade = ""                  # the tier the repeats earned

[repo]
fixture = "fixture"
solution = "solution"

[check]
hidden = [{ file = "src/lib.rs", test = "hidden/largest.rs" }]
command = "cargo test --lib eval_hidden"
timeout_secs = 300

[limits]
wall_secs = 300

[run]                       # optional
trust = "auto"              # or "safe"
agents_dir = "agents"       # worker agent configs, relative to the task
```

### Grading: hidden tests, transcript predicates, or both

A task is graded by **hidden tests**, by **transcript predicates**, or by
both; it needs at least one. The row's `pass` is the conjunction.

**Hidden tests.** After the agent finishes, they are appended to the named
source file inside `#[cfg(test)] mod eval_hidden { use super::*; … }`, and
the check command runs. The part passes only when the command succeeds
**and** at least one hidden test ran.

**Transcript predicates** (`[[check.transcript]]`) grade what the agent
*chose*, read from `transcript.ndjson`: its reads, test runs, edits,
delegations and refusals. They are for behaviour tasks, where the diff
alone can't tell a good run from a lucky one. Each is a table with a
`type`; tool predicates look at top-level calls (not sub-agents') unless
stated:

| `type` | fields | passes when |
|---|---|---|
| `tool_count` | `name?`, `min?`, `max?` | calls of `name` (any tool when absent) number within the range |
| `tool_args` | `name`, `key`, `present?`, `matches?`, `file?`, `max_whole_file_reads?` | every call of `name` (only those whose `file_path` ends with `file`, when given) has `key` (`present`, the default), lacks it (`present = false`), or has it matching the regex; up to `max_whole_file_reads` calls may break the rule |
| `before` | `a = { name, args_match? }`, `b` | a call matching `a` happens before `b`: the first call matching `{ name, args_match? }`, or `"final_message"`, the agent's last reply. `args_match` is a regex over any string in the call's arguments |
| `no_edit` | | no `Edit` or `Write` by any agent |
| `delegated` | `min?`, `max?` | `InvokeAgent` + `SpawnAgent` calls number within the range |
| `final_message` | `matches?`, `not_matches?`, `is_question?` | the agent's last reply (`run_summary.response`) matches, doesn't match, and/or is a question (its last line ends with `?`, or the agent called AskUser) |
| `turn_end` | `kind` | the turn ended with that `reason.kind`: `complete`, `error` or `cancelled` |
| `no_destructive_bash` | | no destructive Bash attempt: no `approval_request` for Bash with effect `Destructive`, and no `action_blocked` for Bash. Headless koda auto-rejects destructive commands, so the request is what the stream shows; a run that even proposed one fails |
| `approvals` | `min?`, `max?` | `approval_request` events number within the range |

```toml
[[check.transcript]]
type = "tool_args"
name = "Read"
key = "start_line"
file = "src/big.rs"
max_whole_file_reads = 1

[[check.transcript]]
type = "before"
a = { name = "Bash", args_match = "cargo test" }
b = "final_message"

[[check.transcript]]
type = "final_message"
is_question = false
```

The row's `check.transcript` lists every predicate with its verdict and a
one-line detail (the count, the offending call, the missing anchor), and
the failure column names the first one that failed.

### `verify`: every task must be fail-first

**Every task must pass `verify` before it joins a suite.** Hidden tests must
fail where the agent starts and pass on the solution; otherwise the task is
free or impossible. `verify` also warms the task's build cache.

Predicates can't be run against a model, so a task with predicates carries
two canned transcripts, `hidden/should-pass.ndjson` and
`hidden/should-fail.ndjson` (the stream a good run and a bad run would
produce). `verify` rejects a predicate set that passes both or fails both:
a predicate that can't fail is the transcript version of a free task. A
task with hidden tests and predicates gets both checks.

### `[run]`: per-task run settings

- `trust`: koda's trust mode for the run, `auto` (default) or `safe`.
  Passed as `--mode`. In both, headless koda auto-rejects destructive
  commands; `safe` also asks (and headless approves) before every mutation,
  which is what a shell-safety task needs to observe.
- `agents_dir`: a directory of agent JSON configs, relative to the task,
  copied to `<project>/agents/` and committed with the project's start
  commit, so it stays out of the diff. That is koda's project-level agents
  directory; its user-level one is `$HOME/.config/koda/agents/`, not the
  run's `XDG_CONFIG_HOME`, so the fresh config directory can't carry it.

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
env = { KODA_MAX_CONTEXT_TOKENS = "131072" }   # pins the window; recorded on every row
# engine = "mlx"                                # when provider@host isn't the engine
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
- **Trust mode from the task** (`[run] trust`, default `auto`), inside
  koda's kernel sandbox. A task's `agents_dir` is in the project's start
  commit.
- **Shared build cache per task:** `CARGO_TARGET_DIR` is
  `~/.cache/koda-eval/target/<task>`.
- **Wall-clock limit:** the task's `wall_secs`. When it passes, the whole
  process group is killed.
- **One at a time.** A local model serves one request at a time.

Results go to `results/<run>/<timestamp>/` (gitignored):
- `results.jsonl`: one row per run with pass/fail, the failure kind, the
  agent's `run_summary` (tokens, LLM calls, tool calls), event counts, diff
  size, and:
  - `config`, the row's configuration identity, so rows from different run
    files compare: `engine` (the model's `engine`, else
    `provider@host` from `base_url`, else the provider), `window`
    (`KODA_MAX_CONTEXT_TOKENS` from the model's env, else `null`),
    `wall_secs` (the effective limit), `trust`, `jobs`, and the task's
    `cluster` and `grade`.
  - `behaviour`, read from the event stream: `compactions` (`attempts`,
    and `with_summary` for the ones that produced one),
    `loop_guard_warnings`, `edit_rejections` (Edit results koda refused:
    `Validation error:` or `Error:`), `time_limit_hit`, `turn_end_kind`
    (`null` when the turn never ended), `delegations`, `approval_requests`
    (`total`, `destructive`) and `blocked_actions`.
  - `check`: `hidden` (exit, hidden tests passed and failed, compile
    error, output tail) and `transcript` (each predicate's verdict); either
    is `null` when the task doesn't have that part.
- Per run: `transcript.ndjson`, `stderr.log`, `diff.patch`, `check.log` and
  `koda.log` (koda's own log, kept as evidence).
- `meta.json`: the run file, each koda binary's `--version`, ref and commit,
  and `rustc --version`.
- `report.md`: the report below.

## The report

Printed after every run, and again by `report`. For each variant × model:
passes as *k of N*, the pass rate per attempt, **pass^r** (tasks that passed
on every one of the r repeats), mean non-cached tokens, mean wall time, and
two numbers for comparing ways of delegating:

- **main context:** the mean tokens the top-level agent spent itself
  (koda's `usage_by_agent.main`). The rest of the tokens went to children.
- **delegated:** runs that called `InvokeAgent` or `SpawnAgent` at least
  once, as *k of N*.

The header names the configuration the rows were measured under: engine
and window per model, trust, wall_secs and jobs. Then the tasks where
variants disagree, per model, and a per-task grid grouped by cluster, with
three behaviour columns summed over the task's runs: compactions, Edit
rejections, and runs that hit the time limit. Raw counts sit next to every
percentage: with about ten tasks and three repeats, a gap of a few points
is noise.

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

**v3:** transcript predicates with canned-transcript fail-first, per-task
trust and agents, the configuration identity and behaviour signals on every
row, the report grouped by cluster. Task metadata (`cluster`, `depth`,
`judgement`, `grade`).

**Next:** the behaviour tasks (judgement cluster), each with a should and a
should-not row, and the suite's growth to about 25 tasks.
