# Rust-centric targets.

.PHONY: bootstrap fmt-check clippy dead-code arch-guard arch-guard-test arch-import-guard lint test-fast test-all test build-rust build-rust-release-images package-platform-zips worker-transport-bench

bootstrap: ## Prefetch Rust dependencies for local development
	$(NO_PROXY_ENV) $(CARGO) fetch

fmt-check: ## Verify Rust formatting
	$(NO_PROXY_ENV) $(CARGO) fmt --all -- --check

clippy: ## Run clippy on all Rust targets
	$(NO_PROXY_ENV) $(CARGO) clippy --workspace --all-targets -- -D warnings

dead-code: ## Run dead-code focused lint pass on all Rust targets
	$(NO_PROXY_ENV) $(CARGO) clippy --workspace --all-targets -- -W dead_code

arch-guard: ## Enforce pragmatic hexagonal internal dependency policy
	bash ./scripts/check-hexagonal-deps.sh

arch-guard-test: ## Run architecture guard regression scenarios
	bash ./scripts/test-hexagonal-deps-guard.sh

arch-import-guard: ## Enforce adapter import policy outside composition root
	bash ./scripts/check-hexagonal-imports.sh

lint: fmt-check clippy arch-guard arch-guard-test arch-import-guard ## Run Rust lint pipeline

test-fast: ## Run Rust unit tests only (lib + bins)
	$(NO_PROXY_ENV) $(CARGO) test --workspace --lib --bins

test-all: ## Run full Rust workspace test suite
	$(NO_PROXY_ENV) $(CARGO) test --workspace

test: test-all ## Alias for full test suite

build-rust: ## Build Rust workspace artifacts
	$(NO_PROXY_ENV) $(CARGO) build --workspace

build-rust-release-images: ## Build release binaries used by runtime-only Docker images
	$(NO_PROXY_ENV) $(CARGO) build --release -p tardigrade-server -p tardigrade-worker

package-platform-zips: dashboard-build ## Build and package zip archives for mac/windows/linux
	./scripts/package-platform-zips.sh

worker-transport-bench: ## Run local worker transport benchmark (HTTP/1 vs HTTP/2)
	$(NO_PROXY_ENV) $(CARGO) run -p tardigrade-worker --features transport-bench --bin transport_bench -- --iterations 200
