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
3. Epic 7 (`UIADM-*`) for product administration IHM coverage.
4. Epic 6 (`REFAC-*`) for Rust source maintainability refactor.
5. Reliability follow-ups (`REL-*`) as hardening milestones.
6. Epic 0 (`INDUS-*`) hardening follow-ups.
7. Epic 5 (`CLOUD-*`) cloud/container delivery track (deferred).
8. Epic 8 (`UXREAL-*`) mockup-to-real-dashboard rollout.

Context update:

- Epic 1 (`DSL-*`) is complete.
- Epic 1b (`TECH-*`) is complete.
- Docker/cloud scope has been deferred and tracked under Epic 5 (`CLOUD-*`).
- Multi-page UX mockup is available and tracked in `UX.md`; implementation is now phased under Epic 8 (`UXREAL-*`).

Release note (2026-04-02):

- Docker/container delivery artifacts and make module were intentionally removed from the active delivery surface.
- Existing non-Docker Make entrypoints remain the canonical local/CI workflow while cloud scope is redesigned under Epic 5 (`CLOUD-*`).

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
- [x] `INDUS-206` Add Node/dashboard targets (`make dashboard-lint`, `make dashboard-build`) through canonical root automation.
- [ ] `INDUS-207` Add Docker targets for server/worker build + security scan.
- [-] `INDUS-208` Add `make ci` as canonical local/CI aggregate target.
- [-] `INDUS-209` Add discoverability target and docs (`make help` + README command matrix).
- [x] `INDUS-210` Wire CI pipeline to call Make targets as canonical entrypoints.
- [x] `INDUS-211` Remove CI redundancy by replacing full `make ci` control rerun with a lightweight pipeline-summary gate.
- [x] `INDUS-212` Add multi-platform release packaging (`make package-platform-zips`) generating mac/windows/linux zip distributions with bin/config/docs/README/LICENSE layout.
- [x] `INDUS-213` Consolidate dashboard delivery in platform zips by exporting assets to top-level `dashboard/` and adding launcher scripts that set `TARDIGRADE_WEB_ROOT`.
- [x] `INDUS-214` Move dashboard frontend sources to repository root (`dashboard/`) and update xtask/CI/docs paths accordingly.
- [x] `INDUS-215` Remove `crates/xtask` and switch dashboard automation to direct Make+npm workflow.
- [x] `INDUS-216` Switch dashboard build output to `target/public` and enforce strict runtime + packaging consumption without legacy fallback.

Definition of done:

- A new contributor can run one command to bootstrap and one command to run the full CI-equivalent flow.
- CI and local developer workflows use the same Make entrypoints.
- Rust, Node, and Docker builds are reproducible from repository root.

Current gap note (2026-04-02):

- Docker make module and related cloud/container artifacts are intentionally removed for a clean reimplementation later in the project lifecycle.
- `INDUS-207` stays pending until the new Docker scope is reintroduced.

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

Refinement outcome for `SCM-04` (2026-03-31):

- Target architecture: dedicated SCM polling service path (introduced with runtime loop support).
- Default polling interval: 30 seconds per repository when configured accordingly.
- Branch scope: all branches (provider adapters can later filter by branch policy).
- Source of truth: polling configuration stored per repository in persistence layer.

Refinement outcome for `SCM-05` (2026-03-31):

- Primary dedup key: provider `event_id` when available in webhook headers.
- Fallback dedup key: (`provider`, `repository`, `commit_sha`, `event_type`).
- Idempotency window: in-memory TTL cache (default 3600s, configurable by env).
- Duplicate behavior: accept webhook (`202`) but skip enqueue side effects.

Refinement outcome for `SCM-06` (2026-03-31):

- Webhook counters: received, accepted, rejected, duplicate.
- Trigger activity counters: builds enqueued from SCM triggers.
- Polling counters: tick count, repositories polled, builds enqueued by polling.
- Exposure path: existing `/metrics` REST and GraphQL dashboard metrics projection.

Refinement outcome for `SCM-07` (2026-03-31):

- Coverage scope: webhook acceptance/rejection, dedup/idempotency, polling tick, and combined webhook+polling path.
- Test level: API integration tests using in-memory state through HTTP routes.
- Acceptance criterion: webhook and polling flows both enqueue builds for matching repository jobs.
- Regression guard: counters in `/metrics` must reflect trigger ingestion outcomes.

