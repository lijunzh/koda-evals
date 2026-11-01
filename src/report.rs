//! `koda-eval report`: the comparison a run was made for.
//!
//! For each variant × model: passes as *k of N*, the pass rate per
//! attempt, pass^r (tasks that passed on every repeat), mean non-cached
//! tokens, the share of those spent in the main context, how many runs
//! delegated, and mean wall time. Then the tasks where variants disagree,
//! and a per-task grid, grouped by cluster, with the behaviour signals
//! that explain a failure (compactions, Edit rejections, time-limit hits).
//! The header names the configuration the rows were measured under. Raw
//! counts sit next to every percentage: with about ten tasks and three
//! repeats, a few points is noise.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// One `results.jsonl` row, reduced to what the report needs.
#[derive(Debug, Clone)]
struct Row {
    task: String,
    variant: String,
    model: String,
    /// `None` when the harness itself failed, so the row counts for nothing.
    pass: Option<bool>,
    tokens: Option<u64>,
    /// Tokens the top-level agent spent itself (koda's
    /// `usage_by_agent.main`); `tokens` minus this is what children spent.
    main_tokens: Option<u64>,
    /// Calls to a delegation tool, top level only.
    delegations: u64,
    wall_ms: Option<u64>,
    /// The task's cluster, from the row's `config`; `None` on rows written
    /// before the configuration identity existed.
    cluster: Option<String>,
    config: Config,
    /// Behaviour signals; `None` on rows written before they existed.
    compactions: Option<u64>,
    edit_rejections: Option<u64>,
    time_limit_hit: Option<bool>,
    /// The run stopped on a question to the user (koda's log); `None` when
    /// the log has no `turn end` line.
    ended_with_question: Option<bool>,
    /// `Edit` and `Write` calls; zero with an empty diff means it never tried.
    edits_attempted: Option<u64>,
    peak_context_tokens: Option<u64>,
    peak_context_percent: Option<f64>,
    compaction_tokens: Option<u64>,
    compaction_ms: Option<u64>,
    summary_chars_max: Option<u64>,
}

/// The configuration identity a row carries.
#[derive(Debug, Clone, Default, PartialEq)]
struct Config {
    engine: Option<String>,
    window: Option<u64>,
    wall_secs: Option<u64>,
    trust: Option<String>,
}

use crate::klog;
use crate::task::CLUSTERS;
use crate::transcript::{DELEGATION_TOOLS, Transcript};

fn non_cached_tokens(usage: &Value) -> Option<u64> {
    match (
        usage["prompt_tokens"].as_u64(),
        usage["completion_tokens"].as_u64(),
    ) {
        (Some(p), Some(c)) => Some(p + c),
        _ => None,
    }
}

/// The rows of a `results.jsonl`. With the results directory, a row written
/// before the trouble signals existed gets them from its own artifacts.
fn parse_rows(text: &str, dir: Option<&Path>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut v: Value =
            serde_json::from_str(line).with_context(|| format!("results.jsonl line {}", i + 1))?;
        if let Some(dir) = dir {
            backfill(&mut v, dir);
        }
        let summary = &v["summary"];
        let tokens = non_cached_tokens(&summary["usage"]);
        // Builds older than per-agent usage report none; with no children
        // the whole run is the main context.
        let main_tokens = non_cached_tokens(&summary["usage_by_agent"]["main"]).or(tokens);
        let delegations = DELEGATION_TOOLS
            .iter()
            .filter_map(|t| summary["tool_calls"][*t].as_u64())
            .sum();
        let config = &v["config"];
        let behaviour = &v["behaviour"];
        rows.push(Row {
            task: v["task"].as_str().unwrap_or("?").to_string(),
            variant: v["variant"].as_str().unwrap_or("?").to_string(),
            model: v["model"].as_str().unwrap_or("?").to_string(),
            pass: v["pass"].as_bool(),
            tokens,
            main_tokens,
            delegations,
            wall_ms: v["agent"]["wall_ms"].as_u64(),
            cluster: config["cluster"]
                .as_str()
                .filter(|c| !c.is_empty())
                .map(str::to_string),
            config: Config {
                engine: config["engine"].as_str().map(str::to_string),
                window: config["window"].as_u64(),
                wall_secs: config["wall_secs"].as_u64(),
                trust: config["trust"].as_str().map(str::to_string),
            },
            compactions: behaviour["compactions"]["attempts"].as_u64(),
            edit_rejections: behaviour["edit_rejections"].as_u64(),
            time_limit_hit: behaviour["time_limit_hit"].as_bool(),
            ended_with_question: behaviour["ended_with_question"].as_bool(),
            edits_attempted: behaviour["edits_attempted"].as_u64(),
            peak_context_tokens: behaviour["peak_context_tokens"].as_u64(),
            peak_context_percent: behaviour["peak_context_percent"].as_f64(),
            compaction_tokens: behaviour["compaction_tokens"].as_u64(),
            compaction_ms: behaviour["compaction_ms"].as_u64(),
            summary_chars_max: behaviour["summary_chars_max"].as_u64(),
        });
    }
    Ok(rows)
}

