//! The agent's `transcript.ndjson`: koda's `stream-json` events, parsed
//! once and queried by the transcript predicates (`check.rs`) and the
//! behaviour signals on every results row.
//!
//! Event kinds the harness reads: `tool_call_start` {id, name, args,
//! parent_id}, `tool_call_result` {id, name, output, outcome}, `text_done`,
//! `approval_request` {tool_name, detail, effect}, `action_blocked`
//! {tool_name, detail}, `ask_user_request`, `compaction` {status,
//! summary_tokens}, `loop_guard`, `turn_end` {reason: {kind}}, and the final
//! `run_summary`. Everything else is only counted, so an unknown type is
//! skipped: koda's protocol allows new notifications without a version change.
//!
//! koda before protocol 1 sent `is_sub_agent`, no `outcome`, and compaction
//! and the loop guard as `info` and `warn` text. Run files compare older
//! binaries, and `bin/claude-as-koda` writes that shape, so those still parse.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The newest koda wire protocol this reader understands.
const PROTOCOL_VERSION: u64 = 1;

/// Tools that hand work to a child agent or session.
pub const DELEGATION_TOOLS: &[&str] = &["InvokeAgent", "SpawnAgent"];

/// Tools that change files.
pub const EDIT_TOOLS: &[&str] = &["Edit", "Write"];

/// A leading `cd <dir> &&`, which says nothing about what a command does.
static LEADING_CD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*cd\s+\S+\s*(?:&&|;)\s*").unwrap());

/// What a run did with Grep, and the Bash calls doing a dedicated tool's job.
/// Sub-agents included. A Bash call is classed by its first command, so a
/// search later in a chain doesn't count.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ToolUse {
    /// Grep calls that got a result.
    pub grep_calls: u64,
    pub grep_empty: u64,
    /// Those whose pattern has regex characters.
    pub grep_regex_calls: u64,
    pub grep_regex_empty: u64,
    pub bash_calls: u64,
    /// Bash calls starting with `grep` or `rg`.
    pub bash_search: u64,
    /// Bash calls starting with `sed` (not `-i`) or `cat` (not a heredoc).
    pub bash_read: u64,
}

impl ToolUse {
    pub fn add(&mut self, other: &ToolUse) {
        self.grep_calls += other.grep_calls;
        self.grep_empty += other.grep_empty;
        self.grep_regex_calls += other.grep_regex_calls;
        self.grep_regex_empty += other.grep_regex_empty;
        self.bash_calls += other.bash_calls;
        self.bash_search += other.bash_search;
        self.bash_read += other.bash_read;
    }
}

fn is_regex(pattern: &str) -> bool {
    pattern.contains(|c| "|\\^$*+?()[]{}".contains(c))
}

fn is_shell_search(command: &str) -> bool {
    let command = LEADING_CD.replace(command, "");
    matches!(command.split_whitespace().next(), Some("grep" | "rg"))
}

