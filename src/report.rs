//! `koda-eval report`: the comparison a run was made for.
//!
//! For each variant × model: passes as *k of N*, the pass rate per
//! attempt, pass^r (tasks that passed on every repeat), mean non-cached
//! tokens, the share of those spent in the main context, how many runs
//! delegated, and mean wall time. Then the tasks where variants disagree,
//! and a per-task grid. Raw counts sit next to every percentage: with
//! about ten tasks and three repeats, a few points is noise.

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
}

/// Tools that hand work to a child agent or session.
const DELEGATION_TOOLS: &[&str] = &["InvokeAgent", "SpawnAgent"];

fn non_cached_tokens(usage: &Value) -> Option<u64> {
    match (
        usage["prompt_tokens"].as_u64(),
        usage["completion_tokens"].as_u64(),
    ) {
        (Some(p), Some(c)) => Some(p + c),
        _ => None,
    }
}

fn parse_rows(text: &str) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value =
            serde_json::from_str(line).with_context(|| format!("results.jsonl line {}", i + 1))?;
        let summary = &v["summary"];
        let tokens = non_cached_tokens(&summary["usage"]);
        // Builds older than per-agent usage report none; with no children
        // the whole run is the main context.
        let main_tokens = non_cached_tokens(&summary["usage_by_agent"]["main"]).or(tokens);
        let delegations = DELEGATION_TOOLS
            .iter()
            .filter_map(|t| summary["tool_calls"][*t].as_u64())
            .sum();
        rows.push(Row {
            task: v["task"].as_str().unwrap_or("?").to_string(),
            variant: v["variant"].as_str().unwrap_or("?").to_string(),
            model: v["model"].as_str().unwrap_or("?").to_string(),
            pass: v["pass"].as_bool(),
            tokens,
            main_tokens,
            delegations,
            wall_ms: v["agent"]["wall_ms"].as_u64(),
        });
    }
    Ok(rows)
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
    let _ = writeln!(
        out,
        "# {run} · {started} · {} task(s) · {repeats} repeat(s)\n",
        tasks.len()
    );

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

    // The full grid.
    let _ = writeln!(out, "\n## Per task\n");
    let header: Vec<String> = cells.iter().map(|(v, m)| format!("{v} / {m}")).collect();
    let _ = writeln!(out, "| task | {} |", header.join(" | "));
    let _ = writeln!(out, "|---|{}", "---|".repeat(cells.len()));
    for task in &tasks {
        let row: Vec<String> = cells
            .iter()
            .map(|(v, m)| task_cell(rows, task, v, m))
            .collect();
        let _ = writeln!(out, "| {task} | {} |", row.join(" | "));
    }
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
    let rows = parse_rows(&text)?;
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
        assert!(md.contains("| a | 1/1 | 1/1 |"), "{md}");
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
        let rows = parse_rows(text).unwrap();
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
        let rows = parse_rows(text).unwrap();
        assert_eq!(rows[0].tokens, Some(1000));
        assert_eq!(rows[0].main_tokens, Some(300));
        assert_eq!(rows[0].delegations, 2);
        let s = cell_stats(&rows, "v", "m");
        assert_eq!(s.delegated, 1);
        assert_eq!(s.mean_main_tokens, Some(200));
        let md = render(&rows, &Value::Null, "x");
        assert!(md.contains("| v | m | 1/2 | 50% | 0/1 | 550 | 200 | 1/2 |"), "{md}");
    }

    #[test]
    fn numbers_get_thousands_separators() {
        assert_eq!(fmt_num(Some(1_234_567)), "1,234,567");
        assert_eq!(fmt_num(Some(999)), "999");
        assert_eq!(fmt_secs(Some(125_000)), "2m05s");
    }
}
