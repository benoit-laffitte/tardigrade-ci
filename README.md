# Tardigrade CI (Bootstrap)

This workspace is a starting point for building an enterprise-grade open-source CI/CD service in Rust.

## Current architecture

- crates/server: executable entry point and HTTP server bootstrap.
- crates/api: HTTP routes and API state.
- crates/core: domain model for jobs, pipeline runs, and statuses.
- crates/scheduler: queueing and scheduling abstractions.
- crates/executor: worker execution logic abstraction.
- crates/storage: persistence abstractions and in-memory implementation.
- crates/plugins: plugin contract and registry.
- crates/auth: authentication primitives.

## Implemented now

- Workspace and crate structure.
- Health check endpoint at GET /health.
- Job lifecycle endpoints: create, list, run, cancel.
- Initial domain and subsystem skeletons.
- API now delegates job/build lifecycle to a service layer backed by the storage crate.
- Build transitions are exposed on the domain model to enforce status invariants.
- Scheduler now uses claim/ack/requeue semantics to model cluster-safe work distribution.
- Worker API endpoints allow external workers to claim and complete builds.
- Example configuration at config/example.toml.

## Architecture target

- Multi-language CI via plugin-driven runtime adapters.
- Horizontally scalable control plane (stateless API instances).
- Distributed queue + worker pool for execution throughput.
- Cluster-resilient operation with durable state and object storage.
- Configurable behavior by environment, organization, and project.

Current state is a bootstrap baseline with in-memory adapters that preserve API contracts while preparing the control-plane/data-plane split.

## Run

env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Then open http://127.0.0.1:8080/health

Modern UI console:

Open http://127.0.0.1:8080/

Frontend dashboard subproject (React + TypeScript + Vite + Apollo + Oxlint):

- cd crates/server/dashboard
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC npm install
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC npm run dev

Centralized workflow with Cargo (Node + Rust via one tool):

- env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo dashboard-install
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo dashboard-lint
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo dashboard-build
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo dashboard-dev

The Cargo aliases call `crates/xtask`, which runs npm with public registry (`https://registry.npmjs.org/`) and bypasses user proxy settings.

Build dashboard assets served by Rust server:

- cd crates/server/dashboard
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC npm run build

The Vite build outputs to `crates/server/static` (`index.html`, `app.js`, `styles.css`) and the Axum server embeds these files at compile time.

Run server with durable queue state on disk:

TARDIGRADE_QUEUE_FILE=.tardigrade/queue-state.json \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Run server with Redis-backed queue:

TARDIGRADE_REDIS_URL=redis://127.0.0.1:6379 \
TARDIGRADE_REDIS_PREFIX=tardigrade \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Run server with PostgreSQL storage + Redis queue:

TARDIGRADE_DATABASE_URL=postgres://tardigrade:tardigrade@127.0.0.1:5432/tardigrade \
TARDIGRADE_REDIS_URL=redis://127.0.0.1:6379 \
TARDIGRADE_REDIS_PREFIX=tardigrade \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Run a dedicated external worker:

TARDIGRADE_SERVER_URL=http://127.0.0.1:8080 \
TARDIGRADE_WORKER_ID=worker-a \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-worker

Cloud-friendly runtime env vars:

- TARDIGRADE_BIND_ADDR (default: 0.0.0.0:8080)
- TARDIGRADE_SERVICE_NAME (default: tardigrade-ci)
- TARDIGRADE_EMBEDDED_WORKER (default: true)
- TARDIGRADE_QUEUE_FILE (optional durable queue state file)
- TARDIGRADE_DATABASE_URL (optional PostgreSQL URL for jobs/builds persistence)
- TARDIGRADE_REDIS_URL (optional Redis URL for distributed queue backend)
- TARDIGRADE_REDIS_PREFIX (optional Redis key prefix, default: tardigrade)
- TARDIGRADE_SERVER_URL (worker -> controller URL)
- TARDIGRADE_WORKER_ID (worker identity)
- TARDIGRADE_WORKER_POLL_MS (worker polling interval)

## Test

env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo test --workspace

Coverage (line threshold gate, default 75%):

./scripts/coverage.sh

Coverage with explicit threshold:

./scripts/coverage.sh 80

PostgreSQL persistence integration test (optional):