- [x] `SCM-01` Define trigger model (manual, webhook, polling).
- [x] `SCM-02` Add webhook endpoint(s) with signature verification.
- [x] `SCM-03` Implement provider adapters (GitHub/GitLab first).
- [x] `SCM-04` Add SCM polling worker loop and configurable intervals.
- [x] `SCM-05` Add deduplication/idempotency for repeated webhook events.
- [x] `SCM-06` Add observability events/metrics for trigger ingestion.
- [x] `SCM-07` Add integration tests for webhook and polling flows.

Definition of done:

- A push event can enqueue builds via webhook.
- Polling can detect and trigger builds reliably.
- Duplicate events do not produce duplicate builds.

### Epic 3: Plugin loading and permissions model

Goal: move from in-memory plugin registry to a controllable runtime plugin system.

Refinement outcome for `PLUG-01` (2026-03-31):

- Lifecycle states: `Loaded` -> `Initialized` -> `Unloaded`.
- Lifecycle operations: explicit `load`, `init`, `execute`, `unload` methods in registry.
- Compatibility: keep `register` as a backward-compatible alias to `load`.
- Error semantics: typed errors for duplicate name, not found, invalid state, and execution failure.

Refinement outcome for `PLUG-02` (2026-03-31):

- Discovery source: filesystem TOML manifest (`plugins.toml`) with `[[plugins]]` entries.
- Entry schema: `name` + optional `enabled` (default `true`).
- Loading strategy: registry reads manifest and asks caller factory to resolve plugin implementations by name.
- Failure policy: fail on unreadable/invalid manifest or unknown plugin references.

Refinement outcome for `PLUG-03` (2026-03-31):

- Capability taxonomy: `network`, `filesystem`, `secrets`, `runtime_hooks`.
- Declaration sources: plugin implementation defaults + optional manifest override.
- Registry metadata: normalized (sorted/deduplicated) capability list per loaded plugin.
- Scope limit: model only in this step; policy enforcement is handled by `PLUG-04`.

- [x] `PLUG-01` Define plugin lifecycle (`load`, `init`, `execute`, `unload`).
- [x] `PLUG-02` Add plugin discovery/loading strategy (filesystem manifest first).
- [x] `PLUG-03` Add plugin capability model (network, fs, secrets, runtime hooks).
- [x] `PLUG-04` Add authorization checks for plugin capabilities.
- [x] `PLUG-05` Add plugin isolation/guardrails and failure containment.
- [x] `PLUG-06` Add tests for duplicate names, denied capabilities, and load failures.
- [x] `PLUG-07` Document plugin authoring and permission declaration.

Definition of done:

- Plugins can be loaded from declared sources.
- Permission checks are enforced before sensitive actions.
- Failure in one plugin does not crash core orchestration.

### Epic 7: Product administration IHM coverage

Goal: provide first-class administration UI for all operational features currently exposed only through API/GraphQL.

Refinement decisions (MVP):

- Prioritize operator workflows over developer/debug-only tooling.
- Reuse existing GraphQL dashboard snapshot/mutations when possible.
- Any new admin action must provide visible success/error feedback in UI.
- Security-sensitive actions require explicit confirmation UX and audit-friendly event messaging.

Refinement outcome for `UIADM-01` (2026-04-02):

- UX scope: add a dedicated "SCM Webhook Security" admin panel in dashboard with repository, provider, secret, and IP allowlist form.
- Primary actions: create/update one repository security config, clear form, and test payload helper (copy/paste headers checklist for provider setup).
- Validation rules: reject empty repository/provider/secret in UI before submit; normalize allowlist as trimmed unique list.
- Security UX: mask secret input by default, explicit reveal toggle, and confirmation prompt before overwriting an existing config.
- Feedback model: show request outcome with clear status banner (`saved`, `invalid input`, `forbidden`, `internal error`) and append event in operator log feed.
- API prerequisite: expose one admin write endpoint (or GraphQL mutation) that maps to existing `UpsertWebhookSecurityConfigRequest` state/service path.
- API compatibility note: current route surface exposes `/webhooks/scm` ingestion but does not yet expose a public admin route for webhook security upsert.
- Test scope: dashboard integration tests for happy path save, invalid form submit, and server error surfacing.

Refinement outcome for `UIADM-02` (2026-04-02):