fn is_shell_read(command: &str) -> bool {
    let command = LEADING_CD.replace(command, "");
    match command.split_whitespace().next() {
        Some("sed") => !command.contains(" -i"),
        Some("cat") => !command.contains("<<"),
        _ => false,
    }
}

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
    /// The call's `id`, which its `tool_call_result` repeats.
    pub id: &'a str,
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
            id: e["id"].as_str().unwrap_or(""),
            name: e["name"].as_str().unwrap_or(""),
            args: &e["args"],
            is_sub_agent: e["parent_id"].is_string()
                || e["is_sub_agent"].as_bool().unwrap_or(false),
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

    /// `tool_call_result` events for one tool.
    fn results_of<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
        self.of_type("tool_call_result")
            .filter(move |(_, e)| e["name"] == name)
            .map(|(_, e)| e)
    }

    /// `compaction` lines with this `status`.
    fn compactions<'a>(&'a self, status: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
        self.of_type("compaction")
            .filter(move |(_, e)| e["status"] == status)
            .map(|(_, e)| e)
    }

    /// `protocol_version` from `hello`. `None` before protocol 1.
    pub fn protocol_version(&self) -> Option<u64> {
        self.of_type("hello")
            .next()
            .and_then(|(_, e)| e["protocol_version"].as_u64())
    }

    /// The output of the `tool_call_result` for the call with this `id`.
    /// `None` when the call never completed: rejected at an approval,
    /// or cut off by the wall-clock limit.
    pub fn result_output(&self, id: &str) -> Option<&str> {
        if id.is_empty() {
            return None;
        }
        self.of_type("tool_call_result")
            .find(|(_, e)| e["id"] == id)
            .and_then(|(_, e)| e["output"].as_str())
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

    /// The failure to report when koda's `hello` names a protocol this reader
    /// doesn't know: a newer one, or a version it can't read. `None` with no
    /// `hello`, since koda before protocol 1 sent none.
    pub fn unsupported_protocol(&self) -> Option<String> {
        let (_, hello) = self.of_type("hello").next()?;
        match &hello["protocol_version"] {
            version if version.as_u64().is_some_and(|v| v <= PROTOCOL_VERSION) => None,
            Value::Null => Some("unsupported protocol".to_string()),
            version => Some(format!("unsupported protocol {version}")),
        }
    }

    /// Whether the call with this `id` reported an `error` outcome.
    pub fn result_failed(&self, id: &str) -> bool {
        !id.is_empty()
            && self
                .of_type("tool_call_result")
                .find(|(_, e)| e["id"] == id)
                .is_some_and(|(_, e)| e["outcome"] == "error")
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

    /// `Edit` and `Write` calls, sub-agents included. Calls, not successes:
    /// a row with none and an empty diff never tried.
    pub fn edits_attempted(&self) -> usize {
        self.tool_calls()
            .filter(|c| EDIT_TOOLS.contains(&c.name))
            .count()
    }

    /// What the run did with Grep and Bash.
    pub fn tool_use(&self) -> ToolUse {
        let mut used = ToolUse::default();
        for call in self.tool_calls() {
            match call.name {
                "Grep" => {
                    let Some(output) = self.result_output(call.id) else {
                        continue;
                    };
                    let regex = call.args["pattern"].as_str().is_some_and(is_regex);
                    let empty = output.starts_with("No matches found");
                    used.grep_calls += 1;
                    used.grep_empty += u64::from(empty);
                    used.grep_regex_calls += u64::from(regex);
                    used.grep_regex_empty += u64::from(regex && empty);
                }
                "Bash" => {
                    let command = call.args["command"].as_str().unwrap_or("");
                    used.bash_calls += 1;
                    used.bash_search += u64::from(is_shell_search(command));
                    used.bash_read += u64::from(is_shell_read(command));
                }
                _ => {}
            }
        }
        used
    }

    pub fn count(&self, kind: &str) -> u64 {
        self.counts.get(kind).copied().unwrap_or(0)
    }

    /// Behaviour signals for the results row. Every signal here is read
    /// from the event stream; `time_limit_hit` is the harness's own
    /// observation. A signal with nothing to read it from is `null`.
    pub fn behaviour(&self, time_limit_hit: bool) -> Value {
        // A `compaction` line with status `started` per attempt, and
        // `completed` with a `summary_tokens` for one that got a summary.
        // Before protocol 1 the same facts were text: an `info` "📦 Context
        // at N% — compacting before sending..." and a "✅ Compacted N
        // messages ...". A run has one form or the other.
        let attempts = self.compactions("started").count()
            + self
                .messages("info")
                .filter(|m| m.contains("compacting before sending"))
                .count();
        let with_summary = self
            .compactions("completed")
            .filter(|e| e["summary_tokens"].is_number())
            .count()
            + self
                .messages("info")
                .filter(|m| m.contains("Compacted ") && m.contains(" messages"))
                .count();
        // Loop guard: a `loop_guard` line each time it acts. Before protocol
        // 1, "Loop detected: ..." when it injected feedback and "Loop guard:
        // ... stopping" when it gave up.
        let loop_guard = self.count("loop_guard") as usize
            + self
                .messages("warn")
                .filter(|m| m.starts_with("Loop detected") || m.starts_with("Loop guard"))
                .count();
        // An Edit the tool refused: validation ("Validation error: File
        // '...' has been modified on disk since you last read it",
        // "'old_str' not found") or execution ("Error: ... has changed on
        // disk since you last read it", "'old_str' matches N times"). The
        // `outcome` says so; before protocol 1 only the text did.
        let edit_rejections = self
            .results_of("Edit")
            .filter(|e| match e["outcome"].as_str() {
                Some(outcome) => outcome == "error",
                None => e["output"]
                    .as_str()
                    .is_some_and(|o| o.starts_with("Validation error:") || o.starts_with("Error:")),
            })
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
            "edits_attempted": self.edits_attempted(),
            "time_limit_hit": time_limit_hit,
            "turn_end_kind": self.turn_end_kind(),
            "protocol_version": self.protocol_version(),
            "delegations": delegations,
            "approval_requests": { "total": approvals, "destructive": destructive },
            "blocked_actions": self.count("action_blocked"),
            "tool_use": self.tool_use(),
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
        assert_eq!(t.tool_calls().next().unwrap().id, "a");
        assert_eq!(t.result_output("a"), Some("fn f() {}"));
        // The destructive Bash was rejected: no result.
        assert_eq!(t.result_output("c"), None);
        assert_eq!(t.result_output(""), None);
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
        assert_eq!(b["edits_attempted"], 1);
        assert_eq!(b["time_limit_hit"], false);
        assert_eq!(b["turn_end_kind"], "complete");
        assert_eq!(b["delegations"], 1);
        assert_eq!(b["approval_requests"]["total"], 1);
        assert_eq!(b["approval_requests"]["destructive"], 1);
        assert_eq!(b["blocked_actions"], 0);
        assert_eq!(b["protocol_version"], Value::Null);
    }

    const SAMPLE_V1: &str = r#"{"type":"hello","protocol_version":1,"koda_version":"0.6.0","capabilities":["one_shot","self_answer"]}
{"type":"session_ready","session_id":"s","provider":"anthropic","model":"m","context_window":1000,"trust":"auto","tools":["Read"]}
{"type":"turn_start","session_id":"s","turn_id":"t"}
{"type":"tool_call_start","session_id":"s","turn_id":"t","id":"a","name":"Bash","args":{"command":"cargo test"}}
{"type":"tool_call_result","session_id":"s","turn_id":"t","id":"a","name":"Bash","output":"Exit code: 101 | stdout: 1 lines","outcome":"error"}
{"type":"compaction","session_id":"s","turn_id":"t","trigger":"threshold","status":"started","context_percent":94}
{"type":"compaction","session_id":"s","turn_id":"t","trigger":"threshold","status":"skipped","reason":"too_short"}
{"type":"compaction","session_id":"s","turn_id":"t","trigger":"threshold","status":"started","context_percent":96}
{"type":"compaction","session_id":"s","turn_id":"t","trigger":"threshold","status":"completed","messages_removed":3,"summary_tokens":18}
{"type":"compaction","session_id":"s","turn_id":"t","trigger":"window","status":"completed","messages_removed":3,"summary_tokens":null}
{"type":"retry","session_id":"s","turn_id":"t","reason":"rate_limit","attempt":1,"max_attempts":5,"delay_ms":1000}
{"type":"loop_guard","session_id":"s","turn_id":"t","action":"feedback","detail":"Bash repeated"}
{"type":"tool_call_start","session_id":"s","turn_id":"t","id":"b","name":"InvokeAgent","args":{}}
{"type":"sub_agent_start","session_id":"s","turn_id":"t","parent_id":"b","agent_name":"explore"}
{"type":"tool_call_start","session_id":"s","turn_id":"t","parent_id":"b","id":"c","name":"Read","args":{}}
{"type":"tool_call_start","session_id":"s","turn_id":"t","id":"d","name":"Edit","args":{"file_path":"a.rs"}}
{"type":"tool_call_result","session_id":"s","turn_id":"t","id":"d","name":"Edit","output":"Validation error: 'old_str' not found","outcome":"error"}
{"type":"tool_call_start","session_id":"s","turn_id":"t","id":"e","name":"Edit","args":{"file_path":"a.rs"}}
{"type":"tool_call_result","session_id":"s","turn_id":"t","id":"e","name":"Edit","output":"Error: is not the outcome","outcome":"success"}
{"type":"text_done","session_id":"s","turn_id":"t"}
{"type":"turn_summary","session_id":"s","turn_id":"t","usage":{"prompt_tokens":1,"completion_tokens":1,"cache_read_tokens":0,"cache_creation_tokens":0,"thinking_tokens":0,"api_calls":1},"elapsed_ms":5}
{"type":"turn_end","session_id":"s","turn_id":"t","reason":{"kind":"complete"}}
{"type":"run_summary","session_id":"s","success":true,"response":"Done.","turn_end":{"kind":"complete"}}
"#;

    #[test]
    fn behaviour_reads_the_typed_protocol() {
        let t = Transcript::parse(SAMPLE_V1);
        assert_eq!(t.protocol_version(), Some(1));
        assert_eq!(t.unsupported_protocol(), None);
        let b = t.behaviour(false);
        assert_eq!(b["protocol_version"], 1);
        assert_eq!(b["compactions"]["attempts"], 2);
        assert_eq!(b["compactions"]["with_summary"], 1);
        assert_eq!(b["loop_guard_warnings"], 1);
        assert_eq!(b["edit_rejections"], 1);
        assert_eq!(b["delegations"], 1);
        assert_eq!(b["turn_end_kind"], "complete");
        assert_eq!(t.top_level_calls().count(), 4);
        assert_eq!(t.tool_calls().count(), 5);
        assert!(t.result_failed("a"));
        assert!(!t.result_failed("e"));
        assert!(!t.result_failed("no such call"));
        assert_eq!(t.final_message(), Some("Done."));
    }

    /// What koda really wrote; `bin/capture-koda-wire` records it.
    const CAPTURED: &str = include_str!("../tests/fixtures/koda-wire-v1.ndjson");

    #[test]
    fn reads_what_koda_wrote() {
        let t = Transcript::parse(CAPTURED);
        assert_eq!(t.count("unparsed_line"), 0);
        assert_eq!(t.protocol_version(), Some(1));
        assert_eq!(t.unsupported_protocol(), None);
        assert_eq!(t.turn_end_kind(), Some("complete"));
        assert_eq!(t.final_message(), Some("Done."));
        assert!(t.final_message_index().is_some());

        // Sub-agent calls are told apart from the agent's own.
        let of = |id: &str| t.tool_calls().find(|c| c.id == id).unwrap();
        assert!(!of("agent").is_sub_agent);
        assert!(of("child-1").is_sub_agent);
        assert_eq!(of("read-ok").args["file_path"], "notes_0.txt");

        // A result is matched to its call, and says how it went.
        assert!(t.result_output("read-ok").unwrap().starts_with("note 0:"));
        assert!(!t.result_failed("read-ok"));
        assert!(t.result_failed("edit-bad"));
        assert!(t.result_failed("bash-fails"));
        assert_eq!(t.result_output("rm"), None, "rejected, so no result");

        assert_eq!(t.count("ask_user_request"), 1);
        assert_eq!(
            t.approval_requests().collect::<Vec<_>>(),
            [("Bash", "Destructive", "rm -rf target")]
        );
        assert_eq!(
            t.blocked_actions().collect::<Vec<_>>(),
            [("Bash", "rm -rf notes")]
        );

        let b = t.behaviour(false);
        assert_eq!(b["protocol_version"], 1);
        assert_eq!(b["turn_end_kind"], "complete");
        assert_eq!(b["compactions"]["attempts"], 4);
        assert_eq!(b["compactions"]["with_summary"], 1);
        assert_eq!(b["loop_guard_warnings"], 1);
        assert_eq!(b["edit_rejections"], 1);
        assert_eq!(b["edits_attempted"], 1);
        assert_eq!(b["delegations"], 1);
        assert_eq!(b["approval_requests"]["destructive"], 1);
        assert_eq!(b["blocked_actions"], 1);
        assert_eq!(b["tool_use"]["bash_calls"], 8);

        let summary = t.summary.as_ref().unwrap();
        assert_eq!(summary["usage_by_agent"]["explore"]["api_calls"], 3);
    }

    #[test]
    fn a_protocol_this_reader_cannot_read_is_named() {
        let of = |hello: &str| Transcript::parse(hello).unsupported_protocol();
        assert_eq!(
            of(r#"{"type":"hello","protocol_version":2}"#).as_deref(),
            Some("unsupported protocol 2")
        );
        assert_eq!(
            of(r#"{"type":"hello","protocol_version":"1"}"#).as_deref(),
            Some(r#"unsupported protocol "1""#)
        );
        assert_eq!(
            of(r#"{"type":"hello"}"#).as_deref(),
            Some("unsupported protocol")
        );
        assert_eq!(of(""), None, "koda before protocol 1 sends no hello");
    }

    #[test]
    fn an_unknown_line_type_is_counted_and_skipped() {
        let t = Transcript::parse(
            r#"{"type":"something_new","session_id":"s","value":1}
{"type":"run_summary","response":"Done.","turn_end":{"kind":"complete"}}
"#,
        );
        assert_eq!(t.count("something_new"), 1);
        assert_eq!(t.count("unparsed_line"), 0);
        assert_eq!(t.final_message(), Some("Done."));
        assert_eq!(t.protocol_version(), None);
    }

    #[test]
    fn tool_use_tells_regex_searches_and_shell_habits_apart() {
        let t = Transcript::parse(
            r#"{"type":"tool_call_start","id":"a","name":"Grep","args":{"pattern":"foo|bar"},"is_sub_agent":false}
{"type":"tool_call_result","id":"a","name":"Grep","output":"No matches found for 'foo|bar'"}
{"type":"tool_call_start","id":"b","name":"Grep","args":{"pattern":"plain"},"is_sub_agent":true}
{"type":"tool_call_result","id":"b","name":"Grep","output":"1 matches (1 file):\na.rs:1:plain"}
{"type":"tool_call_start","id":"c","name":"Grep","args":{"pattern":"absent"},"is_sub_agent":false}
{"type":"tool_call_result","id":"c","name":"Grep","output":"No matches found for 'absent'"}
{"type":"tool_call_start","id":"d","name":"Grep","args":{"pattern":"cut off"},"is_sub_agent":false}
{"type":"tool_call_start","id":"e","name":"Bash","args":{"command":"cd /work && grep -n x a.rs | head"},"is_sub_agent":false}
{"type":"tool_call_start","id":"f","name":"Bash","args":{"command":"rg x"},"is_sub_agent":false}
{"type":"tool_call_start","id":"g","name":"Bash","args":{"command":"sed -n 1,20p a.rs"},"is_sub_agent":false}
{"type":"tool_call_start","id":"h","name":"Bash","args":{"command":"sed -i s/a/b/ a.rs"},"is_sub_agent":false}
{"type":"tool_call_start","id":"i","name":"Bash","args":{"command":"cat a.rs"},"is_sub_agent":false}
{"type":"tool_call_start","id":"j","name":"Bash","args":{"command":"cat > a.rs <<'EOF'\nx\nEOF"},"is_sub_agent":false}
{"type":"tool_call_start","id":"k","name":"Bash","args":{"command":"cargo test; grep x a.rs"},"is_sub_agent":false}
"#,
        );
        assert_eq!(
            t.tool_use(),
            ToolUse {
                grep_calls: 3,
                grep_empty: 2,
                grep_regex_calls: 1,
                grep_regex_empty: 1,
                bash_calls: 7,
                bash_search: 2,
                bash_read: 2,
            }
        );
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
