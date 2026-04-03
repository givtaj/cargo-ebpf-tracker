# eBPF_tracker

`eBPF_tracker` is the installable CLI in this workspace. It wraps commands such
as `cargo run` or `npm run`, runs them inside a Linux Docker runtime, and
attaches tracing for the lifetime of that session.

This README stays intentionally high level. The root should explain the product
and the workspace shape. Detailed behavior belongs in the crate READMEs and
example READMEs linked below.

## What It Does

- runs Rust and Node commands inside a managed Linux tracing runtime
- traces the full wrapped session, not only the final app process
- supports `bpftrace` by default and `perf trace` as an alternate transport
- can emit raw terminal output or JSONL event streams
- can launch a repo-local dashboard and replay stored sessions

If you run `eBPF_tracker cargo run`, you should expect to see the whole session:
`cargo`, `rustc`, linkers, and then your app. The same idea applies to Node
commands such as `npm run <script>`.

## Requirements

- Rust toolchain
- Docker Desktop or another Docker engine that supports privileged containers
- Node.js on the host only if you want the repo-local dashboard/viewer

## Install

Install from a local clone:

```bash
cargo install --path . --locked
```

Install from GitHub:

```bash
cargo install --git https://github.com/givtaj/cargo-ebpf-tracker --locked
```

The first public release is GitHub-release-first. Use `cargo install --git ...`
or a tagged source checkout; the workspace crates are not published to
crates.io yet.

That installs the `eBPF_tracker` binary only. Repo-local helpers such as
`cargo demo`, `cargo see`, `cargo dataset`, `cargo otel`, `cargo jaeger`, and
`cargo viewer` are workspace aliases for contributors working from a clone of
this repository.

Runtime assets are materialized under `~/.cache/ebpf-tracker` by default. Set
`EBPF_TRACKER_CACHE_DIR=/your/path` to override that location.

## Quick Start

First smoke test after install:

```bash
eBPF_tracker --help
```

First real tracing smoke test (requires Docker support):

```bash
eBPF_tracker /bin/true
```

Basic Rust session:

```bash
eBPF_tracker cargo run
```

Basic Node session:

```bash
eBPF_tracker npm test
```

JSONL stream:

```bash
eBPF_tracker --emit jsonl cargo run
```

Alternate `perf` transport:

```bash
eBPF_tracker --transport perf --emit jsonl cargo run
```

Repo-local dashboard:

```bash
./target/debug/eBPF_tracker --dashboard cargo run
./target/debug/eBPF_tracker see
cargo see
```

Project config is optional. If `ebpf-tracker.toml` exists in the current
project, it is loaded automatically. See
[`ebpf-tracker.toml.example`](./ebpf-tracker.toml.example).

## Main Modes

- **`run`**: the default mode; trace a wrapped command inside the managed Docker runtime
- **`demo`**: run one of the repo example manifests under `examples/`
- **`see`**: shortcut for the repo-local dashboard demo experience
- **`attach`**: scaffold-only today; validates targets and prints the intended integration path, but does not start tracing yet

Examples:

```bash
eBPF_tracker cargo test
eBPF_tracker --runtime node /bin/sh -lc "npm run dev"
eBPF_tracker demo --list
eBPF_tracker demo --dashboard session-io-demo
eBPF_tracker attach k8s --selector app=payments
```

Dashboard runs force `--emit jsonl` and preserve replay logs for later review.
Regular `run` sessions log under the current project's `./logs`. `demo` and
`see` sessions log under `examples/<demo-name>/logs/`.

## Workspace Map

- **Root CLI (`eBPF_tracker`)**: installable command-line entry point, runtime orchestration, config loading, `demo`, `see`, and `attach` flow. Source lives under [`src/`](./src).
- **`crates/ebpf-tracker-events`**: shared event schema, line parsers, and session aggregation helpers used across the workspace. [README](./crates/ebpf-tracker-events/README.md)
- **`crates/ebpf-tracker-dataset`**: dataset bundle writer and analyzer for JSONL streams and replay logs. [README](./crates/ebpf-tracker-dataset/README.md)
- **`crates/ebpf-tracker-otel`**: OTLP exporter plus local Jaeger helper commands. [README](./crates/ebpf-tracker-otel/README.md)
- **`crates/ebpf-tracker-perf`**: `perf trace` normalizer and transport boundary for the non-default collector path. [README](./crates/ebpf-tracker-perf/README.md)
- **`crates/ebpf-tracker-viewer`**: browser dashboard and replay viewer. [README](./crates/ebpf-tracker-viewer/README.md)

These crates do separate jobs. The root README should not duplicate their full
behavior contracts.

## Repo-Local Commands

If you are working from a clone of this repo, the Cargo aliases are:

- `cargo demo`: run example manifests from [`examples/`](./examples/README.md)
- `cargo see`: shortcut for the default dashboard demo flow
- `cargo viewer`: launch the live viewer or replay stored sessions
- `cargo dataset`: capture or analyze dataset bundles
- `cargo otel`: export JSONL sessions over OTLP
- `cargo jaeger`: manage the local Jaeger stack used by the OTLP flow

Each command has its own module-level README above.

## Examples

Runnable examples live under [`examples/`](./examples/README.md):

- [`examples/session-io-demo`](./examples/session-io-demo/README.md): build-time plus runtime file, network, and output activity in one trace
- [`examples/postcard-generator-rust`](./examples/postcard-generator-rust/README.md): visible postcard-generation flow in Rust
- [`examples/postcard-generator-node`](./examples/postcard-generator-node/README.md): the same visible workflow in Node.js

Useful repo-local entry points:

```bash
cargo demo
cargo demo --emit jsonl session-io-demo
cargo demo --transport perf --emit jsonl session-io-demo
cargo viewer --replay examples/session-io-demo/logs/ebpf-tracker-YYYYMMDD-HHMMSS.log
bash scripts/dashboard-smoke.sh
```

## Current Limits

- `attach` is scaffold-only right now
- the default collector is still `bpftrace`
- the alternate collector is Linux `perf trace`, not a direct perf-event-array or ring-buffer path
- there is no stable target-only or process-tree-only filtering yet
- the viewer is browser-first; it is not a separate native TUI

## More Docs

- [`docs/cli.md`](./docs/cli.md)
- [`examples/README.md`](./examples/README.md)
- [`docs/trace-payment-engine.md`](./docs/trace-payment-engine.md)
- [`CONTRIBUTING.md`](./CONTRIBUTING.md)
- [`SECURITY.md`](./SECURITY.md)
- [`RELEASE.md`](./RELEASE.md)
