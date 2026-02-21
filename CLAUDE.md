# Seycore

RSS feed syncing and fetching core library, part of the Seymour project.

## Code organization

```
src/
  lib.rs      — Public API: Feed struct, Storage trait, Error enum
  main.rs     — CLI binary (clap): command handlers, table formatting, MockStore + golden tests
  sqlite.rs   — SQLite Storage implementation (Store), migrations
testdata/     — Golden file expected outputs for CLI tests
Cargo.toml    — Crate config; two binaries (cli, uniffi-bindgen) + a library (cdylib/staticlib/lib)
```

- `lib.rs` defines the `Storage` trait that `sqlite::Store` implements. CLI handlers are generic over `impl Storage`.
- `sqlite.rs` uses `rusqlite` + `rusqlite_migration`. `Store::new()` persists to `~/.seymour/data.sqlite3`; `Store::default()` uses an in-memory DB for unit tests.
- `main.rs` contains all CLI command handlers and the `write_table` utility. Tests live in a `#[cfg(test)]` module at the bottom.

## CLI command pattern

Each CLI command handler should accept an `impl Write` parameter for output instead of writing directly to stdout. In `main()`, pass `io::stdout()`. This makes commands testable by passing a `Vec<u8>` buffer in tests.

```rust
fn handle_something(store: &Store, mut out: impl Write) -> anyhow::Result<()> {
    writeln!(out, "output")?;
    Ok(())
}

// in main:
handle_something(&store, io::stdout())?;

// in tests:
let mut buf = Vec::new();
handle_something(&MockStore::default(), &mut buf)?;
let output = String::from_utf8(buf).unwrap();
assert_eq!(output, golden("something.txt"));
```

## CLI test mock

Tests in `src/main.rs` use a `MockStore` instead of a real SQLite `Store`. `MockStore` implements `Storage` with deterministic feed data (fixed UUIDs, timestamps, etc.) so that output is fully reproducible for golden file comparisons. Use `MockStore::default()` in all CLI handler tests.

## Golden file tests

Expected CLI output lives in `testdata/*.txt`. Tests compare handler output against these files using the `golden(name)` helper:

```rust
assert_eq!(output, golden("something.txt"));
```

When a handler's output format changes, update the corresponding file in `testdata/`. Golden files should match the handler output exactly, including trailing newlines.

## Build and verification loop

After making changes, always run this loop to verify correctness:

1. `cargo build --bin cli` — ensure the project compiles
2. `cargo test` — run all unit and golden file tests
3. If golden file tests fail because of an intentional output change, run the CLI command manually (e.g. `cargo run --bin cli -- feeds`) and update the corresponding `testdata/*.txt` file with the new expected output
4. If adding a new CLI command, build and run it with `cargo run --bin cli -- <command> [args]` to smoke-test the output before writing the golden file