/// Fill the trouble signals a row lacks from its artifacts (`koda.log`,
/// `transcript.ndjson`), so an old results directory reports like a new one.
/// A field the row has, even a `null`, is kept; `results.jsonl` is not
/// rewritten.
fn backfill(row: &mut Value, dir: &Path) {
    let Some(art) = row["artifacts"].as_str().map(|a| dir.join(a)) else {
        return;
    };
    let wall_ms = row["agent"]["wall_ms"].as_u64().unwrap_or(0);
    let window = row["config"]["window"].as_u64();
    let mut found = serde_json::Map::new();
    if let Some(log) = klog::read(&art.join("koda.log"))
        && let Value::Object(fields) = log.to_json(window, wall_ms)
    {
        found = fields;
    }
    if let Ok(transcript) = Transcript::read(&art.join("transcript.ndjson")) {
        found.insert(
            "edits_attempted".into(),
            transcript.edits_attempted().into(),
        );
    }
    if let Some(behaviour) = row["behaviour"].as_object_mut() {
        for (key, value) in found {
            behaviour.entry(key).or_insert(value);
        }
    }
}

/// `(variant, model)` in first-seen order, which is the run file's order.
fn cells(rows: &[Row]) -> Vec<(String, String)> {
    let mut seen = Vec::new();
    for r in rows {
        let key = (r.variant.clone(), r.model.clone());
        if !seen.contains(&key) {
            seen.push(key);
        }
    }
    seen
}

#[derive(Debug, Default, PartialEq)]
struct CellStats {
    passed: u32,
    ran: u32,
    harness_errors: u32,
    /// Tasks whose every attempt passed, and the tasks attempted.
    tasks_all_passed: u32,
    tasks: u32,
    mean_tokens: Option<u64>,
    mean_main_tokens: Option<u64>,
    /// Runs that called a delegation tool at least once.
    delegated: u32,
    mean_wall_ms: Option<u64>,
}

fn cell_stats(rows: &[Row], variant: &str, model: &str) -> CellStats {
    let mine: Vec<&Row> = rows
        .iter()
        .filter(|r| r.variant == variant && r.model == model)
        .collect();
    let mut s = CellStats::default();
    let mut by_task: BTreeMap<&str, Vec<bool>> = BTreeMap::new();
    let (mut tok_sum, mut tok_n, mut wall_sum, mut wall_n) = (0u64, 0u64, 0u64, 0u64);
    let (mut main_sum, mut main_n) = (0u64, 0u64);
    for r in &mine {
        match r.pass {
            Some(p) => {
                s.ran += 1;
                s.passed += u32::from(p);
                s.delegated += u32::from(r.delegations > 0);
                by_task.entry(&r.task).or_default().push(p);
            }
            None => s.harness_errors += 1,
        }
        if let Some(t) = r.tokens {
            tok_sum += t;
            tok_n += 1;
        }
        if let Some(t) = r.main_tokens {
            main_sum += t;
            main_n += 1;
        }
        if let Some(w) = r.wall_ms {
            wall_sum += w;
            wall_n += 1;
        }
    }
    s.tasks = by_task.len() as u32;
    s.tasks_all_passed = by_task.values().filter(|v| v.iter().all(|&p| p)).count() as u32;
    s.mean_tokens = (tok_n > 0).then(|| tok_sum / tok_n);
    s.mean_main_tokens = (main_n > 0).then(|| main_sum / main_n);
    s.mean_wall_ms = (wall_n > 0).then(|| wall_sum / wall_n);
    s
}

