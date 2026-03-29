# ebpf-tracker-perf

Transport crate for the non-default `perf` path and future native
kernel-to-userspace transport work.

What this crate does today:

- normalize Linux `perf trace` output into `StreamRecord` values
- keep the CLI's transport boundary separate from the default `bpftrace` +
  stdout path
- provide a small userspace counter layer for aggregate metrics
- leave perf-event-array and ring-buffer decisions out of the CLI crate

Current coverage:

- the root CLI can run with `--transport perf`
- `parse_perf_trace_line` accepts the current `perf trace` shape used by this
  project: `timestamp: comm/pid syscall(args) = return`
- supported syscall kinds are `execve`, `openat`, `write`, and `connect`
- `stream_record_for_perf_trace_line_at` turns supported lines into
  `StreamRecord::Syscall`
- `PerfTraceSession` emits `StreamRecord::Aggregate` values for `execve`,
  `openat`, `writes`, and `connects`
- `default_perf_event_kinds()` currently returns `execve` only, which matches
  the default probe path in the CLI
- `default_transport_plan()` documents the current split between the shipped
  `bpftrace` stdout path, the available `perf trace` path, and future
  perf-event-array / ring-buffer work

Current limitations:

- file-path arguments are best-effort and may be omitted when `perf trace`
  cannot decode userspace string pointers
- `write` and `connect` parsing only captures the fields this crate already
  normalizes: `count`/`len` for bytes and `fd`/`sockfd` for file descriptors
- unsupported syscall names and malformed lines are ignored rather than
  partially decoded
- this crate does not provide direct perf-event-array or ring-buffer capture
  yet

Future work here:

- direct perf-event-array transport
- ring buffer transport
- richer syscall argument decoding than plain `perf trace` can provide
