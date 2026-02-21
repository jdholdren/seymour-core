# seycore

RSS feed syncing core library for the Seymour project. Provides a CLI and a Rust library with a C-compatible FFI layer for use in native clients (e.g. Swift via UniFFI).

## Structure

```
src/
  lib.rs          Core<S,F> service object, Storage/Fetcher traits, Error enum
  sqlite.rs       SQLite implementation of Storage (~/.seymour/data.sqlite3)
  http.rs         HTTP + RSS/Atom parsing implementation of Fetcher
  ffi.rs          FFICore: concrete wrapper for UniFFI/Swift consumers
  main.rs         CLI binary
testdata/         Golden file expected outputs for CLI tests
Cargo.toml
Makefile
```

## Building

```
cargo build --bin cli
```

Run all tests:

```
cargo test
```

Generate Swift FFI bindings (outputs to `out/`):

```
make bindgen
```

## CLI commands

| Command | Description |
|---|---|
| `feeds` | List all tracked feeds |
| `feeds <id>` | Describe a single feed |
| `add <url>` | Add and sync a feed |
| `entries <feed-id>` | List approved entries for a feed |
| `entries <feed-id> --all` | List all entries including unapproved |
| `timeline` | Show approved entries across all feeds, newest first |
| `sync-all` | Re-sync all feeds from their sources |
