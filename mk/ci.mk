# Aggregated project-level targets.

.PHONY: build ci help

build: build-rust dashboard-build ## Build project deliverables (Rust + dashboard)

ci: lint test-all e2e-runtime dashboard-install build  ## Run local CI-equivalent pipeline

help: ## Show available Make targets
	@echo "Tardigrade CI Make targets"
	@echo
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-22s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
