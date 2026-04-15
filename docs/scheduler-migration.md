# Scheduler Migration Guide (Explicit Backend Selection)

This guide covers migration from legacy runtime fallback behavior to explicit scheduler backend selection.

## Summary

- Supported scheduler backends: `in-memory`, `file`, `redis`, `postgres`.
- Set `TARDIGRADE_SCHEDULER_BACKEND` to force backend selection.
- Default behavior when backend is not set:
  - Prod mode: Redis scheduler (requires `TARDIGRADE_REDIS_URL`).
  - Dev mode: Redis when configured, otherwise in-memory.
- `TARDIGRADE_QUEUE_FILE` is used only by the `file` backend.
- PostgreSQL scheduler uses `TARDIGRADE_SCHEDULER_DATABASE_URL` or falls back to `TARDIGRADE_DATABASE_URL`.

## What changed

### Before

- Runtime mode selected the scheduler through a strict mode-specific fallback chain.
- File-backed scheduler path was not directly selectable.

### After

- Runtime mode is selected from config file:
  - `TARDIGRADE_CONFIG_FILE=config/runtime-dev.toml`
  - `TARDIGRADE_CONFIG_FILE=config/runtime-prod.toml`
- Scheduler backend can be selected explicitly:
  - `TARDIGRADE_SCHEDULER_BACKEND=in-memory|file|redis|postgres`
- Scheduler-specific parameters:
  - `TARDIGRADE_QUEUE_FILE` for `file`
  - `TARDIGRADE_REDIS_URL` and optional `TARDIGRADE_REDIS_PREFIX` for `redis`
  - `TARDIGRADE_SCHEDULER_DATABASE_URL` and optional `TARDIGRADE_SCHEDULER_NAMESPACE` for `postgres`

## Migration checklist

1. Set runtime mode in config file:
   - Dev: `[runtime] mode = "dev"`
   - Prod: `[runtime] mode = "prod"`
2. Select scheduler backend explicitly when needed:
  - `TARDIGRADE_SCHEDULER_BACKEND=redis` for distributed queue
  - `TARDIGRADE_SCHEDULER_BACKEND=postgres` for PostgreSQL scheduler
  - `TARDIGRADE_SCHEDULER_BACKEND=file` for local file-backed state
3. Ensure required backend variables are set for the selected backend.
4. Validate startup logs show runtime mode and selected scheduler backend.
5. Run smoke checks:
   - `GET /live`
   - `GET /ready`
   - `GET /workers`
6. Execute one end-to-end job run and verify claim/complete flow.

## Compose migration

Use production runtime config in controller service:

- `TARDIGRADE_CONFIG_FILE=/app/config/runtime-prod.toml`

Keep:

- `TARDIGRADE_DATABASE_URL=postgres://...`
- `TARDIGRADE_SCHEDULER_BACKEND=redis` (or `postgres`)
- Backend-specific connection variable (`TARDIGRADE_REDIS_URL` or `TARDIGRADE_SCHEDULER_DATABASE_URL`)

## Kubernetes migration

Set in server deployment:

- `TARDIGRADE_CONFIG_FILE=/app/config/runtime-prod.toml`
- `TARDIGRADE_DATABASE_URL=postgres://...`
- `TARDIGRADE_SCHEDULER_BACKEND=redis` (or `postgres`)
- For `redis`: `TARDIGRADE_REDIS_URL=redis://...` and optional `TARDIGRADE_REDIS_PREFIX=tardigrade`
- For `postgres`: `TARDIGRADE_SCHEDULER_DATABASE_URL=postgres://...` and optional `TARDIGRADE_SCHEDULER_NAMESPACE=tardigrade`

## Rollback strategy

If startup fails after migration:

1. Switch to dev runtime mode temporarily:
   - `TARDIGRADE_CONFIG_FILE=/app/config/runtime-dev.toml`
2. Set `TARDIGRADE_SCHEDULER_BACKEND=in-memory` for emergency restore.
3. Keep PostgreSQL enabled where possible to preserve persistent job/build data.
4. Fix selected scheduler backend connectivity/configuration.
5. Switch back to prod runtime mode.

Note: dev fallback is for recovery and local workflows, not for long-running clustered production.

## Acceptance criteria

- Selected scheduler backend starts with required variables configured.
- Production default remains Redis unless explicitly overridden.
- Operational runbook includes health checks and rollback steps.
