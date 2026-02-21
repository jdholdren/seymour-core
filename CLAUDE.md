# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Seycore

RSS feed syncing and fetching core library, part of the Seymour project.

## Code organization

```
src/
  lib.rs      — Public API: Feed/FeedEntry structs, Storage/Fetcher traits, Error enum, Core<S,F>
  ffi.rs      — FFICore: concrete wrapper around Core for UniFFI/Swift consumers
  http.rs     — FeedFetcher: HTTP + RSS/Atom parsing implementation of Fetcher
  sqlite.rs   — Store: SQLite implementation of Storage, migrations
  main.rs     — CLI binary (clap): command handlers, write_table, MockStore + golden tests
testdata/     — Golden file expected outputs for CLI tests
Cargo.toml    — Crate config; two binaries (cli, uniffi-bindgen) + a library (cdylib/staticlib/lib)
```

## Service object architecture

`Core<S: Storage, F: Fetcher>` is the central service object defined in `lib.rs`. It owns:
- A `Mutex<S>` — the storage layer
- An `F` — the fetcher (reserved for orchestration: refresh, judge, etc.)

It is generic so that tests can inject `MockStore` and `MockFetcher` without hitting the DB or network. CLI handlers in `main.rs` are generic over `Core<S, F>` for the same reason.

```rust
// Production (main.rs)
let core = Core::new(Store::new()?, FeedFetcher {});

// Tests (main.rs)
fn mock_core() -> Core<MockStore, FeedFetcher> {
    Core::new(MockStore::default(), FeedFetcher {})
}
```

## FFI module

`src/ffi.rs` exists because UniFFI (the Rust→Swift/Kotlin bridge) cannot export generic types. Swift has no concept of Rust type parameters, so `Core<S, F>` cannot be annotated with `#[uniffi::export]` directly.

`FFICore` is a thin concrete wrapper — `Core<Store, FeedFetcher>` — with no logic of its own. All methods delegate to the inner `Core`. When UniFFI annotations are added, they go here, not on `Core`. `Core` itself stays generic and test-friendly.

```
Core<S, F>   — generic, all business logic, fully testable
FFICore      — concrete, zero logic, future UniFFI entry point for Swift
```

## Storage and Fetcher traits

- `Storage` (`lib.rs`): `list_feeds`, `add_feed` (async), `get_feed`, `list_entries`. Implemented by `sqlite::Store`.
- `Fetcher` (`lib.rs`): `fetch(&self, url: &str)` (async). Implemented by `http::FeedFetcher`.
- `Store` (`sqlite.rs`): persists to `~/.seymour/data.sqlite3` via `Store::new()`; `Store::new_in_memory()` opens an in-memory DB for sqlite-level unit tests.
- `FeedFetcher` (`http.rs`): fetches a URL with `reqwest`, parses RSS 2.0 XML with `serde-xml-rs`, converts `pubDate` strings to Unix seconds via `chrono`.

## CLI command pattern

Handlers are generic over `Core<S, F>` and accept `impl Write` for output — never write directly to stdout.

```rust
fn handle_something<S: Storage, F: Fetcher>(
    core: &Core<S, F>,
    mut out: impl Write,
) -> anyhow::Result<()> {
    writeln!(out, "{}", core.list_feeds()?[0].url)?;
    Ok(())
}
```

## CLI tests and golden files

Tests in `src/main.rs` use `mock_core()` (returns `Core<MockStore, FeedFetcher>`) with deterministic feed data (fixed UUIDs, timestamps) for golden file comparisons. Expected output lives in `testdata/*.txt`.

```rust
assert_eq!(output, golden("something.txt"));
```

When output format changes intentionally, update the corresponding `testdata/` file.

## Build and verification loop

1. `cargo build --bin cli` — ensure the project compiles
2. `cargo test` — run all unit and golden file tests
3. If golden file tests fail due to an intentional output change, run `cargo run --bin cli -- <command>` and update `testdata/*.txt`
4. If adding a new CLI command, smoke-test with `cargo run --bin cli -- <command> [args]` before writing the golden file
