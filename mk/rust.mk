# Rust-centric targets.

.PHONY: bootstrap fmt-check clippy lint test-fast test-all test build-rust

bootstrap: ## Prefetch Rust dependencies for local development
	$(NO_PROXY_ENV) $(CARGO) fetch

fmt-check: ## Verify Rust formatting
	$(NO_PROXY_ENV) $(CARGO) fmt --all -- --check

clippy: ## Run clippy on all Rust targets
	$(NO_PROXY_ENV) $(CARGO) clippy --workspace --all-targets -- -D warnings

lint: fmt-check clippy ## Run Rust lint pipeline

test-fast: ## Run Rust unit tests only (lib + bins)
	$(NO_PROXY_ENV) $(CARGO) test --workspace --lib --bins

test-all: ## Run full Rust workspace test suite
	$(NO_PROXY_ENV) $(CARGO) test --workspace

test: test-all ## Alias for full test suite

build-rust: ## Build Rust workspace artifacts
	$(NO_PROXY_ENV) $(CARGO) build --workspace
