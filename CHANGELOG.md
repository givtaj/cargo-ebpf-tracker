# Changelog

This file tracks notable repo changes in progress on this branch.

## Unreleased

### Added

- Added the `ebpf-tracker-viewer` workspace crate to own the dashboard and replay viewer.
- Added the `ebpf-tracker-dataset` workspace crate to turn JSONL streams and replay logs into per-run dataset bundles.
- Added dataset analysis support for local or remote OpenAI-compatible backends, including LM Studio defaults.
- Added a `cargo viewer` workspace alias for launching the viewer locally.
- Added a `cargo dataset` workspace alias for launching the dataset tool locally.
- Added a typed `session` stream record for demo branding metadata.
- Added demo manifest branding fields and propagated them into demo runtime environment variables.
- Added an `eBPF_tracker see` shortcut and matching `cargo see` alias for the default dashboard demo flow.
- Added root agent workflow guidance in `AGENT.md`.
- Added an initial `attach` CLI scaffold and backend adapter layer so customer-owned container and Kubernetes targets can sit beside the existing managed runtime path.

### Changed

- Moved the live trace matrix viewer asset out of `scripts/` into `crates/ebpf-tracker-viewer`.
- Wired dashboard mode to preserve replayable logs and documented replay via the viewer crate.
- Bundled small replay samples into the viewer crate and refreshed the README/docs wording around the current event schema.
- Moved session-trace construction into `ebpf-tracker-events` so multiple consumers can share the same trace summary model.
- Tightened viewer-side noise filtering for infra and toolchain file paths in the live matrix dashboard.
- Updated examples and docs to describe replay flow, manifest-driven demos, and branded demo artifacts.
- Updated docs to describe dataset capture, replay-log ingestion, local analysis, and the shorter `see` entrypoint.
- Capped dataset analysis prompt sections so LM Studio can accept runs on smaller `4096`-token local contexts.
- Added an `--intelligence-dataset` flow that supervises dataset capture and LM Studio analysis from the main tracer, with live dashboard status.
- Added optional live dataset-analysis tracing so LM Studio server logs and analyzer progress can be watched in real time and persisted per run.
- Switched the LM Studio dataset analyzer path to LM Studio's native chat API with reasoning disabled so local Qwen models return final analysis content reliably.
- Made dataset-analysis live logging truly opt-in instead of always echoing analyzer progress on `stderr`.
- Documented AWS-first attach scoping around EKS on EC2 and captured the remaining backend/platform follow-up work in `TASKLOG.md`.
- Clarified the README vocabulary for `run` versus `attach` and made the attach direction explicitly depend on existing eBPF backends instead of a homegrown Kubernetes control plane.
- Scoped the Docker cleanup helper to tracked `ebpf-tracker` Compose projects so it no longer removes generic cache volumes or prunes global Docker cache unless `--all` is requested.
