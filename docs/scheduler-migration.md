# Scheduler Migration Guide (Redis-First)

This guide covers migration from legacy queue-file usage to Redis-first scheduler behavior.

## Summary

- Production mode (`[runtime] mode = "prod"`) now requires:
  - `TARDIGRADE_DATABASE_URL`
  - `TARDIGRADE_REDIS_URL`
- Dev mode (`[runtime] mode = "dev"`) uses:
  - Redis scheduler when `TARDIGRADE_REDIS_URL` is set
  - In-memory scheduler otherwise
- `TARDIGRADE_QUEUE_FILE` is deprecated and ignored.
- Deprecation target for queue-file migration: `2026-09-30`.

## What changed

### Before

- Runtime behavior was selected mostly from environment variables.
- Scheduler fallback could include file-backed queue state.

### After

- Runtime mode is selected from config file:
  - `TARDIGRADE_CONFIG_FILE=config/runtime-dev.toml`
  - `TARDIGRADE_CONFIG_FILE=config/runtime-prod.toml`
- Scheduler behavior is strict in production:
  - Missing Redis in prod => startup failure
- File-backed queue is no longer part of runtime fallback path.

## Migration checklist

1. Set runtime mode in config file:
   - Dev: `[runtime] mode = "dev"`
   - Prod: `[runtime] mode = "prod"`
2. Ensure production environment variables are set:
   - `TARDIGRADE_DATABASE_URL`
   - `TARDIGRADE_REDIS_URL`
   - Optional: `TARDIGRADE_REDIS_PREFIX`
3. Remove `TARDIGRADE_QUEUE_FILE` from all deployments.
4. Validate startup logs show runtime mode and Redis scheduler selection.
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
- `TARDIGRADE_REDIS_URL=redis://...`

## Kubernetes migration

Set in server deployment:

- `TARDIGRADE_CONFIG_FILE=/app/config/runtime-prod.toml`
- `TARDIGRADE_DATABASE_URL=postgres://...`
- `TARDIGRADE_REDIS_URL=redis://...`
- Optional: `TARDIGRADE_REDIS_PREFIX=tardigrade`

## Rollback strategy

If startup fails after migration:

1. Switch to dev runtime mode temporarily:
   - `TARDIGRADE_CONFIG_FILE=/app/config/runtime-dev.toml`
2. Unset `TARDIGRADE_REDIS_URL` to use in-memory scheduler for emergency restore.
3. Keep PostgreSQL enabled where possible to preserve persistent job/build data.
4. Fix Redis connectivity/configuration.
5. Switch back to prod runtime mode.

Note: dev fallback is for recovery and local workflows, not for long-running clustered production.

## Acceptance criteria

- Production starts only with PostgreSQL + Redis configured.
- No deployment relies on `TARDIGRADE_QUEUE_FILE`.
- Operational runbook includes health checks and rollback steps.