- UX scope: add a dedicated "SCM Polling" admin panel with repository/provider selector, enable toggle, interval input, and branches editor.
- Primary actions: save polling configuration, disable polling for a repository, and run one manual polling tick from UI.
- Validation rules: repository required, provider required, interval must be integer > 0; branches input normalized to trimmed unique list.
- Trigger UX: manual tick button returns immediate summary (`polled_repositories`, `enqueued_builds`) and writes one operator log line.
- Safety UX: confirmation prompt before disabling an active polling configuration.
- API mapping: use existing `/scm/polling/configs` upsert and `/scm/polling/tick` trigger endpoints.
- State visibility: surface last known polling config values and last tick result in panel for operator verification.
- Test scope: dashboard integration tests for config save/update, validation errors, disable flow, and manual tick outcome rendering.

Refinement outcome for `UIADM-03` (2026-04-02):

- UX scope: add a "Worker Control" panel focused on day-2 diagnostics and manual worker-flow simulation.
- Primary actions: list workers, claim next build for a selected worker id, and complete a claimed build with `success`/`failed` + optional log line.
- Validation rules: worker id required for claim/complete simulation; build id required for completion action; status limited to known enum values.
- Diagnostic UX: display claim result (`no build` vs `build id`), completion result, active builds count, last seen timestamp, and worker status.
- Error UX: expose ownership conflict (`409`) and invalid transitions as explicit operator-friendly messages.
- API mapping: use existing `/workers`, `/workers/{worker_id}/claim`, and `/workers/{worker_id}/builds/{id}/complete` endpoints.
- Safety UX: completion simulation requires explicit confirmation when status is `failed` to avoid accidental retries/dead-letter side effects.
- Test scope: dashboard integration tests for claim success/empty queue, completion success, completion conflict, and error banner rendering.

Refinement outcome for `UIADM-04` (2026-04-02):

- UX scope: add a "Plugin Administration" panel with inventory table (name, lifecycle state, declared capabilities, source manifest entry).
- Primary actions: load plugin, initialize plugin, execute plugin (diagnostic), unload plugin, and refresh plugin state snapshot.
- Visibility requirements: show per-plugin lifecycle (`Loaded`, `Initialized`, `Unloaded`) and normalized capability set.
- Error UX: map lifecycle errors (`duplicate`, `invalid state`, `not found`, `execution failed`, `execution panicked`) to actionable operator messages.
- Safety UX: require explicit confirmation before `unload` and before diagnostic `execute` on production-tagged contexts.
- API prerequisite: expose plugin registry read/write admin endpoints or GraphQL fields/mutations (list plugins, lifecycle actions, capability metadata).
- API compatibility note: plugin registry capabilities currently exist in `crates/plugins`, but API/dashboard layer does not yet expose plugin inventory/lifecycle operations.
- Test scope: dashboard integration tests for lifecycle happy path (load->init->execute->unload), invalid transition error rendering, and panic-safe execution reporting.

Refinement outcome for `UIADM-05` (2026-04-02):

- UX scope: add a "Plugin Policy" panel to manage granted capabilities per execution context (global default + optional context override).
- Primary actions: edit granted capabilities set, preview effective permissions for a plugin, and run a dry-run authorization check before execution.
- Policy model: represent granted set using existing capability taxonomy (`network`, `filesystem`, `secrets`, `runtime_hooks`) with explicit toggles.
- Decision UX: display required vs granted diff and explicit deny reason when one required capability is missing.
- Error UX: map `UnauthorizedCapability(...)` to a clear remediation hint (grant missing capability or choose another context).
- Safety UX: changes to contexts that grant `secrets` require confirmation and are logged in operator event feed.
- API prerequisite: expose policy persistence/read APIs and one authorization-check endpoint (or GraphQL mutation) returning allow/deny + missing capabilities.
- API compatibility note: runtime authorization exists through `execute_authorized`, but there is no API-managed policy store/context mapping yet.
- Test scope: dashboard integration tests for allow path, deny path with missing capability highlight, and secrets-grant confirmation flow.

Refinement outcome for `UIADM-06` (2026-04-02):

