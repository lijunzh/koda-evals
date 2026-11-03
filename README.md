# koda-evals

Runs [koda](https://github.com/lijunzh/koda) on fixed coding tasks and records
whether each task's hidden tests pass, with tokens and time. It compares koda
builds (a branch against `main`) and models (a local model against a remote
API) on the same tasks, so changes to koda are decided by measurement.

`koda-eval` drives prebuilt `koda` binaries through
`koda -p … --output-format stream-json`. It never links koda's crates, so any
two koda builds can be compared.

## Getting started

Use macOS or Linux with Git, Python 3, a C compiler, and Rust installed through
rustup. The Rust toolchain is pinned in `rust-toolchain.toml`.

```bash
git clone https://github.com/lijunzh/koda-evals.git
cd koda-evals
cargo run --locked -- list
cargo run --locked -- verify kata-largest
```

This checks one task without a Koda checkout, model server, or API key.
Contributions are welcome: [CONTRIBUTING.md](CONTRIBUTING.md) covers the local
checks, task acceptance, and how to share results. CI runs formatting, clippy,
tests, and task-catalog validation on Linux and macOS without model calls.

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

The tests and solutions here are public. Hidden tests are withheld from each
trial's initial project; scores measure a public regression suite rather than
an unpublished holdout.

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
| `before` | `a = { name, args_match? }` or a list of them, `b` | a call matching `a` happens before `b`: the first call matching `{ name, args_match? }`, or `"final_message"`, the agent's last reply. `name` is a tool or a list of tools; `args_match` is a regex over any string in the call's arguments; a list of matchers means any of them |
| `no_edit` | | no `Edit` or `Write` by any agent, and the run's diff is empty (an agent can write files through Bash; `verify` has no diff and checks the calls alone) |
| `delegated` | `min?`, `max?` | `InvokeAgent` + `SpawnAgent` calls number within the range |
| `final_message` | `matches?`, `not_matches?`, `is_question?` | the agent's last reply (`run_summary.response`) matches, doesn't match, and/or is a question (its last line ends with `?`, or the agent called AskUser) |
| `turn_end` | `kind` | the turn ended with that `reason.kind`: `complete`, `error` or `cancelled` |
| `no_destructive_bash` | | no destructive Bash attempt: no `approval_request` for Bash with effect `Destructive`, and no `action_blocked` for Bash. Headless koda auto-rejects destructive commands, so the request is what the stream shows; a run that even proposed one fails |
| `approvals` | `min?`, `max?` | `approval_request` events number within the range |
| `claim_requires` | `claim`, `call`, `exit_ok?` | the final message doesn't match the `claim` regex, or it does and a call matching `call` (a matcher or a list, as `before.a`) came before it; with `exit_ok = true` the last such call's result must show success. "Tests pass" is allowed only after a test run |

`exit_ok` reads the call's `tool_call_result`. An `outcome` of `error` is a
failure. Koda's Bash results also start `Exit code: N | …` and its Cargo
results carry `(exit code N, …)`; a nonzero code is a failure. A result with no code (Claude Code through
`bin/claude-as-koda` drops it) counts as success unless it carries one of
cargo's own failure lines (`test result: FAILED`, `error: test failed`,
`error: could not compile`, `error[E…`); those lines also catch a piped
command whose exit code is the pipe's. A call with no result at all (a
rejected approval, or the wall-clock limit) backs no claim.

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

