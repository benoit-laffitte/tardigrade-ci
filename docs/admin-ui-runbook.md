# Tardigrade CI Admin UI Runbook

## Scope

This runbook defines step-by-step operational procedures for the Tardigrade admin UI:

- SCM Webhook Security
- SCM Polling
- Worker Control
- Plugin Administration
- Plugin Policy
- Webhook Security Operations
- Advanced Observability
- Admin Activity

The goal is to let operators handle day-2 tasks without API/CLI fallback.

## Ownership and Review

- Runbook owner: Platform Team
- Operations reviewer: On-call Operations Lead
- Update policy: mandatory update whenever UIADM workflows or API contracts change

## Roles and Boundaries

- `viewer`: read-only troubleshooting views, no mutations
- `operator`: operational actions (run/claim/complete), no sensitive policy/security writes
- `admin`: full access including sensitive changes (webhook security, plugin unload, policy grants)

Sensitive actions must be executed as `admin` and are recorded in the `Admin Activity` panel.

## Secret Handling Rules

- Keep webhook secrets masked unless actively validating value entry.
- Never paste secrets into free-text logs.
- Do not capture screenshots with visible secret values.
- Do not copy raw secret values into tickets/chat; use masked form (`****`).

## Environment Prerequisites

- Server running: `env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server`
- Dashboard reachable: `http://127.0.0.1:8080/`
- Staging data seeded with representative jobs/builds/workers/plugins before drill exercises.

## Playbook 1: SCM Webhook Security

Intent:
Configure or rotate repository webhook security settings.

UI path:
`Tardigrade CI Console -> SCM Webhook Security`

Procedure:
1. Select `admin` role.
2. Fill `Repository URL`, `Provider`, and `Secret`.
3. Optionally set `IP allowlist` entries.
4. Click `Enregistrer` and confirm overwrite if prompted.

Expected signals:
- Banner: `Configuration webhook enregistree.`
- Log line in `Journal de bord` confirming repository/provider.

Rollback:
- Re-apply previous secret/allowlist values.
- If unknown previous value, coordinate with SCM owner and rotate again.

Escalation:
- Platform Team if 4xx/5xx persists.
- Security Team if suspicious webhook activity exists.

## Playbook 2: SCM Polling Configuration and Manual Tick

Intent:
Enable/disable polling and validate trigger path.

UI path:
`Tardigrade CI Console -> SCM Polling`

Procedure:
1. Select `admin` role.
2. Fill repository/provider.
3. Set `Interval` (> 0) and optional branches.
4. Toggle `Enabled` and click `Enregistrer`.
5. Click `Tick manuel` to validate immediate behavior.

Expected signals:
- Banner: `Configuration polling enregistree.` or `Polling desactive.`
- Tick summary with `polled_repositories` and `enqueued_builds`.

Rollback:
- Restore previous interval and branch set, or disable polling.

Escalation:
- Platform Team if tick endpoint repeatedly fails.

## Playbook 3: Worker Control

Intent:
Diagnose worker execution and perform controlled claim/complete simulation.

UI path:
`Tardigrade CI Console -> Worker Control`

Procedure:
1. Select `operator` or `admin` role.
2. Fill `Worker ID`.
3. Click `Claim next build`.
4. If build claimed, verify `Build ID` auto-filled.
5. Choose completion status.
6. For `failed`, confirm destructive prompt.
7. Click `Complete build`.

Expected signals:
- Claim success or empty-queue message.
- Completion success message with build status transition.

Rollback:
- Re-run claim/complete using `success` if failure was accidental and build is still recoverable.
- If dead-letter reached, follow incident process.

Escalation:
- Platform Team for repeated ownership conflicts or invalid transitions.

## Playbook 4: Plugin Administration Lifecycle

Intent:
Operate plugin lifecycle safely for diagnostics.

UI path:
`Tardigrade CI Console -> Plugin Administration`

Procedure:
1. Select role:
- `admin` for `Load` and `Unload`
- `operator` or `admin` for `Init` and `Execute`
2. Enter plugin name.
3. Run lifecycle actions in order: `Load -> Init -> Execute -> Unload`.

Expected signals:
- Status banners after each action.
- Plugin row state transitions in inventory list.

Rollback:
- Re-run `Load` then `Init` for a plugin unintentionally unloaded.
- Keep plugin unloaded if panic/instability is observed.

Escalation:
- Platform Team if execution panics repeatedly.

## Playbook 5: Plugin Policy and Authorization Dry-Run

Intent:
Manage granted capability sets and validate policy decisions before execution.

UI path:
`Tardigrade CI Console -> Plugin Policy`

Procedure:
1. Select `admin` role.
2. Set context (`global` or environment-specific context).
3. Toggle granted capabilities.
4. Confirm when granting `secrets`.
5. Click `Save policy`.
6. Enter plugin name in Plugin Administration and click `Dry-run authorize`.

