# flowistry-cli

A command-line interface for [Flowistry](https://github.com/willcrichton/flowistry) information-flow analysis on Rust programs.

Flowistry answers questions like "which lines of code affect this value?" and "what does this expression influence?". `flowistry-cli` exposes that analysis as a clean CLI suitable for scripting, CI pipelines, and MCP tools.

## Rationale
While Flowistry already exposes a CLI, it's not suitable for human consumption - it's single-mindedly dedicated to be used by an IDE extension as a client. This tool aims to expose it in a more palatable way for more general usages.

> [!NOTE]
> Eventual end-goal is to build an MCP server that exposes Flowistry magic as tools for LLMs.

## Requirements

- Rust nightly `2025-08-20` with components `rust-src`, `rustc-dev`, and `llvm-tools-preview` (the `rust-toolchain.toml` in this repo sets this up automatically)
- A `Cargo.toml`-based project to analyse

## Installation

From this repository:

```sh
cargo install --path .
```

This installs two binaries: `cargo-flowistry` (the user-facing entry point) and `flowistry-driver` (the rustc wrapper, invoked automatically).

## Usage

All commands are run from the root of the Rust project you want to analyse, as `cargo flowistry <subcommand>`. Positions use **1-based** line and column numbers.

### `slice` — backward slice

Which locations in the file influence the target range?

```sh
cargo flowistry slice src/main.rs:42:7
cargo flowistry slice src/main.rs:42:7-44:15
```

### `influence` — forward slice

Which locations are influenced by the target range?

```sh
cargo flowistry influence src/main.rs:10:5
cargo flowistry influence src/main.rs:10:5-10:20
```

### `spans` — list analysable ranges

List every function body in a file that Flowistry can analyse:

```sh
cargo flowistry spans src/main.rs
```

### `focus` — IDE focus mode

For every analysable location in the enclosing function, return its bidirectional slice. Useful for building "fade out irrelevant code" features in editors.

```sh
cargo flowistry focus src/main.rs:42:7
```

### `version`

Print the crate version and the rustc version this binary was built against:

```sh
cargo flowistry version
```

## Output formats

### Default (human-readable text)

```
$ cargo flowistry slice src/lib.rs:42:7-44:15

Slice of src/lib.rs:42:7-44:15
target  src/lib.rs:42:7-44:15
3 dependencies:

  src/lib.rs:10:3-10:18
  src/lib.rs:7:5-7:12
  src/lib.rs:3:1-3:8
```

### `--json`

Pretty-printed JSON:

```sh
cargo flowistry slice src/lib.rs:42:7 --json
cargo flowistry slice src/lib.rs:42:7 --json | jq '.dependencies | length'
```

```json
{
  "command": "slice",
  "target": { "file": "src/lib.rs", "start": { "line": 42, "col": 7 }, "end": { "line": 42, "col": 7 } },
  "dependencies": [
    { "file": "src/lib.rs", "start": { "line": 10, "col": 3 }, "end": { "line": 10, "col": 18 } }
  ]
}
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Analysis error (code compiled but analysis failed) |
| 2 | Build error (rustc could not compile the target) |
| 3 | Usage error (bad arguments, file not found, bad location format) |