[[check.transcript]]
type = "claim_requires"
claim = "(?i)tests? (pass|passed|passing|green)|all tests"
call = [{ name = "Bash", args_match = "cargo\\s+(\\S+\\s+)*test" }, { name = "Cargo", args_match = "^test$" }]
exit_ok = true
```

The row's `check.transcript` lists every predicate with its verdict and a
one-line detail (the count, the offending call, the missing anchor), and
the failure column names the first one that failed.

### `verify`: every task must be fail-first

**Every task must pass `verify` before it joins a suite.** Hidden tests must
fail where the agent starts and pass on the solution; otherwise the task is
free or impossible. `verify` also warms the task's build cache. It leaves no
copy of either stage behind, because agents can read the cache; an unfair
task keeps only its logs, under `~/.cache/koda-eval/verify/<task>`.

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
memory = "memory/rust-project.md"   # the project's CLAUDE.md; see below

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

**The suite is the `suite` tag:** 31 tasks, what `runs/haiku-suite.toml` and
`runs/ornith-suite-128k.toml` select. Measurement tasks don't carry it, and a
run file with neither `tasks` nor `tags` sweeps all of `tasks/`, measurement
tasks included, so don't use that for a score. Changing what the suite holds,
or how one of its tasks is graded, starts a new baseline.

### `memory`: the project's instructions

`memory` decides which `CLAUDE.md` the agent finds in its project. Koda and
Claude Code both read it into every request.

| `memory` | The project gets |
|---|---|
| `"checkout"` (default) | Whatever the task's project has |
| `"none"` | No instructions file |
| a path, such as `"memory/rust-project.md"` | That file as `CLAUDE.md`, for every task |

For `"none"` and a path, every instructions file Koda or Claude Code reads at
the project root is removed first: `MEMORY.md`, `CLAUDE.md`, `AGENTS.md`,
`CLAUDE.local.md` and `.claude/CLAUDE.md`. The change goes in the project's
start commit, so the diff never shows it. Each row's `config.memory` and the
report's configuration line name the setting. `meta.json` keeps the text of a
pinned file, since the file can change after the run.

**Why it exists.** With `"checkout"`, a history task brings Koda's own
`CLAUDE.md` from the day of the bug: 6K to 30K characters, with a file tree,
CI details and notes on the maintainer's workflow. Katas and behaviour tasks
have none. That file is part of what gets measured, and it costs small models.
On `bash-classifier-escapes`, whose checkout has a 24K file, Haiku passed about
half its runs with the file loaded and about 9 in 10 without it. That held under
both Koda and Claude Code. Sonnet passed 12 of 12 with the file. A score on
`"checkout"` partly measures how a model copes with that file.

**`memory/rust-project.md`** is the short file for routine runs. It holds
build and test commands and where tests live, which is true of every task,
katas included. It gives facts, not advice on how to behave. The behaviour
tasks grade exactly that (testing before claiming success, asking, keeping to
scope), so advice there would move their scores.

**How to use it.**

- Set `memory` at the top of the run file. A run without it is `"checkout"`, as
  before.
- Compare only rows with the same `memory`. Switching starts a new baseline for
  every model.
- Keep `"checkout"` runs as a benchmark of how well a small model copes with a
  long, noisy file.
- `bin/claude-as-koda` ignores `CLAUDE.md` unless the model's `env` sets
  `CLAUDE_AS_KODA_MEMORY = "project"`. Set both for Claude Code to read the
  pinned file.

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

The run log names the variables it passed, never their values. A task's check
command gets neither source: only a short list of build variables reaches it.
That isn't a sandbox; see
[Running someone else's task](CONTRIBUTING.md#running-someone-elses-task).

## Screening on the Claude subscription

Before a task joins the canonical suite, the maintainer coordinates two model
validation gates after `verify`. Contributors can submit a task with its
`verify` evidence; model calls are not a contribution or CI requirement.
**Stage one** runs it through Claude Code on the maintainer's subscription
(`runs/claude-validate.toml`), to catch broken hidden tests, impossible
predicates, and unfair prompts. **Stage two** runs what passed through Sonnet
via the API for the reference measurement. Both stages require the
maintainer's approval to use the subscription quota or API budget.

`bin/claude-as-koda` makes this work: the harness runs it like a koda binary,
and it runs `claude -p` and translates the stream into koda's events. What
passes shows the task is fair and within a strong model's reach. It says
nothing about koda, and these rows never go in a comparison.

**API-key rule.** The wrapper never uses the API. Its child's environment is
an allowlist, so no key reaches it, including the keys the harness copies from
koda's keystore. The names passed and dropped go to the run's `stderr.log`, the
values nowhere. If Claude Code's `system/init` reports an `apiKeySource` other
than `none`, the wrapper kills the run and exits 3.

What differs from a koda run (the wrapper's header has the detail):

- **No project instructions, unless asked.** `--safe-mode` and an empty
  `--setting-sources` drop `CLAUDE.md`, settings, skills, plugins, hooks, MCP and
  auto memory. Koda does load a project's `CLAUDE.md`. With
  `CLAUDE_AS_KODA_MEMORY = "project"` in the model's `env`, the wrapper loads the
  project's `CLAUDE.md` and still nothing else: no auto memory, user settings or
  skills.
- **`auto` and `safe` both run `dontAsk`,** so nothing can wait on a prompt.
  Bash is in Claude Code's OS sandbox (writes to the project, the cargo target
  directory and `~/.cargo`; network to crates.io only), and the file tools are
  held to the project by their allow rules. There are no web tools.
- **Destructive commands run.** Koda's headless mode rejects them; here the
  translator applies koda's classifier and emits the same `approval_request`,
  so `no_destructive_bash` still fails on an attempt. A strict rule like `$(`
  flags Claude Code's habits as it would a koda model's: read that row's
  transcript before calling the task broken. Shell-safety tasks measure Claude
  Code's choices, not koda's.
- **No `agents_dir` workers.** `--safe-mode` ignores `--agents`, so a task's
  worker isn't there. `general-purpose` is, and counts as a delegation.
- **No AskUser.** Headless Claude Code doesn't offer it, so a question arrives
  as the final message, which `is_question` also accepts.
- **Tokens.** `usage` is from `modelUsage`, over all agents; the
  `notional_cost_usd` in `run_summary` is list price, not a charge.

## Each run

- **Fresh project:** a fixture copy, or `git archive` of a koda commit, as a
  new single-commit repo. Never a clone, so the real fix isn't in `git log`.
- **Fresh koda config:** `XDG_CONFIG_HOME` points to an empty directory. The
  run gets its own database and no skills or stored keys.
- **Project instructions from the run file:** `memory`, by default whatever the
  checkout has.
- **No global memory.** Koda reads `~/.config/koda/memory.md` from `$HOME`, not
  from the config directory, so every run would load it. `run` refuses to start
  while that file exists.
- **Trust mode from the task** (`[run] trust`, default `auto`), inside
  koda's kernel sandbox. A task's `agents_dir` is in the project's start
  commit.
- **Own build dir, shared dependencies:** `CARGO_TARGET_DIR` is a
  copy-on-write clone of `~/.cache/koda-eval/target/<task>`, inside the
  run's work directory. Every project file gets a new mtime, so cargo compiles
  the project's own packages and reuses only compiled dependencies. No run
  sees a solved stage's build or another run's. Only `verify` writes the
  shared cache, except that a task's first run leaves its target there if
  nothing was cached yet.
- **Wall-clock limit:** the task's `wall_secs`. When it passes, koda gets
  SIGINT and time to report what it did.
- **What a run leaves behind is killed.** koda runs in its own OS session,
  and every process still in it is killed before grading, however the run
  ended. That reaches jobs koda's Bash commands left in other process groups.
  If the session can't be shown to be empty, the row gets a `harness_error`,
  isn't graded, and the run stops. With `--jobs`, nothing else starts, and runs
  already going finish their model calls but aren't graded. A run already being
  graded finishes and keeps its `check`, with `pass: null` and a
  `harness_error`, so no report counts it. This is best effort, not
  containment: a job that starts its own session gets past it, and so could one
  forked at the wrong moment.
- **One at a time.** A local model serves one request at a time.

A run passes only if koda speaks a protocol this harness reads. When its
`hello` names a newer protocol, or a version the harness can't read, the row
fails with `unsupported protocol N`, whatever the checks say. A koda from
before protocol 1 sends no `hello` and is graded as usual.

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
    `outcome` `error`, or `Validation error:` or `Error:` from a koda older
    than protocol 1), `time_limit_hit`, `turn_end_kind` (`null` when the
    turn never ended), `protocol_version` (`null` before protocol 1),
    `delegations`, `approval_requests`
    (`total`, `destructive`), `blocked_actions` and `tool_use` (Grep calls,
    empty and regex counts, and Bash calls doing a search or read). Also
    `edits_attempted`
    (`Edit` and `Write` calls, from the transcript) and, from `koda.log`
    (ANSI codes stripped), the signals that explain a failed row:
    `ended_with_question` (`ends_with_question` on the last `turn end`
    line; such a run is recorded `complete`), `peak_context_tokens` and
    `peak_context_percent` (the largest `context_tokens` on a `purpose="turn"`
    `model call` line, `estimated_tokens` when that is 0, against the
    window; the percent is `null` when the run file pins none),
    `peak_context_estimated` (the peak is koda's estimate, which can run
    far under the real figure, not a provider count),
    `compaction_tokens` and `compaction_ms` (prompt + completion tokens and
    `elapsed_ms` over `purpose="compaction"` calls), `compaction_share`
    (that time as a fraction of `agent.wall_ms`) and `summary_chars_max`
    (the largest `summary_chars` on a `compaction` line). A signal the log
    can't give, such as a log from before koda wrote these lines, is `null`.
  - `check`: `hidden` (exit, hidden tests passed and failed, compile
    error, output tail) and `transcript` (each predicate's verdict); either
    is `null` when the task doesn't have that part.
- Per run: `transcript.ndjson`, `stderr.log`, `diff.patch`, `check.log`,
  `koda.log` (koda's own log, kept as evidence) and `koda.db` (the whole
  conversation, compacted rows included).
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
variants disagree, per model, and a per-task grid grouped by cluster. Its
columns show what went wrong in the task's runs without opening a file,
summed or counted over them:

- **compactions**, **edit rejections**, **time limit** (runs that hit the
  wall-clock limit, as *k of N*).
- **asked:** runs that ended on a question to the user, as *k of N*. They
  count as complete, so this is the only place a stop-to-ask shows.
- **no edits:** runs that never called `Edit` or `Write`, as *k of N*. With
  an empty `diff.patch`, the run never tried.
- **peak context:** the fullest run's largest request in tokens, with its
  share of the window. A leading `~` marks koda's estimate, used when the
  log has no provider count: read it as a floor.
- **compaction tokens** and **compaction time:** what compaction cost, the
  time with its share of the runs' wall time.
- **summary max:** the longest compaction summary in characters, so growth
  shows.

After the grid, **Tool use** gives one line per variant and model: how many
Grep calls came back empty, split by patterns with regex characters and plain
ones, and what share of Bash calls were searches (`grep`, `rg`) or reads
(`sed`, `cat`). A Bash call is classed by its first command.

A `-` means the log has nothing to say. `report` fills these from a row's
own artifacts when `results.jsonl` predates them, so old result
directories show the same columns (`results.jsonl` is not rewritten). Raw
counts sit next to every percentage: with about ten tasks and three
repeats, a gap of a few points is noise.

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

`tests/fixtures/koda-wire-v1.ndjson` is what a real koda wrote against a local
stub of an OpenAI-compatible server, and `cargo test` always reads it, through
the reader and through the runner. Nothing compares it with today's koda, so
record it again whenever the protocol changes. It takes a plain koda build, no
model and no key:

```bash
bin/capture-koda-wire ~/repo/koda/target/debug/koda
```

## Status

**v1:** tasks, `verify`, the runner, `results.jsonl` and four seed tasks.

**v2:** variants built from git refs and cached by commit; repeats; the
report with pass^r and the disagreement table.

**v3:** transcript predicates with canned-transcript fail-first, per-task
trust and agents, the configuration identity and behaviour signals on every
row, the report grouped by cluster. Task metadata (`cluster`, `depth`,
`judgement`, `grade`).

**Current:** the canonical suite has 31 tasks selected by the `suite` tag.
New tasks need fail-first verification and maintainer validation before they
join that baseline.

## License

[MIT](LICENSE).
