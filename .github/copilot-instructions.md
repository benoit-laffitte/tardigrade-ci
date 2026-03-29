## Tardigrade CI - Copilot Instructions

### Project Context
- Tardigrade CI is a Rust workspace for an open-source CI/CD platform.
- Main crates and roles:
	- `crates/server`: Axum server entry point, static web console.
	- `crates/api`: HTTP routes and API state.
	- `crates/core`: domain entities (`JobDefinition`, `BuildRecord`, `JobStatus`).
	- `crates/storage`: storage trait with in-memory implementation.
	- `crates/scheduler`: scheduling trait with in-memory queue.
	- `crates/executor`: worker execution abstraction/simulation.
	- `crates/plugins`: plugin contract and registry.
	- `crates/auth`: authentication primitives.

### Current API Surface
- `GET /health`
- `POST /jobs`
- `GET /jobs`
- `POST /jobs/{id}/run`
- `POST /builds/{id}/cancel`
- `GET /builds`

### Build, Test, and Run
- Always use proxy-safe commands in this repository:
	- `env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo test --workspace`
	- `env -u https_proxy -u http_proxy -u PXY_FAB_FONC cargo run -p tardigrade-server`
- Respect local Cargo registry overrides from `.cargo/config.toml` (workspace uses `cargo-public`).

### Coding Expectations
- Keep changes minimal, focused, and aligned with the current modular architecture.
- Preserve existing public APIs unless a change request requires otherwise.
- Add or update tests when behavior changes.
- Keep documentation and examples in sync with implementation changes.
- Code must be correctly commented:
	- Add clear comments for non-obvious logic, decisions, edge cases, and invariants.
	- Prefer intent-focused comments over line-by-line narration.
	- Avoid redundant comments that repeat self-explanatory code.

### Collaboration Guidelines
- Work through tasks systematically and report progress concisely.
- Follow Rust and Axum best practices for error handling, async code, and type safety.
- Keep this instructions file updated over time with major project directions and any development rules the team formally adopts.
