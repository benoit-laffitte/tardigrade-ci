# Tardigrade CI (Bootstrap)

This workspace is a starting point for building an enterprise-grade open-source CI/CD service in Rust.

## Current architecture

- crates/server: executable entry point and HTTP server bootstrap.
- crates/api: HTTP routes and API state.
- crates/core: domain model for jobs, pipeline runs, and statuses.
- crates/scheduler: queueing and scheduling abstractions.
- crates/executor: worker execution logic abstraction.
- crates/storage: persistence abstractions with in-memory and PostgreSQL implementations.
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

Current DSL runtime model is technology-agnostic: each pipeline step defines its own container image and command, so one pipeline can mix Rust, Python, Java, Node, or any stack available as an OCI image.

Current state is a bootstrap baseline with pluggable adapters: in-memory for local bootstrap, plus PostgreSQL storage and Redis queue backends for distributed deployments.

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

Run server in dev mode from config file (Redis optional, in-memory fallback):

TARDIGRADE_CONFIG_FILE=config/runtime-dev.toml \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Run server with Redis-backed queue:

TARDIGRADE_REDIS_URL=redis://127.0.0.1:6379 \
TARDIGRADE_REDIS_PREFIX=tardigrade \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Run server with PostgreSQL storage + Redis queue:

TARDIGRADE_CONFIG_FILE=config/runtime-prod.toml \
TARDIGRADE_DATABASE_URL=postgres://tardigrade:tardigrade@127.0.0.1:5432/tardigrade \
TARDIGRADE_REDIS_URL=redis://127.0.0.1:6379 \
TARDIGRADE_REDIS_PREFIX=tardigrade \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Run a dedicated external worker:

TARDIGRADE_SERVER_URL=http://127.0.0.1:8080 \
TARDIGRADE_WORKER_ID=worker-a \
env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-worker

Cloud-friendly runtime env vars:

- TARDIGRADE_CONFIG_FILE (optional config file path, default: config/example.toml)
- TARDIGRADE_BIND_ADDR (default: 0.0.0.0:8080)
- TARDIGRADE_SERVICE_NAME (default: tardigrade-ci)
- TARDIGRADE_EMBEDDED_WORKER (default: true)
- TARDIGRADE_DATABASE_URL (optional PostgreSQL URL for jobs/builds persistence)
- TARDIGRADE_REDIS_URL (optional Redis URL for distributed queue backend)
- TARDIGRADE_REDIS_PREFIX (optional Redis key prefix, default: tardigrade)
- TARDIGRADE_SERVER_URL (worker -> controller URL)
- TARDIGRADE_WORKER_ID (worker identity)
- TARDIGRADE_WORKER_POLL_MS (worker polling interval)

Runtime mode is read from config file under `[runtime]`:

- `mode = "dev"`: scheduler uses Redis when configured, otherwise in-memory fallback.
- `mode = "prod"`: server fails fast unless both PostgreSQL and Redis are configured.

`TARDIGRADE_QUEUE_FILE` is deprecated and ignored.

Migration notes for Redis-first scheduler rollout:

- [docs/scheduler-migration.md](docs/scheduler-migration.md)

Multi-technology pipeline recipes (Rust, Python, Java, mixed stacks):

- [docs/pipeline-recipes.md](docs/pipeline-recipes.md)

## Test

env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo test --workspace

## Make Commands

Unified automation entrypoints from repository root:

- `make help` (list available targets)
- `make bootstrap` (Rust dependencies prefetch)
- `make lint` (Rust fmt + clippy)
- `make test-fast` (Rust unit tests only)
- `make test-all` (full Rust workspace tests)
- `make dashboard-install` (frontend dependencies via `xtask`)
- `make dashboard-lint` (frontend lint via `xtask`)
- `make dashboard-build` (frontend build via `xtask`)
- `make build` (Rust + dashboard build)
- `make docker-build` (server + worker images)
- `make docker-scan` (Trivy image scan when available)
- `make ci` (local CI-equivalent aggregate)

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

Create a job with inline pipeline YAML validation (optional `pipeline_yaml`):

curl -X POST http://127.0.0.1:8080/jobs \
	-H 'content-type: application/json' \
	-d '{"name":"build-api-inline","repository_url":"https://example.com/api.git","pipeline_path":"pipelines/api.yml","pipeline_yaml":"version: 1\nstages:\n  - name: build\n    steps:\n      - name: cargo-build\n        image: \"rust:1.94\"\n        command:\n          - cargo\n          - build"}'

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

## Backlog

Roadmap items are now decomposed into an actionable backlog in:

- [BACKLOG.md](BACKLOG.md)
