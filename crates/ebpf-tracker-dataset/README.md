# ebpf-tracker-dataset

Dataset writer and analyzer for `ebpf-tracker` JSONL streams and replay logs.

The root `eBPF_tracker` CLI can also run this crate in a supervised background
mode via `--intelligence-dataset`, so end users do not have to pipe JSONL into
`cargo dataset` manually just to capture and analyze a session.

Examples:

```bash
eBPF_tracker --emit jsonl cargo run | cargo dataset --test-name cargo-run-smoke
cargo dataset --replay logs/ebpf-tracker-YYYYMMDD-HHMMSS.log
cargo dataset analyze --run datasets/<run-id> --provider lm-studio --model qwen/qwen3.5-9b
cargo dataset analyze --run datasets/<run-id> --provider lm-studio --model qwen/qwen3.5-9b --live-logs
```

Each run writes a bundle under `./datasets/<run-id>/`:

- `run.json`: run metadata and dataset pointers
- `events.jsonl`: normalized typed stream records
- `processes.json`: per-process rollup
- `aggregates.json`: aggregate metrics from the stream
- `features.json`: derived focus process, top files, top writes, and kind counts

Model analysis writes into `./datasets/<run-id>/analysis/`.
The first adapter is LM Studio over its local server at
`http://127.0.0.1:1234`. The `lm-studio` provider uses LM Studio's native chat
API with reasoning disabled so local Qwen reasoning models produce final
analysis text instead of reasoning-only payloads. The CLI also supports a
generic `openai-compatible` provider so the same analysis flow can move to
stronger models later without changing the dataset format.

Use `--live-logs` to stream dataset-analyzer progress plus the current LM Studio
server log lines to `stderr` while the request is in flight. The same log stream
is also persisted under `./datasets/<run-id>/analysis/*.live.log`.
