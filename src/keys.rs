//! API keys for isolated runs.
//!
//! Each run gets an empty koda config directory, so the koda it runs has
//! no keystore. Keys reach it as environment variables: whatever the
//! harness's own environment already holds, plus the `apikey:*` rows of
//! the user's real koda keystore. An environment variable wins over the
//! keystore. Values are never logged; only the variable names are.
//!
//! This is the one place the harness looks inside koda rather than
//! talking to the binary. The contract is small: koda keeps keys in
//! `<config dir>/db/koda.db`, table `kv_store`, under `apikey:<ENV_NAME>`
//! (koda #693).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};

use crate::util;

/// Where koda keeps its keystore for the user running the harness.
pub fn default_keystore() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| util::home().join(".config"));
    base.join("koda").join("db").join("koda.db")
}

/// `ENV_NAME -> value` for every key stored in `keystore`. Missing file
/// means no keys, not an error.
pub fn read_keystore(keystore: &Path) -> Result<BTreeMap<String, String>> {
    if !keystore.is_file() {
        return Ok(BTreeMap::new());
    }
    let conn = Connection::open_with_flags(keystore, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("open koda keystore {}", keystore.display()))?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM kv_store WHERE key LIKE 'apikey:%'")
        .context("koda keystore has no kv_store table")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut keys = BTreeMap::new();
    for row in rows {
        let (key, value) = row?;
        if let Some(name) = key.strip_prefix("apikey:")
            && !name.is_empty()
            && !value.is_empty()
        {
            keys.insert(name.to_string(), value);
        }
    }
    Ok(keys)
}

/// The keys a run should inherit from the keystore: those whose variable
/// isn't already set in the harness's environment.
pub fn keys_for_child(keystore: &Path) -> Result<BTreeMap<String, String>> {
    let mut keys = read_keystore(keystore)?;
    keys.retain(|name, _| std::env::var_os(name).is_none());
    Ok(keys)
}

/// A stored value that can't be an API key: whitespace inside it, or far
/// too short. The classic case is a shell command pasted into koda's
/// `/key` prompt. The caller reports the name, never the value.
pub fn looks_wrong(value: &str) -> bool {
    value.len() < 16 || value.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keystore_with(rows: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("koda.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL, \
             updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);",
        )
        .unwrap();
        for (k, v) in rows {
            conn.execute("INSERT INTO kv_store (key, value) VALUES (?1, ?2)", [k, v])
                .unwrap();
        }
        (tmp, path)
    }

    #[test]
    fn reads_only_apikey_rows_and_strips_the_prefix() {
        let (_tmp, path) = keystore_with(&[
            ("apikey:ANTHROPIC_API_KEY", "sk-test"),
            ("apikey:EMPTY_KEY", ""),
            ("setting:theme", "dark"),
        ]);
        let keys = read_keystore(&path).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys["ANTHROPIC_API_KEY"], "sk-test");
    }

    #[test]
    fn flags_values_that_cannot_be_keys() {
        assert!(looks_wrong(
            "echo 'export ANTHROPIC_API_KEY=x' >> ~/.zshenv"
        ));
        assert!(looks_wrong("short"));
        assert!(!looks_wrong("sk-ant-api03-0123456789abcdef"));
    }

    #[test]
    fn a_missing_keystore_means_no_keys() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(
            read_keystore(&tmp.path().join("nope.db"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn environment_wins_over_the_keystore() {
        // PATH is always set; a keystore row for it must be ignored.
        let (_tmp, path) =
            keystore_with(&[("apikey:PATH", "x"), ("apikey:KODA_EVAL_TEST_ONLY", "y")]);
        let keys = keys_for_child(&path).unwrap();
        assert!(!keys.contains_key("PATH"));
        assert_eq!(keys["KODA_EVAL_TEST_ONLY"], "y");
    }
}
