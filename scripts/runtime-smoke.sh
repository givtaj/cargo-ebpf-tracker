#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"

fail() {
  printf 'runtime smoke: %s\n' "$1" >&2
  exit 1
}

command -v cargo >/dev/null 2>&1 || fail "cargo is required"
command -v docker >/dev/null 2>&1 || fail "docker is required"
docker info >/dev/null 2>&1 || fail "docker daemon is not available; start Docker and retry"

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/ebpf-tracker-runtime-smoke.XXXXXX")"
output_file="${tmp_root}/trace.jsonl"

cleanup() {
  rm -rf "${tmp_root}"
}

trap cleanup EXIT INT TERM

cd "${repo_root}"

printf '[runtime-smoke] running minimal traced session with /bin/true\n'

if ! cargo run --locked --quiet --bin eBPF_tracker -- --emit jsonl /bin/true >"${output_file}"; then
  fail "minimal traced session failed"
fi

if [[ ! -s "${output_file}" ]]; then
  fail "no JSONL records were emitted"
fi

if ! grep -q '"type":"syscall"' "${output_file}"; then
  fail "smoke run completed but did not emit syscall records"
fi

record_count="$(wc -l < "${output_file}" | tr -d '[:space:]')"
printf '[runtime-smoke] captured %s JSONL record(s)\n' "${record_count}"
printf '[runtime-smoke] tracer path looks healthy\n'
