# Tardigrade CI Backlog

This file is the delivery backlog derived from the current roadmap.

## Team working agreement

- From now on, priorities are reminded at the start of backlog-related discussions.
- Every new feature request must be tracked in this file before implementation.
- New entries should use an epic prefix-based ID (examples: `INDUS-201`, `DSL-01`, `SCM-03`).
- Each new entry must include: goal, scope, status, and acceptance criteria.
- Before starting any epic, we run an explicit refinement step (affinage) together and record the outcome in the backlog discussion.

### Priority reminder (current)

1. Epic 2 (`SCM-*`) for external trigger automation.
2. Epic 3 (`PLUG-*`) for extension safety model.
3. Reliability follow-ups (`REL-*`) as hardening milestones.
4. Epic 0 (`INDUS-*`) hardening follow-ups.
5. Epic 5 (`CLOUD-*`) cloud/container delivery track (deferred).

Context update:

- Epic 1 (`DSL-*`) is complete.
- Epic 1b (`TECH-*`) is complete.
- Docker/cloud scope has been deferred and tracked under Epic 5 (`CLOUD-*`).

## Status legend

- `[ ]` not started
- `[-]` in progress
- `[x]` done

## Prioritized epics

### Epic 0: Industrialization and build orchestration

Goal: provide one consistent command surface to build/test/package the full project (Rust, Node, Docker).

Refinement decisions:

- MVP includes `make ci` from day one.
- `bootstrap` remains Rust-only at this stage.
- `test-fast` runs unit tests only; `test-all` runs unit + integration scope.
- Docker scope includes image builds and security scan.
- Make setup is modular (`Makefile` + included `.mk` files).

- [x] `INDUS-201` Add modular Make structure (`Makefile` + `mk/*.mk`) and root command entrypoints.
- [x] `INDUS-202` Add proxy-safe defaults across all Make targets (matching repository conventions).
- [x] `INDUS-203` Add Rust-only bootstrap target (`make bootstrap`).
- [x] `INDUS-204` Add quality targets (`make lint`, `make fmt-check`, `make clippy`).
- [x] `INDUS-205` Add test matrix targets: `make test-fast` (unit only) and `make test-all` (unit + integration).
- [x] `INDUS-206` Add Node/dashboard targets via `xtask` (`make dashboard-lint`, `make dashboard-build`).
- [x] `INDUS-207` Add Docker targets for server/worker build + security scan.
- [x] `INDUS-208` Add `make ci` as canonical local/CI aggregate target.
- [x] `INDUS-209` Add discoverability target and docs (`make help` + README command matrix).
- [x] `INDUS-210` Wire CI pipeline to call Make targets as canonical entrypoints.
- [x] `INDUS-211` Remove CI redundancy by replacing full `make ci` control rerun with a lightweight pipeline-summary gate.

Definition of done:

- A new contributor can run one command to bootstrap and one command to run the full CI-equivalent flow.
- CI and local developer workflows use the same Make entrypoints.
- Rust, Node, and Docker builds are reproducible from repository root.

### Epic 1: Pipeline DSL (YAML) parser and validator

Goal: make pipeline definitions explicit, validated, and versioned.

- [x] `DSL-01` Define pipeline schema (`version`, `stages`, `steps`, retry policy hooks).
- [x] `DSL-02` Add YAML parser/validator crate integration (`serde_yaml` + structural validation).
- [x] `DSL-03` Add API validation path for pipeline files before build enqueue.
- [x] `DSL-04` Add clear error model for invalid pipeline definitions (HTTP + GraphQL surfaces).
- [x] `DSL-05` Add tests for valid/invalid DSL samples and edge cases.
- [x] `DSL-06` Document DSL format with examples in docs/README.

Definition of done:

- Pipeline file can be parsed and validated deterministically.
- Invalid definitions return actionable errors.
- Tests cover happy path and common failure modes.

### Epic 1b: Multi-technology pipeline execution profiles

Goal: make Tardigrade CI clearly usable for Rust, Python, Java, and other stacks with first-class examples and execution defaults.

- [x] `TECH-01` Define technology profile model (language/runtime/build strategy metadata).
- [x] `TECH-02` Provide built-in profile catalog for Rust, Python, Java, Node, and Go.
- [x] `TECH-03` Add pipeline examples for each profile in docs (`docs/pipeline-recipes.md`).
- [x] `TECH-04` Add validation hints/recommendations (non-blocking) for common language pitfalls.
- [x] `TECH-05` Add end-to-end smoke matrix (at least Rust + Python + Java templates).
- [x] `TECH-06` Document onboarding flow for adding a new stack profile.

Definition of done:

- A user can bootstrap a valid pipeline quickly for at least Rust, Python, and Java.
- Multi-stack behavior is documented with copy/paste-ready pipeline examples.
- Validation and test matrix reduce regressions across supported stacks.

### Epic 2: Webhook triggers and SCM polling

Goal: trigger builds from SCM events and periodic repository checks.

Refinement decisions (MVP):

- Delivery mode: webhook + polling in parallel.
- First providers: GitHub and GitLab.
- Trigger events: `push`, `pull_request` / `merge_request`, `tag`, and manual dispatch.
- Webhook security baseline: signature verification + IP allowlist.
- Dedup strategy: provider `event_id` first, fallback to (`repo`, `commit_sha`, `event_type`).
- Polling scope: branch polling by default (`main` / `master` + configured branches).
- Ingestion failure behavior: return error, emit logs, emit metrics.

