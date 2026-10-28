# Changelog

## 2.0.0

Breaking:

- `parse_config(text)` is replaced by `Config::parse(text, &ParseOptions)`.
- Parsing is strict by default: a line that is not blank, not a `#`
  comment and has no `=` is now an error. 1.x skipped such lines; pass
  `ParseOptions { strict: false, .. }` for the old behavior.
- Keys are no longer lower-cased by default. 1.x lower-cased every key;
  pass `ParseOptions { lowercase_keys: true, .. }` for the old behavior.
- `Config::get` returns `Option<&str>` instead of `Option<String>`.
- `Config::get_or` is removed; use `get(key).unwrap_or(default)`.

## 1.4.0

- `get_or(key, default)`.
