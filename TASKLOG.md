# Task Log

This file tracks user-requested work and delegated agent tasks in progress on this branch.

## Workflow

- Add or update a task entry before starting substantial work.
- Add or update a task entry before spawning a subagent.
- Keep the task status current as work moves from planned to running to completed.
- Link any resulting notes in `subagents/` when they exist.
- Group related tasks under the most relevant category so the log stays easy to scan.

## Active

### Platform Attach And Cloud Targets

- 2026-03-25 | Task: Follow up with an `inspektor-gadget` attach backend implementation for Kubernetes and AWS EKS targets | Owner: Codex | Status: planned | Output: pending
- 2026-03-25 | Task: Follow up with a `tetragon` attach backend implementation for long-running Kubernetes and AWS EKS targets | Owner: Codex | Status: planned | Output: pending
- 2026-03-25 | Task: Evaluate and scope `aws-ecs` attach support for EC2 launch type after the EKS path is stable | Owner: Codex | Status: planned | Output: pending

## Completed

### Developer Tooling

- 2026-03-29 | Task: Verify the Docker cleanup helper is commit-ready as a standalone topic and only targets this repo's Compose projects by default | Owner: Codex | Status: completed | Output: dry-run verification notes, commit-ready verdict: yes
- 2026-03-29 | Task: Restrict the Docker cleanup helper so it only removes `ebpf-tracker`-owned Compose stacks for this repo instead of generic cache volumes | Owner: Codex | Status: completed | Output: `scripts/docker-cleanup.sh`, `README.md`, `CHANGELOG.md`, `TASKLOG.md`
- 2026-03-25 | Task: Add a repo-local Docker cleanup script for reclaiming disk space from tracing runtimes and Jaeger helpers | Owner: Codex | Status: completed | Output: `scripts/docker-cleanup.sh`, `README.md`, `TASKLOG.md`

### Workflow And Project Hygiene

- 2026-03-25 | Task: Add task tracking workflow and root task log | Owner: Codex | Status: completed | Output: `AGENT.md`, `TASKLOG.md`
- 2026-03-25 | Task: Reorganize `TASKLOG.md` into related categories for easier scanning | Owner: Codex | Status: completed | Output: `TASKLOG.md`
- 2026-03-25 | Task: Review worktree and commit ready changes with selective staging | Agent: Kepler | Status: completed | Output: `3a2a22e`, `AGENT.md`, `TASKLOG.md`, `subagents/meridian-docs-audit.md`, `subagents/initiative-completion-audit.md`

### Repository Understanding And Onboarding

- 2026-03-25 | Task: Explore repository structure and onboarding flow | Agent: Halley | Status: completed | Output: `subagents/halley-repo-onboarding-summary.md`

### Dashboard And Frontend Review

- 2026-03-25 | Task: Review dashboard extension from a frontend perspective and identify top improvements | Agent: Einstein | Status: completed | Output: `subagents/einstein-dashboard-frontend-review.md`

### Customer Experience Review

- 2026-03-25 | Task: Review repository from the customer experience standpoint and identify top improvements | Agent: Euclid | Status: completed | Output: `subagents/euclid-customer-experience-review.md`

### Platform Attach And Cloud Targets

- 2026-03-29 | Task: Verify the `attach` scaffold is commit-ready as a standalone topic, including its AWS scope wording and tests | Owner: Codex | Status: completed | Output: attach CLI verification notes, commit-ready verdict: yes
- 2026-03-29 | Task: Audit the `attach` scaffold/docs for a commit-ready boundary and tighten first-wave AWS scope wording in the scaffold output | Owner: Codex | Status: completed | Output: `src/attach.rs`, `README.md`, `TASKLOG.md`
- 2026-03-29 | Task: Explicitly document `aws-eks` and `aws-ecs` Fargate limitations for attach mode and keep them out of the first-wave scope | Owner: Codex | Status: completed | Output: `src/attach.rs`, `README.md`, `TASKLOG.md`
- 2026-03-25 | Task: Scaffold a new `attach` command and backend adapter layer while preserving the existing managed-runtime flow | Owner: Codex | Status: completed | Output: `src/attach.rs`, `src/lib.rs`, `README.md`, `TASKLOG.md`
- 2026-03-25 | Task: Define and document the first-wave AWS attach target around EKS on EC2 with existing eBPF backends | Owner: Codex | Status: completed | Output: `src/attach.rs`, `src/lib.rs`, `README.md`, `TASKLOG.md`

### Dataset And Learning Pipeline Exploration

- 2026-03-25 | Task: Evaluate a local agent or extension for turning stream output into a reusable dataset | Agent: Euler | Status: completed | Output: `subagents/euler-stream-dataset-feasibility.md`
- 2026-03-25 | Task: Demonstrate the dataset feature by running it on a real trace and verifying it with tests | Owner: Codex | Status: completed | Output: `datasets/replay-demo-20260324-212957/`, `datasets/synthetic-jsonl-demo/`
- 2026-03-25 | Task: Run the full end-to-end `cargo demo --emit jsonl ... | cargo dataset ...` flow and verify local Docker-backed tracing support | Owner: Codex | Status: completed | Output: `datasets/e2e-session-io-demo/`, terminal verification notes

### Product Entry And CLI Ergonomics

- 2026-03-25 | Task: Add a short CLI action to launch the product demo experience | Owner: Codex | Status: completed | Output: `src/lib.rs`, `.cargo/config.toml`, `README.md`

### Documentation And Onboarding

- 2026-03-25 | Task: Audit and refresh README/documentation files for the current product surface | Agent: Meridian | Status: completed | Output: `README.md`, `docs/trace-payment-engine.md`, `examples/README.md`, `examples/session-io-demo/README.md`, `crates/ebpf-tracker-viewer/README.md`, `crates/ebpf-tracker-dataset/README.md`, `subagents/meridian-docs-audit.md`

### Initiative Review And Closure

- 2026-03-25 | Task: Review prior subagent outputs and determine whether the initiative is complete | Agent: Atlas | Status: completed | Output: `subagents/initiative-completion-audit.md`, `TASKLOG.md`