Expected signals:
- `Policy enregistree (...)` banner.
- Dry-run `Allow`/`Deny` summary with missing capability list.

Rollback:
- Remove over-granted capabilities and save again.

Escalation:
- Security Team for unexpected secrets grants.

## Playbook 6: Webhook Security Operations Diagnostics

Intent:
Triage webhook ingestion rejections quickly.

UI path:
`Tardigrade CI Console -> Webhook Security Operations`

Procedure:
1. Open panel and click `Refresh diagnostics`.
2. Filter by provider and/or repository.
3. Inspect counters and rejection timeline entries.

Expected signals:
- Counters: `Received`, `Accepted`, `Rejected`, `Duplicate`.
- Rejection entries with `reason_code`, provider, repository, timestamp.

Rollback:
- N/A (diagnostic view). Use corrective actions in relevant playbooks.

Escalation:
- Platform Team when rejection ratio spikes unexpectedly.

## Playbook 7: Advanced Observability and Export

Intent:
Build incident evidence package and narrow noisy event streams.

UI path:
`Tardigrade CI Console -> Advanced Observability`

Procedure:
1. Apply severity/kind/resource/time window filters.
2. Validate event freshness timestamp.
3. Export filtered view as JSON or CSV.

Expected signals:
- Filtered event list with resource IDs.
- Freshness timestamp updates as new events arrive.

Rollback:
- Reset filters to broad defaults (`all`, empty text, 15 minutes).

Escalation:
- Platform Team if stream appears stale while systems are active.

## Incident Scenarios

### Scenario A: Webhook Rejection Storm

Symptoms:
- `Rejected` counter rising rapidly.
- Rejection timeline dominated by `invalid_webhook_signature` or `webhook_forbidden`.

Immediate actions:
1. Filter by provider/repository.
2. Verify webhook secret and allowlist settings.
3. Confirm SCM side signature/token setup.

Expected counter trend:
- Rejected stabilizes, Accepted resumes growth.

### Scenario B: Polling Stall

Symptoms:
- Manual tick reports `polled_repositories=0` unexpectedly.

Immediate actions:
1. Validate polling config enabled state and interval.
2. Trigger manual tick.
3. Confirm matching jobs exist for repository URL.

Expected counter trend:
- `scm_polling_ticks_total` and enqueue counters increase after correction.

### Scenario C: Worker Ownership Conflicts

Symptoms:
- Completion returns conflict, ownership metrics increase.

Immediate actions:
1. Check worker ID and current claim owner context.
2. Retry claim before completion.
3. Avoid parallel manual completions for same build.

Expected counter trend:
- Ownership conflicts stop increasing after coordination.

### Scenario D: Plugin Execution Panic

Symptoms:
- Plugin execute returns panic-contained error.

Immediate actions:
1. Stop repeated execute actions.
2. Unload affected plugin.
3. Capture observability export and admin activity trail.

Expected counter trend:
- No repeated panic events once plugin is unloaded.

### Scenario E: Policy Deny Regression

Symptoms:
- Dry-run denies for previously allowed plugin/context.

Immediate actions:
1. Load effective policy.
2. Compare required vs granted capabilities.
3. Apply minimal missing grants and re-run dry-run.

Expected counter trend:
- Dry-run shifts from `Deny` to `Allow` without over-granting.

## Post-Action Verification Checklist

After any admin mutation, verify:
1. User-facing banner shows expected success outcome.
2. `Journal de bord` contains corresponding operation line.
3. `Admin Activity` contains actor/action/target/timestamp.
4. Relevant counters change in expected direction.
5. Observability feed confirms follow-up events with fresh timestamp.

## On-Call Severity Mapping

- `SEV-1`: platform-wide trigger failure, queue blocked, or major security exposure.
- `SEV-2`: partial trigger/worker degradation with customer-visible delay.
- `SEV-3`: isolated repo/plugin issue with known workaround.
- `SEV-4`: cosmetic UI issue with no operational risk.

## On-Call First Response Checklist

1. Identify impacted scope (provider, repository, worker, plugin, context).
2. Capture current counters and filtered observability export.
3. Capture relevant `Admin Activity` entries.
4. Apply smallest reversible mitigation from playbooks.
5. Re-validate counters and event freshness.
6. Escalate with evidence package when unresolved in 15 minutes (SEV-1/2).

## Testability and Staging Drill

Each procedure must be rehearsed in staging with seeded demo scenarios.

Minimum cadence:
- Weekly: SCM webhook/polling and worker control drills.
- Bi-weekly: plugin lifecycle/policy drills.
- Monthly: incident simulation for all five scenarios.

Evidence to retain:
- Observability export (JSON/CSV)
- Screenshot of key panels
- Short post-drill note with pass/fail and follow-ups
