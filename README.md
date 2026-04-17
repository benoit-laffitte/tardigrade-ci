# Tardigrade CI (Bootstrap)

This workspace is a starting point for building an enterprise-grade open-source CI/CD service in Rust.

## Architecture actuelle

- crates/server: executable entry point and HTTP server bootstrap.
- crates/api: GraphQL schema/router and API state.
- crates/core: domain model for jobs, pipeline runs, and statuses.
- crates/scheduler: queueing and scheduling abstractions.
- crates/storage: persistence abstractions with in-memory and PostgreSQL implementations.
- crates/plugins: plugin contract and registry.
- crates/auth: authentication primitives.
- crates/worker: dedicated external worker process for build execution.

## Implemente actuellement

- Workspace and crate structure.
- GraphQL control-plane endpoint at `/graphql`.
- Initial domain and subsystem skeletons.
- API now delegates job/build lifecycle to a service layer backed by the storage crate.
- Build transitions are exposed on the domain model to enforce status invariants.
- Scheduler now uses claim/ack/requeue semantics to model cluster-safe work distribution.
- Worker API endpoints allow external agents d execution to claim and complete builds.
- Example configuration at config/example.toml.

## Plugin runtime status

- Plugins support explicit lifecycle transitions: load, init, execute, unload.
- Discovery supports filesystem manifest loading via `plugins.toml`.
- Capability permissions are enforced through explicit authorized execution checks.
- Plugin execution failures are typed (`ExecutionFailed`) and panic-safe (`ExecutionPanicked`).
- One plugin failure does not block execution of other healthy plugins.

## Cible d architecture

- Multi-language CI via plugin-driven runtime adapters.
- Horizontally scalable control plane (stateless API instances).
- Distributed queue + agent d execution pool for execution throughput.
- Cluster-resilient operation with durable state and object storage.
- Configurable behavior by environment, organization, and project.

Current DSL runtime model is technology-agnostic: each pipeline step defines its own container image and command, so one pipeline can mix Rust, Python, Java, Node, or any stack available as an OCI image.

Current state is a bootstrap baseline with pluggable adapters: in-memory/file/Redis/PostgreSQL scheduler backends, plus PostgreSQL storage for durable jobs/builds.

## Run

env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server

Then open http://127.0.0.1:8080/health

Modern UI console:

Open http://127.0.0.1:8080/

Frontend dashboard subproject (React + TypeScript + Vite + Apollo + Oxlint):

- cd dashboard
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC npm install
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC npm run dev

Centralized workflow with Make (Node + Rust via one command surface):

- make dashboard-install
- make dashboard-lint
- make dashboard-build
- make dashboard-dev

Dashboard Make targets run npm from `dashboard/` with public registry (`https://registry.npmjs.org/`) and bypass user proxy settings.

Build dashboard assets served by Rust server:

- cd dashboard
- env -u https_proxy -u http_proxy -u PXY_FAB_FONC npm run build

The Vite build outputs to `target/public` (`index.html`, `app.js`, `styles.css`) and the Axum server serves them dynamically at runtime.
If `target/public` is missing, dashboard routes return runtime errors until `make dashboard-build` is executed.

Run server in dev mode from TOML config file:

env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server -- config/runtime-dev.toml

Run server in prod mode from TOML config file:

env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server -- config/runtime-prod.toml

Run a dedicated external agent d execution from the same TOML file base:

env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-worker -- config/runtime-dev.toml

Runtime mode is read from config file under `[runtime]`:

- `mode = "dev"`: scheduler uses Redis when configured, otherwise in-memory fallback.
- `mode = "prod"`: scheduler defaults to Redis and fails fast when Redis is missing.
- `queue.backend` overrides runtime defaults to one of in-memory/file/redis/postgres.

`queue.file_path` is used only when `queue.backend=file`.

Migration notes for scheduler backend selection:

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
- `make dead-code` (Rust dead-code focused clippy pass)
- `make arch-guard` (hexagonal dependency policy check)
- `make arch-guard-test` (hexagonal dependency guard regression scenarios)
- `make test-fast` (Rust unit tests only)
- `make test-all` (full Rust workspace tests)
- `make dashboard-install` (frontend dependencies via npm)
- `make dashboard-lint` (frontend lint via npm)
- `make dashboard-build` (frontend build via npm)
- `cd dashboard && npm run e2e` (Playwright admin E2E suite)
- `make build` (Rust + dashboard build)
- `make agent d execution-transport-bench` (local mock HTTP/1 vs HTTP/2 agent d execution transport benchmark; add `--real-server-url` in direct command for real server scenarios)
- `make package-platform-zips` (create release zip per platform: mac/windows/linux)
- `make ci` (local CI-equivalent aggregate)

Platform zip packaging details:

- Each archive includes `bin/`, `config/`, `docs/`, `dashboard/`, `README.md`, and `LICENSE.txt`.
- Dashboard assets are exported from `target/public` to a top-level `dashboard/` folder in each zip.
- Launchers in `bin/` (`start-server.sh`, `start-server.ps1`, `start-server.cmd`) start server with `config/runtime-prod.toml` by default.

Current note:

- `make docker-build` and `make docker-scan` are intentionally unavailable for now while the Docker/cloud delivery scope is being redesigned.

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
- POST /webhooks/scm

GraphQL snapshot example (single request for dashboard panels):

curl -X POST http://127.0.0.1:8080/graphql \
	-H 'content-type: application/json' \
	-d '{"query":"query { dashboard_snapshot { jobs { id name } builds { id status } agents d execution { id status active_builds } metrics { reclaimed_total retry_requeued_total ownership_conflicts_total dead_letter_total } dead_letter_builds { id status } } }"}'

Pipeline DSL reference and examples:

- [docs/pipeline-schema.md](docs/pipeline-schema.md)
- [docs/pipeline-recipes.md](docs/pipeline-recipes.md)
- [docs/admin-ui-runbook.md](docs/admin-ui-runbook.md)
- [docs/technology-profile-onboarding.md](docs/technology-profile-onboarding.md)
- [docs/plugin-authoring-permissions.md](docs/plugin-authoring-permissions.md)

Invalid pipeline behavior:

- GraphQL `create_job` returns an error with `extensions.code=invalid_pipeline` and optional `extensions.details`.
- Blank `pipeline_yaml` is rejected as a bad request.

## Cloud/Container Track Status

Cloud and container delivery is deferred for a later planning cycle (see Epic 5 `CLOUD-*` in [BACKLOG.md](BACKLOG.md)).

Current repository snapshot does not include Dockerfiles, Kubernetes manifests, or docker-compose descriptors as first-class tracked artifacts.

Local helper scripts under `scripts/` remain available for developer workflows where applicable.

## Backlog

Roadmap items are now decomposed into an actionable backlog in:

- [BACKLOG.md](BACKLOG.md)