- UX scope: add a "Webhook Security Operations" panel focused on signature/replay/allowlist health and rejection diagnostics.
- Primary views: webhook counters (`received`, `accepted`, `rejected`, `duplicate`) plus recent rejection reasons timeline.
- Primary actions: filter by provider/repository, inspect last failed webhook summary, and copy remediation checklist for provider configuration.
- Security diagnostics: classify failures into missing/invalid signature, replay-window violation, and forbidden IP/repository/provider.
- Metric mapping: reuse existing SCM counters from runtime metrics endpoint as top-level KPI cards.
- API prerequisite: expose rejection-reason breakdown stream or query endpoint (current counters are aggregate-only and do not include per-reason history).
- API compatibility note: ingestion path already returns typed 400/401/403 responses, but dashboard does not yet receive structured per-event security diagnostics.
- UX guardrails: redact secrets/tokens from any displayed payload snippets and keep IP visibility limited to diagnostics context.
- Test scope: dashboard integration tests for counters rendering, rejection reason drill-down fallback states, and no-secret-leak assertions in UI logs.

Refinement outcome for `UIADM-07` (2026-04-02):

- UX scope: add an "Advanced Observability" panel combining runtime counters, live event stream, and troubleshooting filters in one operator workspace.
- Primary views: time-sliced counters dashboard + searchable event timeline (`kind`, `severity`, `message`, `job_id`, `build_id`, `worker_id`, `at`).
- Primary actions: filter by severity/kind/resource id, pin a time window preset, and export current view (JSON/CSV) for incident handoff.
- Metric mapping: use existing runtime metrics endpoint fields (queue reliability + SCM counters) as source of truth.
- Stream mapping: use existing SSE `/events` feed as near-real-time source, with polling fallback when stream is unavailable.
- Reliability UX: show stream health badge (`online`, `degraded`, `offline`) and explicit data freshness timestamp.
- API prerequisite: add optional server-side event query endpoint for historical pagination (SSE is best-effort and not a durable history source).
- API compatibility note: current event model carries enough identifiers for drill-down, but there is no persisted event history API yet.
- Test scope: dashboard integration tests for filter behavior, export payload schema, stream reconnect fallback, and stale-data indicator rendering.

Refinement outcome for `UIADM-08` (2026-04-02):

- UX scope: harden all administration panels with role-aware visibility, explicit destructive-action confirmations, and audit-trail surfacing.
- Access model: define baseline admin roles (`viewer`, `operator`, `admin`) with progressively broader action permissions.
- Guard behavior: unauthorized actions are hidden by default and optionally shown disabled with a "missing permission" hint in troubleshooting mode.
- Destructive flow: high-impact actions (plugin unload, failed completion simulation, security/policy updates) require typed confirmation and contextual warning copy.
- Audit UX: every admin mutation surfaces actor, action, target, and timestamp in an "Admin Activity" stream.
- Compliance UX: secrets/tokens remain masked in all views; copy actions never expose raw secret values.
- API prerequisite: expose role claims to frontend session context and provide audit-event ingestion/query APIs for admin actions.
- API compatibility note: current event stream includes operational events but does not yet persist actor-aware audit records for admin mutations.
- Accessibility requirement: confirmation dialogs and permission-state controls must be keyboard navigable and screen-reader labeled.
- Test scope: dashboard integration tests for role gating matrix, confirmation bypass prevention, audit entry emission, and accessibility smoke checks.

Refinement outcome for `UIADM-09` (2026-04-02):

- Test scope: establish an end-to-end admin UI suite covering SCM, workers, plugin administration, policy deny/allow flows, and observability panels.
- Priority journeys: webhook security save, polling config + manual tick, worker claim/complete simulation, plugin lifecycle actions, and policy deny feedback.
- Negative coverage: invalid forms, unauthorized role attempts, ownership conflict paths, missing capability deny, and stream disconnect fallback.
- Data strategy: deterministic fixture seed for jobs/builds/workers/plugins to keep snapshots and assertions stable.
- Environment strategy: run E2E against in-memory backend profile by default, with optional extended run against postgres+redis profile.
- Tooling expectation: use one browser automation stack with trace/video capture enabled on failures for triage.
- CI gate: admin E2E suite required on pull requests touching dashboard/admin/API contract surfaces.
- Reporting: publish flaky-test quarantine list and mean-time-to-fix KPI for admin-critical regressions.
- API prerequisite: expose test-only or seed endpoints/helpers to preload admin scenarios without brittle UI bootstrapping.
- Exit criteria: all UIADM panels have at least one happy path and one failure path automated in CI.

