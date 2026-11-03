# CLAUDE.md

A Rust project built with Cargo: one crate, or a workspace whose root
`Cargo.toml` lists its members.

## Commands

```bash
cargo build                                # build
cargo test                                 # every test, in every member
cargo test -p <crate>                      # one workspace member
cargo test <name>                          # tests whose path contains <name>
cargo test -p <crate> --test <file>        # one file in the crate's tests/
cargo fmt --all                            # format
cargo clippy --all-targets -- -D warnings  # lint
```

Some tests need a Cargo feature. The `[features]` table in a crate's
`Cargo.toml` lists them; add `--features <feature>` with `-p <crate>`.

## Layout

- Unit tests sit beside the code, in `#[cfg(test)] mod tests`.
- Integration tests are in each crate's `tests/` directory.

## Conventions

Follow the code around a change: its error handling, naming and comments.