Refinement outcome for `SCM-02` (2026-03-31):

- Unified endpoint path: `/webhooks/scm`.
- Secret source: per-repository secrets stored in persistence layer (database-backed model).
- Validation mode: strict reject on missing/invalid signature (`401/403`).
- Replay defense: timestamp window set to 5 minutes.
- Security scope for current step: implement signature verification and IP allowlist together.

- [x] `SCM-01` Define trigger model (manual, webhook, polling).
- [x] `SCM-02` Add webhook endpoint(s) with signature verification.
- [ ] `SCM-03` Implement provider adapters (GitHub/GitLab first).
- [ ] `SCM-04` Add SCM polling worker loop and configurable intervals.
- [ ] `SCM-05` Add deduplication/idempotency for repeated webhook events.
- [ ] `SCM-06` Add observability events/metrics for trigger ingestion.
- [ ] `SCM-07` Add integration tests for webhook and polling flows.

Definition of done:

- A push event can enqueue builds via webhook.
- Polling can detect and trigger builds reliably.
- Duplicate events do not produce duplicate builds.

### Epic 3: Plugin loading and permissions model

Goal: move from in-memory plugin registry to a controllable runtime plugin system.

- [ ] `PLUG-01` Define plugin lifecycle (`load`, `init`, `execute`, `unload`).
- [ ] `PLUG-02` Add plugin discovery/loading strategy (filesystem manifest first).
- [ ] `PLUG-03` Add plugin capability model (network, fs, secrets, runtime hooks).
- [ ] `PLUG-04` Add authorization checks for plugin capabilities.
- [ ] `PLUG-05` Add plugin isolation/guardrails and failure containment.
- [ ] `PLUG-06` Add tests for duplicate names, denied capabilities, and load failures.
- [ ] `PLUG-07` Document plugin authoring and permission declaration.

Definition of done:

- Plugins can be loaded from declared sources.
- Permission checks are enforced before sensitive actions.
- Failure in one plugin does not crash core orchestration.

### Epic 4: Redis-first production scheduler mode

Goal: make Redis the default production scheduler path and reduce file-backed usage to local/dev only.

- [x] `SCHED-01` Add explicit runtime mode selection (`dev`, `prod`).
- [x] `SCHED-02` Enforce Redis scheduler for production mode.
- [x] `SCHED-03` Remove file-backed scheduler from runtime fallback path.
- [x] `SCHED-04` Add startup diagnostics when production prerequisites are missing.
- [x] `SCHED-05` Update deployment manifests and docs to reflect Redis-first behavior.
- [x] `SCHED-06` Add migration notes for users still relying on file-backed queue in clustered runs.

Definition of done:

- Production mode cannot start without Redis configuration.
- Local developer workflow remains simple with non-Redis fallback.

### Epic 5: Cloud/container delivery track (deferred)

Goal: reintroduce and harden container/cloud delivery later, after current SCM and platform priorities.

Status: deferred for a later planning cycle.

- [ ] `CLOUD-01` Reintroduce reproducible Linux container builds for server and worker.
- [ ] `CLOUD-02` Define image tagging/versioning policy for local, CI, and release channels.
- [ ] `CLOUD-03` Add registry publish flow and pull authentication model.
- [ ] `CLOUD-04` Rebuild Kubernetes baseline manifests (server=1, worker=2, dependencies).
- [ ] `CLOUD-05` Add cloud smoke checks (health endpoints, worker registration, basic pipeline run).
- [ ] `CLOUD-06` Document deployment and troubleshooting runbook (TLS, pull failures, architecture mismatch).

Definition of done:

- Tardigrade can be deployed from published images to a Kubernetes cluster with documented steps.
- CI validates image/runtime architecture compatibility before deployment.
- Operational runbook covers local cluster and cloud deployment paths.

## Queue reliability follow-ups

### Completed

- [x] Redis-backed queue scheduler (distributed claim/ack/requeue).
- [x] Worker ownership check on build completion (`409` on mismatch).
- [x] Stale lease reclaim with configurable timeout (`TARDIGRADE_WORKER_LEASE_TIMEOUT_SECS`).
- [x] Runtime metrics API (`GET /metrics`) with:
  - `reclaimed_total`
  - `retry_requeued_total`
  - `ownership_conflicts_total`
  - `dead_letter_total`
- [x] Dashboard panel displaying runtime metrics in real time.
- [x] Real-time event stream (`GET /events`) wired to dashboard live feed.
- [x] Dead-letter flow for builds exceeding max retries (`GET /dead-letter-builds`) visible in dashboard.

### Remaining

- [ ] `REL-01` Retry policy refinement (configurable caps per job profile).
- [ ] `REL-02` Metrics persistence/export (Prometheus/OpenTelemetry).

## Suggested delivery order

1. Epic 2 (`SCM-*`) for external trigger automation.
2. Epic 3 (`PLUG-*`) for extension safety model.
3. Reliability follow-ups (`REL-*`) as hardening milestones.
4. Epic 0 (`INDUS-*`) hardening follow-ups.
5. Epic 5 (`CLOUD-*`) cloud/container delivery track (deferred).