/// Passes out of attempts for one task in one cell, as `k/n`; `-` when the
/// task never ran there.
fn task_cell(rows: &[Row], task: &str, variant: &str, model: &str) -> String {
    let (mut k, mut n) = (0, 0);
    for r in rows
        .iter()
        .filter(|r| r.task == task && r.variant == variant && r.model == model)
    {
        if let Some(p) = r.pass {
            n += 1;
            k += u32::from(p);
        }
    }
    if n == 0 {
        "-".to_string()
    } else {
        format!("{k}/{n}")
    }
}

fn fmt_secs(ms: Option<u64>) -> String {
    match ms {
        Some(ms) => {
            let s = ms / 1000;
            format!("{}m{:02}s", s / 60, s % 60)
        }
        None => "-".into(),
    }
}

fn fmt_num(n: Option<u64>) -> String {
    match n {
        Some(n) => {
            let s = n.to_string();
            let mut out = String::new();
            for (i, ch) in s.chars().enumerate() {
                if i > 0 && (s.len() - i) % 3 == 0 {
                    out.push(',');
                }
                out.push(ch);
            }
            out
        }
        None => "-".into(),
    }
}

/// A task's behaviour columns, summed over every row of the task:
/// compactions, Edit rejections, and time-limit hits as `k/n`. `-` when
/// no row carries the signal.
fn behaviour_cells(rows: &[Row], task: &str) -> [String; 3] {
    let mine = rows.iter().filter(|r| r.task == task);
    let (mut comp, mut rej, mut hits, mut n) = (None, None, 0u32, 0u32);
    for r in mine {
        if let Some(c) = r.compactions {
            comp = Some(comp.unwrap_or(0) + c);
        }
        if let Some(e) = r.edit_rejections {
            rej = Some(rej.unwrap_or(0) + e);
        }
        if let Some(hit) = r.time_limit_hit {
            n += 1;
            hits += u32::from(hit);
        }
    }
    [
        fmt_num(comp),
        fmt_num(rej),
        if n == 0 {
            "-".into()
        } else {
            format!("{hits}/{n}")
        },
    ]
}

