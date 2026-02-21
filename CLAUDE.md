# Seycore

RSS feed syncing and fetching core library, part of the Seymour project.

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
