//! Koda's own log (`koda.log`), read for the signals that explain a failed
//! row: whether the run stopped to ask, how large the context got, and what
//! compaction cost.
//!
//! Lines are `tracing` text: `<time> <LEVEL> <target>: <message> k=v k="v" …`,
//! possibly wrapped in ANSI colour codes. The messages read here are
//! `model call`, `compaction` and `turn end`. Tool output that koda echoes
//! into the log can contain those words, so a line only counts when it
//! starts like a log line. A log written before koda logged these lines
//! yields `None` for every signal, not zero.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

static ANSI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap());
static FIELD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(\w+)=("(?:[^"\\]|\\.)*"|\S+)"#).unwrap());

/// What the log says about one run. `None`: the log has nothing to say.
#[derive(Debug, Default, PartialEq)]
pub struct LogSignals {
    /// `ends_with_question` on the last `turn end` line.
    pub ended_with_question: Option<bool>,
    /// The largest context any `turn` model call carried: the provider's
    /// `context_tokens`, or koda's `estimated_tokens` when that is 0 or
    /// missing.
    pub peak_context_tokens: Option<u64>,
    /// `prompt_tokens + completion_tokens` over `compaction` model calls.
    pub compaction_tokens: Option<u64>,
    /// `elapsed_ms` over `compaction` model calls.
    pub compaction_ms: Option<u64>,
    /// The largest `summary_chars` on a `compaction` line.
    pub summary_chars_max: Option<u64>,
}

impl LogSignals {
    /// The fields these signals add to a row's `behaviour`, with the
    /// figures that need the row's context `window` and `wall_ms`. Percent
    /// has one decimal; share is a fraction of the wall time with three.
    pub fn to_json(&self, window: Option<u64>, wall_ms: u64) -> Value {
        let percent = self
            .peak_context_tokens
            .zip(window.filter(|&w| w > 0))
            .map(|(peak, window)| round(100.0 * peak as f64 / window as f64, 1));
        let share = self
            .compaction_ms
            .filter(|_| wall_ms > 0)
            .map(|ms| round(ms as f64 / wall_ms as f64, 3));
        json!({
            "ended_with_question": self.ended_with_question,
            "peak_context_tokens": self.peak_context_tokens,
            "peak_context_percent": percent,
            "compaction_tokens": self.compaction_tokens,
            "compaction_ms": self.compaction_ms,
            "compaction_share": share,
            "summary_chars_max": self.summary_chars_max,
        })
    }
}

fn round(x: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    (x * k).round() / k
}

/// Read `koda.log`; `None` when there is no log.
pub fn read(path: &Path) -> Option<LogSignals> {
    let bytes = std::fs::read(path).ok()?;
    Some(parse(&String::from_utf8_lossy(&bytes)))
}

pub fn parse(text: &str) -> LogSignals {
    let mut s = LogSignals::default();
    let mut saw_model_call = false;
    for line in text.lines() {
        let line = ANSI.replace_all(line, "");
        let Some(message) = message(&line) else {
            continue;
        };
        if let Some(rest) = message.strip_prefix("model call ") {
            saw_model_call = true;
            let f = Fields::new(rest);
            match f.get("purpose") {
                Some("turn") => {
                    let context = f
                        .num("context_tokens")
                        .filter(|&n| n > 0)
                        .or_else(|| f.num("estimated_tokens"));
                    s.peak_context_tokens = s.peak_context_tokens.max(context);
                }
                Some("compaction") => {
                    let tokens = f.num("prompt_tokens").unwrap_or(0)
                        + f.num("completion_tokens").unwrap_or(0);
                    *s.compaction_tokens.get_or_insert(0) += tokens;
                    *s.compaction_ms.get_or_insert(0) += f.num("elapsed_ms").unwrap_or(0);
                }
                _ => {}
            }
        } else if let Some(rest) = message.strip_prefix("compaction ") {
            s.summary_chars_max = s
                .summary_chars_max
                .max(Fields::new(rest).num("summary_chars"));
        } else if let Some(rest) = message.strip_prefix("turn end ")
            && let Some(asked) = Fields::new(rest).get("ends_with_question")
        {
            s.ended_with_question = Some(asked == "true");
        }
    }
    // A log that records model calls but none for compaction means
    // compaction cost nothing.
    if saw_model_call {
        s.compaction_tokens.get_or_insert(0);
        s.compaction_ms.get_or_insert(0);
    }
    s
}

/// The message of a `tracing` line (everything after `<target>:`), or
/// `None` for a line that doesn't start like one.
fn message(line: &str) -> Option<&str> {
    let (time, rest) = line.trim_start().split_once(' ')?;
    if !time.starts_with(|c: char| c.is_ascii_digit()) || !time.contains('T') {
        return None;
    }
    let (level, rest) = rest.trim_start().split_once(' ')?;
    if !matches!(level, "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR") {
        return None;
    }
    let (target, message) = rest.trim_start().split_once(' ')?;
    target.ends_with(':').then_some(message.trim_start())
}

/// The `key=value` pairs of a message; quoted values lose their quotes.
struct Fields<'a>(Vec<(&'a str, &'a str)>);

impl<'a> Fields<'a> {
    fn new(text: &'a str) -> Self {
        Fields(
            FIELD
                .captures_iter(text)
                .map(|c| {
                    let key = c.get(1).map_or("", |m| m.as_str());
                    let value = c.get(2).map_or("", |m| m.as_str());
                    (key, value.trim_matches('"'))
                })
                .collect(),
        )
    }