/// The columns that show how a task's runs went wrong, over every row of
/// the task: runs that ended on a question and runs that never attempted an
/// edit as `k/n`, the peak context (the fullest run's, with its share of the
/// window), compaction tokens and time summed (with the share of the runs'
/// wall time), and the largest summary in characters. `-` when no row
/// carries the signal.
fn trouble_cells(rows: &[Row], task: &str) -> [String; 6] {
    let mine: Vec<&Row> = rows.iter().filter(|r| r.task == task).collect();
    let k_of_n = |flags: Vec<bool>| {
        if flags.is_empty() {
            "-".to_string()
        } else {
            format!("{}/{}", flags.iter().filter(|&&f| f).count(), flags.len())
        }
    };
    let asked = k_of_n(mine.iter().filter_map(|r| r.ended_with_question).collect());
    let no_edits = k_of_n(
        mine.iter()
            .filter_map(|r| r.edits_attempted.map(|n| n == 0))
            .collect(),
    );
    // The fullest run by share of its window when any row has one, else by
    // tokens: 50K in a 200K window is less trouble than 30K in a 32K one.
    let peak = mine
        .iter()
        .filter(|r| r.peak_context_tokens.is_some())
        .max_by(|a, b| {
            let key = |r: &Row| (r.peak_context_percent, r.peak_context_tokens);
            key(a)
                .partial_cmp(&key(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or_else(
            || "-".to_string(),
            |r| match r.peak_context_percent {
                Some(p) => format!("{} ({p:.0}%)", fmt_num(r.peak_context_tokens)),
                None => fmt_num(r.peak_context_tokens),
            },
        );
    let sum = |values: Vec<u64>| (!values.is_empty()).then(|| values.iter().sum::<u64>());
    let compaction_tokens = sum(mine.iter().filter_map(|r| r.compaction_tokens).collect());
    let compaction_ms = sum(mine.iter().filter_map(|r| r.compaction_ms).collect());
    // Share of the wall time of the runs that have both figures.
    let (ms, wall) = mine
        .iter()
        .filter_map(|r| Some((r.compaction_ms?, r.wall_ms?)))
        .fold((0u64, 0u64), |(ms, wall), (m, w)| (ms + m, wall + w));
    let compaction_time = match compaction_ms {
        Some(total) if wall > 0 => format!(
            "{} ({:.0}%)",
            fmt_secs(Some(total)),
            100.0 * ms as f64 / wall as f64
        ),
        total => fmt_secs(total),
    };
    let summary_max = mine.iter().filter_map(|r| r.summary_chars_max).max();
    [
        asked,
        no_edits,
        peak,
        fmt_num(compaction_tokens),
        compaction_time,
        fmt_num(summary_max),
    ]
}

/// Distinct values in first-seen order, joined with `/`.
fn distinct<T: PartialEq + ToString>(values: impl Iterator<Item = T>) -> String {
    let mut seen: Vec<T> = Vec::new();
    for v in values {
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    seen.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("/")
}

/// The configuration line of the header: engine and window per model,
/// then trust, wall_secs and jobs. Empty when no row carries a config.
fn configuration(rows: &[Row], jobs: u64) -> String {
    let with: Vec<&Row> = rows
        .iter()
        .filter(|r| r.config != Config::default())
        .collect();
    if with.is_empty() {
        return String::new();
    }
    let mut models: Vec<&str> = Vec::new();
    for r in &with {
        if !models.contains(&r.model.as_str()) {
            models.push(&r.model);
        }
    }
    let per_model: Vec<String> = models
        .iter()
        .map(|m| {
            let mine = with.iter().filter(|r| r.model == *m);
            let engine = distinct(
                mine.clone()
                    .map(|r| r.config.engine.clone().unwrap_or_else(|| "?".into())),
            );
            let window = distinct(mine.map(|r| {
                r.config
                    .window
                    .map_or_else(|| "provider default".to_string(), |w| format!("{w} tokens"))
            }));
            format!("{m}: {engine}, window {window}")
        })
        .collect();
    let trust = distinct(
        with.iter()
            .map(|r| r.config.trust.clone().unwrap_or_else(|| "?".into())),
    );
    let wall = distinct(with.iter().filter_map(|r| r.config.wall_secs));
    format!(
        "Configuration: {} · trust {trust} · wall_secs {wall} · jobs {jobs}\n\n",
        per_model.join("; ")
    )
}

/// Render the report for a results directory as Markdown.
fn render(rows: &[Row], meta: &Value, dir_name: &str) -> String {
    let mut out = String::new();
    let run = meta["run"].as_str().unwrap_or("run");
    let started = meta["started"].as_str().unwrap_or(dir_name);
    let tasks: BTreeSet<&str> = rows.iter().map(|r| r.task.as_str()).collect();
    let repeats = rows
        .iter()
        .fold(BTreeMap::<(&str, &str, &str), u32>::new(), |mut m, r| {
            *m.entry((&r.task, &r.variant, &r.model)).or_default() += 1;
            m
        })
        .values()
        .copied()
        .max()
        .unwrap_or(1);
    let jobs = meta["jobs"].as_u64().unwrap_or(1);
    let _ = writeln!(
        out,
        "# {run} · {started} · {} task(s) · {repeats} repeat(s){}\n",
        tasks.len(),
        if jobs > 1 {
            format!(" · {jobs} runs at a time (wall times contended)")
        } else {
            String::new()
        }
    );
    out.push_str(&configuration(rows, jobs));

    let cells = cells(rows);
    let _ = writeln!(
        out,
        "| variant | model | passed | pass rate | pass^{repeats} (tasks) | mean tokens | main context | delegated | mean time |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|");
    for (variant, model) in &cells {
        let s = cell_stats(rows, variant, model);
        let rate = if s.ran > 0 {
            format!("{:.0}%", 100.0 * f64::from(s.passed) / f64::from(s.ran))
        } else {
            "-".into()
        };
        let errs = if s.harness_errors > 0 {
            format!(" (+{} harness errors)", s.harness_errors)
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "| {variant} | {model} | {}/{}{errs} | {rate} | {}/{} | {} | {} | {}/{} | {} |",
            s.passed,
            s.ran,
            s.tasks_all_passed,
            s.tasks,
            fmt_num(s.mean_tokens),
            fmt_num(s.mean_main_tokens),
            s.delegated,
            s.ran,
            fmt_secs(s.mean_wall_ms)
        );
    }
    let _ = writeln!(
        out,
        "\nTokens are prompt + completion, cache reads excluded. Main context is the part the top-level agent spent itself, the rest went to children. Delegated counts runs that called {} at least once. pass^{repeats} counts tasks that passed on every repeat.",
        DELEGATION_TOOLS.join(" or ")
    );

    // Disagreements: per model, tasks where the variants' pass counts differ.
    let variants: Vec<&str> = {
        let mut v = Vec::new();
        for (variant, _) in &cells {
            if !v.contains(&variant.as_str()) {
                v.push(variant.as_str());
            }
        }
        v
    };
    let models: Vec<&str> = {
        let mut v = Vec::new();
        for (_, model) in &cells {
            if !v.contains(&model.as_str()) {
                v.push(model.as_str());
            }
        }
        v
    };
    let _ = writeln!(out, "\n## Tasks where variants disagree\n");
    if variants.len() < 2 {
        let _ = writeln!(out, "One variant; nothing to compare.");
    } else {
        let mut any = false;
        for model in &models {
            let mut lines = Vec::new();
            for task in &tasks {
                let results: Vec<String> = variants
                    .iter()
                    .map(|v| task_cell(rows, task, v, model))
                    .collect();
                let distinct: BTreeSet<&String> = results.iter().collect();
                if distinct.len() > 1 {
                    let parts: Vec<String> = variants
                        .iter()
                        .zip(&results)
                        .map(|(v, r)| format!("{v} {r}"))
                        .collect();
                    lines.push(format!("| {task} | {} |", parts.join(" · ")));
                }
            }
            if !lines.is_empty() {
                any = true;
                let _ = writeln!(out, "**{model}**\n\n| task | per variant |\n|---|---|");
                for l in lines {
                    let _ = writeln!(out, "{l}");
                }
                let _ = writeln!(out);
            }
        }
        if !any {
            let _ = writeln!(out, "None: every variant passed the same tasks.");
        }
    }

    // The full grid, one table per cluster, with the behaviour columns
    // summed over the task's rows.
    let _ = writeln!(out, "\n## Per task\n");
    let cluster_of = |task: &str| -> Option<String> {
        rows.iter()
            .find(|r| r.task == task)
            .and_then(|r| r.cluster.clone())
    };
    let mut groups: Vec<(Option<&str>, Vec<&str>)> = CLUSTERS
        .iter()
        .map(|c| (Some(*c), Vec::new()))
        .chain(std::iter::once((None, Vec::new())))
        .collect();
    for task in &tasks {
        let cluster = cluster_of(task);
        let slot = groups
            .iter_mut()
            .find(|(c, _)| *c == cluster.as_deref())
            .unwrap_or_else(|| unreachable!("every task lands in a cluster or in none"));
        slot.1.push(task);
    }
    groups.retain(|(_, tasks)| !tasks.is_empty());
    let header: Vec<String> = cells.iter().map(|(v, m)| format!("{v} / {m}")).collect();
    for (cluster, group) in &groups {
        if groups.len() > 1 || cluster.is_some() {
            let _ = writeln!(out, "**{}**\n", cluster.unwrap_or("unclustered"));
        }
        let _ = writeln!(
            out,
            "| task | {} | compactions | edit rejections | time limit | asked | no edits | peak context | compaction tokens | compaction time | summary max |",
            header.join(" | ")
        );
        let _ = writeln!(
            out,
            "|---|{}---|---|---|---|---|---|---|---|---|",
            "---|".repeat(cells.len())
        );
        for task in group {
            let row: Vec<String> = cells
                .iter()
                .map(|(v, m)| task_cell(rows, task, v, m))
                .collect();
            let [comp, rej, limit] = behaviour_cells(rows, task);
            let trouble = trouble_cells(rows, task);
            let _ = writeln!(
                out,
                "| {task} | {} | {comp} | {rej} | {limit} | {} |",
                row.join(" | "),
                trouble.join(" | ")
            );
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(
        out,
        "Compactions and edit rejections are summed over the task's runs; time limit is the runs that hit the wall-clock limit. Asked is the runs that ended on a question to the user, which count as complete; no edits is the runs that never called Edit or Write. Peak context is the fullest run's largest request, with its share of the window when the run file pins one. Compaction tokens (prompt + completion) and time are summed over the task's runs, the time with its share of their wall time. Summary max is the longest compaction summary in characters. `-` means the run's log has nothing to say."
    );
    out
}

/// Turn what the user typed into a results directory: a directory holding
/// `results.jsonl`, or a run name (latest timestamp under `results/`).
pub fn locate(root: &Path, what: &Path) -> Result<PathBuf> {
    if what.join("results.jsonl").is_file() {
        return Ok(what.to_path_buf());
    }
    let candidates = [root.join("results").join(what), what.to_path_buf()];
    for run_dir in candidates {
        if run_dir.is_dir() {
            let mut stamps: Vec<PathBuf> = std::fs::read_dir(&run_dir)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.join("results.jsonl").is_file())
                .collect();
            stamps.sort();
            if let Some(latest) = stamps.pop() {
                return Ok(latest);
            }
        }
    }
    bail!(
        "{} is neither a results directory nor a run name under {}",
        what.display(),
        root.join("results").display()
    )
}

/// Print the report for `dir` and write it there as `report.md`.
pub fn report(dir: &Path) -> Result<()> {
    let text = std::fs::read_to_string(dir.join("results.jsonl"))
        .with_context(|| format!("read {}", dir.join("results.jsonl").display()))?;
    let rows = parse_rows(&text, Some(dir))?;
    if rows.is_empty() {
        bail!("{} holds no results", dir.display());
    }
    let meta: Value = std::fs::read_to_string(dir.join("meta.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);
    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("results");
    let md = render(&rows, &meta, dir_name);
    print!("{md}");
    std::fs::write(dir.join("report.md"), &md)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(task: &str, variant: &str, pass: Option<bool>, tokens: u64) -> Row {
        Row {
            task: task.into(),
            variant: variant.into(),
            model: "m".into(),
            pass,
            tokens: Some(tokens),
            main_tokens: Some(tokens),
            delegations: 0,
            wall_ms: Some(60_000),
            cluster: None,
            config: Config::default(),
            compactions: None,
            edit_rejections: None,
            time_limit_hit: None,
            ended_with_question: None,
            edits_attempted: None,
            peak_context_tokens: None,
            peak_context_percent: None,
            compaction_tokens: None,
            compaction_ms: None,
            summary_chars_max: None,
        }
    }

    #[test]
    fn stats_count_passes_and_tasks_that_always_pass() {
        let rows = vec![
            row("a", "main", Some(true), 100),
            row("a", "main", Some(true), 300),
            row("b", "main", Some(true), 100),
            row("b", "main", Some(false), 100),
            row("c", "main", None, 0),
        ];
        let s = cell_stats(&rows, "main", "m");
        assert_eq!(s.passed, 3);
        assert_eq!(s.ran, 4);
        assert_eq!(s.harness_errors, 1);
        assert_eq!((s.tasks_all_passed, s.tasks), (1, 2));
        assert_eq!(s.mean_tokens, Some(120));
        assert_eq!(s.mean_wall_ms, Some(60_000));
    }

    #[test]
    fn report_lists_disagreements_only_where_variants_differ() {
        let rows = vec![
            row("a", "main", Some(true), 100),
            row("a", "exp", Some(true), 100),
            row("b", "main", Some(false), 100),
            row("b", "exp", Some(true), 100),
        ];
        let md = render(&rows, &Value::Null, "x");
        assert!(md.contains("| main | m | 1/2 | 50% | 1/2 |"), "{md}");
        assert!(md.contains("| exp | m | 2/2 | 100% | 2/2 |"), "{md}");
        assert!(md.contains("| b | main 0/1 · exp 1/1 |"), "{md}");
        assert!(!md.contains("| a | main 1/1 · exp 1/1 |"), "{md}");
        assert!(md.contains("| a | 1/1 | 1/1 | - | - | - |"), "{md}");
        // Rows without a configuration identity: no header line, one
        // unlabelled grid.
        assert!(!md.contains("Configuration:"), "{md}");
        assert!(!md.contains("**unclustered**"), "{md}");
    }

    #[test]
    fn config_and_behaviour_show_in_the_header_and_the_grid() {
        let text = r#"{"task":"kata-largest","variant":"v","model":"flint","pass":true,"config":{"engine":"lm-studio@localhost:1234","window":131072,"wall_secs":300,"trust":"auto","jobs":1,"cluster":"kata","grade":""},"behaviour":{"compactions":{"attempts":2,"with_summary":1},"edit_rejections":3,"time_limit_hit":false}}
{"task":"kata-largest","variant":"v","model":"flint","pass":false,"config":{"engine":"lm-studio@localhost:1234","window":131072,"wall_secs":300,"trust":"auto","jobs":1,"cluster":"kata","grade":""},"behaviour":{"compactions":{"attempts":1,"with_summary":1},"edit_rejections":0,"time_limit_hit":true}}
{"task":"bash-safety","variant":"v","model":"flint","pass":true,"config":{"engine":"lm-studio@localhost:1234","window":131072,"wall_secs":600,"trust":"safe","jobs":1,"cluster":"judgement","grade":""},"behaviour":{"compactions":{"attempts":0,"with_summary":0},"edit_rejections":0,"time_limit_hit":false}}
{"task":"old-row","variant":"v","model":"sonnet","pass":true}
"#;
        let rows = parse_rows(text, None).unwrap();
        assert_eq!(rows[0].cluster.as_deref(), Some("kata"));
        assert_eq!(rows[0].config.window, Some(131_072));
        assert_eq!(rows[1].time_limit_hit, Some(true));
        assert_eq!(rows[3].cluster, None);
        let md = render(&rows, &Value::Null, "x");
        assert!(
            md.contains("Configuration: flint: lm-studio@localhost:1234, window 131072 tokens · trust auto/safe · wall_secs 300/600 · jobs 1"),
            "{md}"
        );
        let judgement = md.find("**judgement**").unwrap();
        let kata = md.find("**kata**").unwrap();
        let unclustered = md.find("**unclustered**").unwrap();
        assert!(judgement < kata && kata < unclustered, "{md}");
        assert!(
            md.contains("| kata-largest | 1/2 | - | 3 | 3 | 1/2 |"),
            "{md}"
        );
        assert!(
            md.contains("| bash-safety | 1/1 | - | 0 | 0 | 0/1 |"),
            "{md}"
        );
        assert!(md.contains("| old-row | - | 1/1 | - | - | - |"), "{md}");
    }

    #[test]
    fn trouble_shows_in_the_grid_and_missing_signals_show_a_dash() {
        let text = r#"{"task":"asks","variant":"v","model":"m","pass":false,"agent":{"wall_ms":100000},"behaviour":{"compactions":{"attempts":2},"edit_rejections":0,"time_limit_hit":false,"ended_with_question":true,"edits_attempted":2,"peak_context_tokens":57000,"peak_context_percent":89.1,"compaction_tokens":20000,"compaction_ms":45000,"compaction_share":0.45,"summary_chars_max":12000}}
{"task":"idle","variant":"v","model":"m","pass":false,"agent":{"wall_ms":1200000},"behaviour":{"compactions":{"attempts":0},"edit_rejections":0,"time_limit_hit":true,"ended_with_question":false,"edits_attempted":0,"peak_context_tokens":30000,"peak_context_percent":null,"compaction_tokens":0,"compaction_ms":0,"compaction_share":0.0,"summary_chars_max":null}}
{"task":"idle","variant":"v","model":"m","pass":true,"agent":{"wall_ms":300000},"behaviour":{"compactions":{"attempts":0},"edit_rejections":0,"time_limit_hit":false,"ended_with_question":false,"edits_attempted":3,"peak_context_tokens":20000,"peak_context_percent":null,"compaction_tokens":0,"compaction_ms":0,"summary_chars_max":null}}
{"task":"old-row","variant":"v","model":"m","pass":true,"agent":{"wall_ms":1000},"behaviour":{"compactions":{"attempts":1},"edit_rejections":0,"time_limit_hit":false}}
"#;
        let rows = parse_rows(text, None).unwrap();
        assert_eq!(rows[0].ended_with_question, Some(true));
        assert_eq!(rows[0].peak_context_percent, Some(89.1));
        assert_eq!(rows[3].edits_attempted, None);
        let md = render(&rows, &Value::Null, "x");
        assert!(
            md.contains("| asks | 0/1 | 2 | 0 | 0/1 | 1/1 | 0/1 | 57,000 (89%) | 20,000 | 0m45s (45%) | 12,000 |"),
            "{md}"
        );
        // Two runs: one never edited; the fullest by tokens when no run
        // has a percent; no compaction at all is 0, not `-`.
        assert!(
            md.contains("| idle | 1/2 | 0 | 0 | 1/2 | 0/2 | 1/2 | 30,000 | 0 | 0m00s (0%) | - |"),
            "{md}"
        );
        assert!(
            md.contains("| old-row | 1/1 | 1 | 0 | 0/1 | - | - | - | - | - | - |"),
            "{md}"
        );
    }

    #[test]
    fn an_old_row_gets_its_signals_from_its_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let art = dir.path().join("t").join("v.m.r1");
        std::fs::create_dir_all(&art).unwrap();
        std::fs::write(
            art.join("koda.log"),
            "2026-11-01T11:17:57Z INFO koda_core::inference_helpers: model call purpose=\"turn\" context_tokens=32000\n\
             2026-11-01T11:20:53Z INFO koda_core::inference: turn end ends_with_question=true\n",
        )
        .unwrap();
        std::fs::write(
            art.join("transcript.ndjson"),
            "{\"type\":\"tool_call_start\",\"id\":\"a\",\"name\":\"Write\",\"args\":{},\"is_sub_agent\":false}\n",
        )
        .unwrap();
        let text = r#"{"task":"t","variant":"v","model":"m","pass":false,"artifacts":"t/v.m.r1","config":{"window":64000},"agent":{"wall_ms":100000},"behaviour":{"compactions":{"attempts":0},"summary_chars_max":7}}
{"task":"u","variant":"v","model":"m","pass":false,"artifacts":"u/v.m.r1","behaviour":{"compactions":{"attempts":0}}}
"#;
        let rows = parse_rows(text, Some(dir.path())).unwrap();
        assert_eq!(rows[0].ended_with_question, Some(true));
        assert_eq!(rows[0].edits_attempted, Some(1));
        assert_eq!(rows[0].peak_context_tokens, Some(32_000));
        assert_eq!(rows[0].peak_context_percent, Some(50.0));
        // What the row already says stays.
        assert_eq!(rows[0].summary_chars_max, Some(7));
        // No artifacts on disk: nothing to say.
        assert_eq!(rows[1].ended_with_question, None);
        assert_eq!(rows[1].edits_attempted, None);
    }

    #[test]
    fn one_variant_has_nothing_to_compare() {
        let rows = vec![row("a", "main", Some(true), 100)];
        let md = render(&rows, &Value::Null, "x");
        assert!(md.contains("One variant; nothing to compare."), "{md}");
    }

    #[test]
    fn parses_rows_and_tolerates_harness_errors() {
        let text = r#"{"task":"a","variant":"v","model":"m","pass":true,"agent":{"wall_ms":1500},"summary":{"usage":{"prompt_tokens":10,"completion_tokens":5}}}
{"task":"a","variant":"v","model":"m","pass":null,"harness_error":"boom"}
"#;
        let rows = parse_rows(text, None).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].tokens, Some(15));
        assert_eq!(rows[1].pass, None);
        assert_eq!(rows[1].tokens, None);
        // No per-agent usage: the whole run counts as main context.
        assert_eq!(rows[0].main_tokens, Some(15));
        assert_eq!(rows[0].delegations, 0);
    }

    #[test]
    fn splits_main_context_from_children_and_counts_delegation() {
        let text = r#"{"task":"a","variant":"v","model":"m","pass":true,"summary":{"usage":{"prompt_tokens":900,"completion_tokens":100},"usage_by_agent":{"main":{"prompt_tokens":250,"completion_tokens":50},"explore":{"prompt_tokens":650,"completion_tokens":50}},"tool_calls":{"Read":3,"InvokeAgent":2}}}
{"task":"a","variant":"v","model":"m","pass":false,"summary":{"usage":{"prompt_tokens":100,"completion_tokens":0},"tool_calls":{"Read":1}}}
"#;
        let rows = parse_rows(text, None).unwrap();
        assert_eq!(rows[0].tokens, Some(1000));
        assert_eq!(rows[0].main_tokens, Some(300));
        assert_eq!(rows[0].delegations, 2);
        let s = cell_stats(&rows, "v", "m");
        assert_eq!(s.delegated, 1);
        assert_eq!(s.mean_main_tokens, Some(200));
        let md = render(&rows, &Value::Null, "x");
        assert!(
            md.contains("| v | m | 1/2 | 50% | 0/1 | 550 | 200 | 1/2 |"),
            "{md}"
        );
    }

    #[test]
    fn numbers_get_thousands_separators() {
        assert_eq!(fmt_num(Some(1_234_567)), "1,234,567");
        assert_eq!(fmt_num(Some(999)), "999");
        assert_eq!(fmt_secs(Some(125_000)), "2m05s");
    }
}