Refinement outcome for `UIADM-10` (2026-04-02):

- Documentation scope: publish an operations-oriented admin UI runbook with step-by-step procedures for SCM, workers, plugins, policy, and observability panels.
- Playbook structure: each playbook must include intent, prerequisites, exact UI path, expected signals, rollback path, and escalation contacts.
- Incident scenarios: include at least webhook rejection storm, polling stall, worker ownership conflicts, plugin execution panic, and policy deny regressions.
- Security chapter: document secret-handling rules, role boundaries, and audit-trail review process for sensitive actions.
- Verification chapter: provide post-action validation checklist using metrics/event panels and expected counter deltas.
- On-call chapter: define triage severity mapping and first-response checklist for admin UI alerts.
- Versioning policy: runbook updates are mandatory when UIADM workflows or API contracts change.
- Ownership model: assign runbook ownership to platform team with named reviewer from operations.
- Testability requirement: all runbook procedures must be executable in staging using seeded demo scenarios.
- Exit criteria: new operator can complete core admin tasks without API/CLI fallback using only documented runbook steps.

- [-] `UIADM-01` Add SCM webhook administration panel (repository/provider/secret/allowlist management).
- [-] `UIADM-02` Add SCM polling administration panel (enable/disable, intervals, branches, manual tick).
- [-] `UIADM-03` Add worker control panel (manual claim/complete simulation and ownership diagnostics).
- [-] `UIADM-04` Add plugin administration panel (manifest entries, lifecycle state, declared capabilities).
- [-] `UIADM-05` Add plugin policy panel (granted capabilities per execution context with deny feedback).
- [-] `UIADM-06` Add webhook/security operations panel (signature status, replay-window rejects, allowlist diagnostics).
- [-] `UIADM-07` Add advanced observability panel (SCM ingestion counters, filtering, export shortcuts).
- [-] `UIADM-08` Add admin UX hardening (role-based view guards, destructive-action confirmations, audit trail surfacing).
- [-] `UIADM-09` Add end-to-end UI integration tests for critical admin workflows.
- [-] `UIADM-10` Document administration playbooks and UI runbook for operations teams.

Definition of done:

- Every critical product administration workflow has a discoverable UI path.
- API-only operations needed for day-2 operations are reachable from dashboard/admin screens.
- Admin actions expose clear outcomes, errors, and operational telemetry in the interface.
- UI integration tests cover SCM, plugin, and worker administration happy/error paths.

### Epic 4: Redis-first production scheduler mode

### Epic 8: Mockup-to-real dashboard rollout

Goal: transform the validated multi-page mockup into the real dashboard with phased delivery aligned to currently exposed API functions.

Refinement decisions:

- Phase 1 strictly targets current API surface: `GET /health`, `POST /jobs`, `GET /jobs`, `POST /jobs/{id}/run`, `POST /builds/{id}/cancel`, `GET /builds`.
- Non-covered pages/actions stay visible as roadmap but non-blocking and explicitly tagged.
- UX decision traceability remains in `UX.md` and delivery tracking remains in this backlog.
- React implementation should keep existing role/event/snapshot conventions when possible.

- [x] `UXREAL-01` Add real-app multi-page shell (Pipelines, Overview, Workers, SCM Security, Plugins & Policy, Observability, Administration).
- [x] `UXREAL-02` Add explicit API coverage indicator by page (`full`, `partial`, `roadmap`) in real dashboard.
- [x] `UXREAL-03` Deliver Pipelines page on real API functions (`POST /jobs`, `GET /jobs`, `POST /jobs/{id}/run`, `GET /builds`, `POST /builds/{id}/cancel`).
- [x] `UXREAL-04` Deliver Overview page with metrics strictly derivable from `GET /health`, `GET /jobs`, `GET /builds`.
- [-] `UXREAL-05` Implement Workers page once worker runtime endpoints are finalized in public API surface.
- [-] `UXREAL-06` Implement SCM Security page once webhook-security admin endpoints are finalized.
- [ ] `UXREAL-07` Implement Plugins & Policy page once plugin registry/policy endpoints are finalized.
- [ ] `UXREAL-08` Implement Observability page once durable observability query contracts are finalized.
- [ ] `UXREAL-09` Implement Administration page once role/audit APIs are finalized.
- [x] `UXREAL-10` Add frontend integration tests for page navigation, API coverage gating, and Pipelines/Overview flows.
- [x] `UXREAL-11` Serve dashboard static assets dynamically from filesystem (`TARDIGRADE_WEB_ROOT`) to allow runtime web updates without rebuilding the Rust binary.

