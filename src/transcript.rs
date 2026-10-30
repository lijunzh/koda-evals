//! The agent's `transcript.ndjson`: koda's `stream-json` events, parsed
//! once and queried by the transcript predicates (`check.rs`) and the
//! behaviour signals on every results row.
//!
//! Event kinds the harness reads: `tool_call_start` {id, name, args,
//! is_sub_agent}, `tool_call_result` {id, name, output}, `text_done`,
//! `approval_request` {tool_name, detail, effect}, `action_blocked`
//! {tool_name, detail}, `ask_user_request`, `info` and `warn` {message},
//! `turn_end` {reason: {kind}}, and the final `run_summary`. Everything
//! else is only counted.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use serde_json::{Value, json};

/// Tools that hand work to a child agent or session.
pub const DELEGATION_TOOLS: &[&str] = &["InvokeAgent", "SpawnAgent"];

/// Tools that change files.
pub const EDIT_TOOLS: &[&str] = &["Edit", "Write"];

#[derive(Debug, Default)]
pub struct Transcript {
    /// Every parsed event in order, `run_summary` excluded.
    pub events: Vec<Value>,
    /// The `run_summary` line, if koda got that far.
    pub summary: Option<Value>,
    /// Count of every event type, plus `unparsed_line`.
    pub counts: BTreeMap<String, u64>,
}

/// One `tool_call_start`.
#[derive(Debug, Clone, Copy)]
pub struct ToolCall<'a> {
    /// Position in `events`, for ordering.
    pub index: usize,
    pub name: &'a str,
    pub args: &'a Value,
    pub is_sub_agent: bool,
}

impl Transcript {
    pub fn read(path: &Path) -> Result<Self> {
        let text = String::from_utf8_lossy(&std::fs::read(path)?).into_owned();
        Ok(Self::parse(&text))
    }