TARDIGRADE_TEST_DATABASE_URL=postgres://tardigrade:tardigrade@127.0.0.1:5432/tardigrade \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo test --workspace postgres_storage_persists_jobs_and_builds_across_state_recreation

## API (Step 1)

- POST /graphql (GraphQL endpoint)
- GET /graphql (GraphQL Playground UI)
- GET /health
- GET /live
- GET /ready
- GET /events (SSE stream for live dashboard updates)
- POST /jobs
- GET /jobs
- POST /jobs/{id}/run
- POST /builds/{id}/cancel
- GET /builds
- GET /workers
- POST /workers/{worker_id}/claim
- POST /workers/{worker_id}/builds/{id}/complete

GraphQL snapshot example (single request for dashboard panels):

curl -X POST http://127.0.0.1:8080/graphql \
	-H 'content-type: application/json' \
	-d '{"query":"query { dashboard_snapshot { jobs { id name } builds { id status } workers { id status active_builds } metrics { reclaimed_total retry_requeued_total ownership_conflicts_total dead_letter_total } dead_letter_builds { id status } } }"}'

Worker claim example:

curl -X POST http://127.0.0.1:8080/workers/worker-a/claim

Worker completion example:

curl -X POST http://127.0.0.1:8080/workers/worker-a/builds/<build-id>/complete \
	-H 'content-type: application/json' \
	-d '{"status":"success","log_line":"Build completed by external worker"}'

Create a job:

curl -X POST http://127.0.0.1:8080/jobs \
	-H 'content-type: application/json' \
	-d '{"name":"build-api","repository_url":"https://example.com/api.git","pipeline_path":"pipelines/api.yml"}'

List jobs:

curl http://127.0.0.1:8080/jobs

## Cloud Ready Baseline

Build images:

- docker build -f Dockerfile.server -t tardigrade-server:latest .
- docker build -f Dockerfile.worker -t tardigrade-worker:latest .

Kubernetes manifests:

- deploy/k8s/tardigrade-server.yaml
- deploy/k8s/tardigrade-worker.yaml

Example apply sequence:

- kubectl apply -f deploy/k8s/tardigrade-server.yaml
- kubectl apply -f deploy/k8s/tardigrade-worker.yaml

## Docker Compose Cluster (local)

Start controller + workers with one command:

- env -u https_proxy -u http_proxy -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u NO_PROXY -u no_proxy -u PXY_FAB_FONC docker compose up --build -d
- ./scripts/dev-up.sh

The compose stack uses PostgreSQL for jobs/builds storage and Redis as the queue backend for distributed worker coordination.

The compose stack runs an init service (`tardigrade-init-data`) before the controller to ensure `/data` is writable by the runtime user (uid 10001).

All helper scripts in `scripts/` run without proxy variables by default.

- Use `--with-proxy` to keep proxy variables.
- Use `--without-proxy` to force no-proxy mode.

Scale workers horizontally:

- docker compose up -d --scale tardigrade-worker=3

Check runtime:

- curl http://127.0.0.1:8080/ready
- curl http://127.0.0.1:8080/workers
- ./scripts/dev-smoke.sh

Stop cluster:

- docker compose down
- ./scripts/dev-down.sh

## Roadmap (next)

1. Add pipeline DSL (YAML) parser and validator.
2. Persist jobs and builds in SQLite/PostgreSQL.
3. Add webhook triggers and SCM polling.
4. Replace file-backed queue with a managed broker backend (Redis Streams, NATS JetStream, or RabbitMQ).
5. Add plugin loading and permissions model.

## Backlog (Queue Reliability)

- [x] Redis-backed queue scheduler (distributed claim/ack/requeue).
- [x] Worker ownership check on build completion (409 on mismatch).
- [x] Stale lease reclaim with configurable timeout (`TARDIGRADE_WORKER_LEASE_TIMEOUT_SECS`).
- [x] Runtime metrics API (`GET /metrics`) with:
	- `reclaimed_total`
	- `retry_requeued_total`
	- `ownership_conflicts_total`
	- `dead_letter_total`
- [x] Dashboard panel displaying runtime metrics in real time.
- [x] Real-time event stream (`GET /events`) wired to dashboard live feed.
- [ ] Retry policy refinement (configurable caps per job profile).
- [x] Dead-letter flow for builds exceeding max retries (`GET /dead-letter-builds`) visible in dashboard.
- [ ] Metrics persistence/export (Prometheus/OpenTelemetry).