    fn get(&self, key: &str) -> Option<&'a str> {
        self.0.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }

    fn num(&self, key: &str) -> Option<u64> {
        self.get(key)?.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = r#"2026-11-01T11:17:55.567Z  INFO koda_core::inference: context assembled iteration=0 estimated_tokens=17977 window=64000 summary_chars=0
2026-11-01T11:17:57.939Z  INFO koda_core::inference_helpers: model call purpose="turn" provider="anthropic" estimated_tokens=17977 context_tokens=23819 elapsed_ms=2372 prompt_tokens=288 completion_tokens=384 stop_reason=tool_use
2026-11-01T11:18:57.939Z  INFO koda_core::inference_helpers: model call purpose="turn" estimated_tokens=30000 context_tokens=0 elapsed_ms=2372 prompt_tokens=1 completion_tokens=1
2026-11-01T11:18:58.939Z  INFO koda_core::inference_helpers: model call purpose="compaction" estimated_tokens=2684 elapsed_ms=11881 prompt_tokens=3989 completion_tokens=2865 stop_reason=end_turn
2026-11-01T11:18:58.949Z  INFO koda_core::compact: compaction branch="summary" rows_summarized=7 clipped_chars=0 attempts=1 stop_reason=end_turn summary_chars=5066
2026-11-01T11:19:58.939Z  INFO koda_core::inference_helpers: model call purpose="compaction" elapsed_ms=1000 prompt_tokens=10 completion_tokens=20
2026-11-01T11:19:58.949Z  INFO koda_core::compact: compaction branch="summary" attempts=1 summary_chars=3000
model call purpose="turn" context_tokens=999999 (an echoed file, not a log line)
2026-11-01T11:20:53.057Z  INFO koda_core::inference: turn end session_id="83eacea3" outcome="complete" model_calls=20 tool_calls=36 final_text_chars=1891 ends_with_question=true
"#;

    #[test]
    fn reads_the_signals_from_a_log() {
        let s = parse(LOG);
        assert_eq!(s.ended_with_question, Some(true));
        // The second turn reported no context_tokens: its estimate counts.
        assert_eq!(s.peak_context_tokens, Some(30_000));
        assert_eq!(s.compaction_tokens, Some(3989 + 2865 + 30));
        assert_eq!(s.compaction_ms, Some(12_881));
        assert_eq!(s.summary_chars_max, Some(5066));
    }

    #[test]
    fn ansi_colour_codes_are_stripped() {
        let line = "\x1b[2m2026-11-01T11:20:53.057Z\x1b[0m \x1b[32m INFO\x1b[0m \x1b[2mkoda_core::inference\x1b[0m\x1b[2m:\x1b[0m turn end \x1b[3mends_with_question\x1b[0m\x1b[2m=\x1b[0mfalse\n";
        assert_eq!(parse(line).ended_with_question, Some(false));
    }

    #[test]
    fn the_last_turn_end_wins() {
        let log = "2026-11-01T11:20:53Z INFO koda_core::inference: turn end ends_with_question=true\n\
                   2026-11-01T11:21:53Z INFO koda_core::inference: turn end ends_with_question=false\n";
        assert_eq!(parse(log).ended_with_question, Some(false));
    }

    #[test]
    fn a_log_with_model_calls_but_no_compaction_costs_nothing() {
        let s = parse(
            "2026-11-01T11:17:57Z INFO koda_core::inference_helpers: model call purpose=\"turn\" context_tokens=5\n",
        );
        assert_eq!(s.peak_context_tokens, Some(5));
        assert_eq!((s.compaction_tokens, s.compaction_ms), (Some(0), Some(0)));
        assert_eq!(s.summary_chars_max, None);
        assert_eq!(s.ended_with_question, None);
    }

    #[test]
    fn a_log_from_before_the_new_lines_has_no_signals() {
        let log = "2026-10-29T10:53:00.908Z  INFO koda_cli::app: Koda starting. Project root: \"/x\"\n\
                   \x1b[2m2026-10-29T10:53:00.912Z\x1b[0m \x1b[32m INFO\x1b[0m koda_core::db: Created session: f7\n";
        assert_eq!(parse(log), LogSignals::default());
        assert_eq!(parse(""), LogSignals::default());
    }

    #[test]
    fn row_fields_add_percent_and_share() {
        let s = parse(LOG);
        let v = s.to_json(Some(64_000), 100_000);
        assert_eq!(v["ended_with_question"], true);
        assert_eq!(v["peak_context_tokens"], 30_000);
        assert_eq!(v["peak_context_percent"], 46.9);
        assert_eq!(v["compaction_ms"], 12_881);
        assert_eq!(v["compaction_share"], 0.129);
        assert_eq!(v["summary_chars_max"], 5066);
        // No pinned window: no percent. No wall time: no share.
        let v = s.to_json(None, 0);
        assert_eq!(v["peak_context_tokens"], 30_000);
        assert!(v["peak_context_percent"].is_null());
        assert!(v["compaction_share"].is_null());
        // Nothing in the log: every field null.
        let v = LogSignals::default().to_json(Some(64_000), 100_000);
        assert!(v.as_object().unwrap().values().all(Value::is_null), "{v}");
    }

    #[test]
    fn a_missing_log_is_none() {
        assert_eq!(read(Path::new("/nonexistent/koda.log")), None);
    }
}