    pub fn parse(text: &str) -> Self {
        let mut t = Transcript::default();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                *t.counts.entry("unparsed_line".into()).or_default() += 1;
                continue;
            };
            let kind = value["type"].as_str().unwrap_or("untyped").to_string();
            if kind == "run_summary" {
                t.summary = Some(value);
            } else {
                *t.counts.entry(kind).or_default() += 1;
                t.events.push(value);
            }
        }
        t
    }

    fn of_type<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = (usize, &'a Value)> + 'a {
        self.events
            .iter()
            .enumerate()
            .filter(move |(_, e)| e["type"] == kind)
    }

    /// Every `tool_call_start`, top level and sub-agent, in order.
    pub fn tool_calls(&self) -> impl Iterator<Item = ToolCall<'_>> + '_ {
        self.of_type("tool_call_start").map(|(index, e)| ToolCall {
            index,
            name: e["name"].as_str().unwrap_or(""),
            args: &e["args"],
            is_sub_agent: e["is_sub_agent"].as_bool().unwrap_or(false),
        })
    }

    /// Top-level tool calls only: what the agent itself chose to do.
    pub fn top_level_calls(&self) -> impl Iterator<Item = ToolCall<'_>> + '_ {
        self.tool_calls().filter(|c| !c.is_sub_agent)
    }

    /// `approval_request` events: `(tool_name, effect, detail)`.
    pub fn approval_requests(&self) -> impl Iterator<Item = (&str, &str, &str)> + '_ {
        self.of_type("approval_request").map(|(_, e)| {
            (
                e["tool_name"].as_str().unwrap_or(""),
                e["effect"].as_str().unwrap_or(""),
                e["detail"].as_str().unwrap_or(""),
            )
        })
    }

    /// `action_blocked` events: `(tool_name, detail)`.
    pub fn blocked_actions(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.of_type("action_blocked").map(|(_, e)| {
            (
                e["tool_name"].as_str().unwrap_or(""),
                e["detail"].as_str().unwrap_or(""),
            )
        })
    }

    /// Messages of `info` or `warn` events.
    pub fn messages<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.of_type(kind)
            .filter_map(|(_, e)| e["message"].as_str())
    }

    /// Outputs of `tool_call_result` events for one tool.
    pub fn results_of<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.of_type("tool_call_result")
            .filter(move |(_, e)| e["name"] == name)
            .filter_map(|(_, e)| e["output"].as_str())
    }

    /// The agent's last reply (`run_summary.response`), if there is one.
    pub fn final_message(&self) -> Option<&str> {
        self.summary
            .as_ref()
            .and_then(|s| s["response"].as_str())
            .filter(|r| !r.trim().is_empty())
    }

    /// Position of the last `text_done`: where the final message ended in
    /// the event order. `None` when the agent never finished a reply.
    pub fn final_message_index(&self) -> Option<usize> {
        self.of_type("text_done").last().map(|(i, _)| i)
    }

    /// `reason.kind` of the last `turn_end` event, else the run summary's.
    pub fn turn_end_kind(&self) -> Option<&str> {
        self.of_type("turn_end")
            .last()
            .and_then(|(_, e)| e["reason"]["kind"].as_str())
            .or_else(|| {
                self.summary
                    .as_ref()
                    .and_then(|s| s["turn_end"]["kind"].as_str())
            })
    }

    pub fn count(&self, kind: &str) -> u64 {
        self.counts.get(kind).copied().unwrap_or(0)
    }

    /// Behaviour signals for the results row. Every signal here is read
    /// from the event stream; `time_limit_hit` is the harness's own
    /// observation. A signal with nothing to read it from is `null`.
    pub fn behaviour(&self, time_limit_hit: bool) -> Value {
        // Koda announces every pre-flight auto-compaction with an `info`
        // "📦 Context at N% — compacting before sending...", and a
        // summary that came back with "✅ Compacted N messages ...". A
        // compaction without a summary gets a `warn` instead.
        let attempts = self
            .messages("info")
            .filter(|m| m.contains("compacting before sending"))
            .count();
        let with_summary = self
            .messages("info")
            .filter(|m| m.contains("Compacted ") && m.contains(" messages"))
            .count();
        // Loop guard: "Loop detected: ..." when it injects feedback,
        // "Loop guard: ... stopping" when it gives up.
        let loop_guard = self
            .messages("warn")
            .filter(|m| m.starts_with("Loop detected") || m.starts_with("Loop guard"))
            .count();
        // An Edit the tool refused: validation ("Validation error: File
        // '...' has been modified on disk since you last read it",
        // "'old_str' not found") or execution ("Error: ... has changed on
        // disk since you last read it", "'old_str' matches N times").
        let edit_rejections = self
            .results_of("Edit")
            .filter(|o| o.starts_with("Validation error:") || o.starts_with("Error:"))
            .count();
        let delegations = self
            .top_level_calls()
            .filter(|c| DELEGATION_TOOLS.contains(&c.name))
            .count();
        let approvals = self.approval_requests().count();
        let destructive = self
            .approval_requests()
            .filter(|(_, effect, _)| *effect == "Destructive")
            .count();
        json!({
            "compactions": { "attempts": attempts, "with_summary": with_summary },
            "loop_guard_warnings": loop_guard,
            "edit_rejections": edit_rejections,
            "time_limit_hit": time_limit_hit,
            "turn_end_kind": self.turn_end_kind(),
            "delegations": delegations,
            "approval_requests": { "total": approvals, "destructive": destructive },
            "blocked_actions": self.count("action_blocked"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"type":"turn_start","turn_id":"1"}
{"type":"tool_call_start","id":"a","name":"Read","args":{"file_path":"src/lib.rs"},"is_sub_agent":false}
{"type":"tool_call_result","id":"a","name":"Read","output":"fn f() {}"}
{"type":"info","message":"📦 Context at 85% — compacting before sending..."}
{"type":"info","message":"✅ Compacted 12 messages (~300 token summary)"}
{"type":"info","message":"📦 Context at 86% — compacting before sending..."}
{"type":"warn","message":"Compact failed: LLM returned an empty summary. Continuing anyway..."}
{"type":"warn","message":"Loop detected: 'Bash' called 5 times consecutively with identical arguments. Injecting feedback to nudge the model."}
{"type":"tool_call_start","id":"b","name":"Edit","args":{"file_path":"src/lib.rs"},"is_sub_agent":false}
{"type":"tool_call_result","id":"b","name":"Edit","output":"Validation error: File 'src/lib.rs' has been modified on disk since you last read it."}
{"type":"tool_call_start","id":"c","name":"Bash","args":{"command":"rm -rf target"},"is_sub_agent":false}
{"type":"approval_request","id":"x","tool_name":"Bash","detail":"rm -rf target","preview":null,"effect":"Destructive"}
{"type":"tool_call_start","id":"d","name":"InvokeAgent","args":{"agent":"explore"},"is_sub_agent":false}
{"type":"tool_call_start","id":"e","name":"Grep","args":{},"is_sub_agent":true}
{"type":"text_delta","text":"Done."}
{"type":"text_done"}
not json
{"type":"turn_end","turn_id":"1","reason":{"kind":"complete"}}
{"type":"run_summary","success":true,"response":"Done.","turn_end":{"kind":"complete"},"tool_calls":{"Read":1}}
"#;

    #[test]
    fn parses_events_and_keeps_the_summary_apart() {
        let t = Transcript::parse(SAMPLE);
        assert_eq!(t.summary.as_ref().unwrap()["tool_calls"]["Read"], 1);
        assert_eq!(t.count("tool_call_start"), 5);
        assert_eq!(t.count("unparsed_line"), 1);
        assert!(!t.counts.contains_key("run_summary"));
        assert_eq!(t.top_level_calls().count(), 4);
        assert_eq!(t.final_message(), Some("Done."));
        assert_eq!(t.turn_end_kind(), Some("complete"));
        assert!(t.final_message_index().unwrap() > t.tool_calls().last().unwrap().index);
    }

    #[test]
    fn behaviour_signals_come_from_the_stream() {
        let t = Transcript::parse(SAMPLE);
        let b = t.behaviour(false);
        assert_eq!(b["compactions"]["attempts"], 2);
        assert_eq!(b["compactions"]["with_summary"], 1);
        assert_eq!(b["loop_guard_warnings"], 1);
        assert_eq!(b["edit_rejections"], 1);
        assert_eq!(b["time_limit_hit"], false);
        assert_eq!(b["turn_end_kind"], "complete");
        assert_eq!(b["delegations"], 1);
        assert_eq!(b["approval_requests"]["total"], 1);
        assert_eq!(b["approval_requests"]["destructive"], 1);
        assert_eq!(b["blocked_actions"], 0);
    }

    #[test]
    fn an_empty_transcript_has_no_turn_end() {
        let t = Transcript::parse("");
        let b = t.behaviour(true);
        assert_eq!(b["turn_end_kind"], Value::Null);
        assert_eq!(b["time_limit_hit"], true);
        assert_eq!(t.final_message(), None);
        assert_eq!(t.final_message_index(), None);
    }
}