Definition of done:

- Real dashboard navigation matches validated 7-page IA from `UX.md`.
- Pages backed by current API are fully actionable and tested.
- Roadmap pages are clearly identified and do not expose misleading active controls.
- Backlog and UX logs remain synchronized for each delivered page iteration.

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

### Epic 6: Rust source maintainability (one object per file)

Goal: improve maintainability by enforcing one primary Rust object per `.rs` source file.

Refinement outcome for `REFAC-01` (2026-03-31):

- Scope target (at refinement date): all workspace Rust crates (`api`, `auth`, `core`, `executor`, `plugins`, `scheduler`, `server`, `storage`, `worker`, `xtask`).
- Object granularity: one primary object per file (`struct`, `enum`, `trait`, or type-focused module API surface).
- `impl` policy: keep `impl` blocks in the same file as their primary object.
- Module layout: split large `lib.rs`/`main.rs` internals into dedicated module files while keeping crate public APIs stable.
- API compatibility policy: strict backward compatibility for public APIs during the refactor.
- Migration strategy: incremental by crate, with compile + tests passing at each step.
- Execution order (at refinement date): `core` -> `storage` -> `scheduler` -> `plugins` -> `executor` -> `api` -> `server` -> `worker` -> `auth` -> `xtask`.
- Safety constraint: no behavior changes in this epic, only structure and readability improvements.
- Validation gate: run `cargo test --workspace` on every `REFAC-*` ticket.
- Naming convention: file names use `snake_case` and match their primary object (example: `definition.rs` for `PipelineDefinition` inside `pipeline/`).
- Domain folder rule: group sources by domain folders (examples: `job/`, `pipeline/`, `scm/`, `technology/`) instead of root-level prefix-based files.
- Module policy: keep crate root as re-export facade (`lib.rs`/`main.rs`) and avoid deep `mod.rs` nesting unless required by submodule grouping.
- Re-export policy: expose public API from crate root with stable `pub use`, keep internal wiring private by default.
- Test proximity rule: place tests as close as possible to modules using dedicated files in the same domain folder (for example `pipeline/tests.rs` or `pipeline/definition_tests.rs`).
- Test isolation rule: no inline `mod tests` inside production implementation files; keep test code in dedicated test files.
- Test layering rule: keep crate-root tests for cross-module behavior only; module/domain behavior should be validated in colocated tests.

- [x] `REFAC-01` Define and document one-object-per-file conventions (naming, domain folders, `mod.rs` usage, re-export policy, test placement).
- [x] `REFAC-02` Refactor `crates/core` into one-object-per-file module structure.
- [x] `REFAC-03` Refactor `crates/storage` and `crates/scheduler` into one-object-per-file module structure.
- [x] `REFAC-04` Refactor `crates/plugins` and `crates/executor` into one-object-per-file module structure.
- [x] `REFAC-05` Refactor `crates/api` and `crates/server` into one-object-per-file module structure.
- [x] `REFAC-06` Refactor `crates/worker`, `crates/auth`, and `crates/xtask` into one-object-per-file module structure (historical; `xtask` removed later in `INDUS-215`).
- [x] `REFAC-07` Validate workspace stability (`cargo test --workspace`) and update docs/contribution guidelines.

Definition of done:

- Rust source modules follow one-primary-object-per-file convention across targeted crates.
- Rust source layout is organized by domain folders instead of filename prefixes.
- Public crate APIs remain backward compatible unless explicitly approved.
- Formatting, linting, and tests pass after refactor.
- Tests are colocated near modules in dedicated test files, with root tests reserved for cross-module coverage.
- Contribution docs describe the convention for future changes.

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
3. Epic 7 (`UIADM-*`) for product administration IHM coverage.
4. Epic 6 (`REFAC-*`) for Rust source maintainability refactor.
5. Reliability follow-ups (`REL-*`) as hardening milestones.
6. Epic 0 (`INDUS-*`) hardening follow-ups.
7. Epic 5 (`CLOUD-*`) cloud/container delivery track (deferred).
8. Epic 8 (`UXREAL-*`) mockup-to-real-dashboard rollout.
