# Frontend/dashboard targets powered by cargo xtask aliases.

.PHONY: dashboard-install dashboard-lint dashboard-build

dashboard-install: ## Install dashboard dependencies via xtask
	$(NO_PROXY_ENV) $(CARGO) dashboard-install

dashboard-lint: dashboard-install ## Lint dashboard frontend via xtask
	$(NO_PROXY_ENV) $(CARGO) dashboard-lint

dashboard-build: dashboard-install ## Build dashboard frontend assets via xtask
	$(NO_PROXY_ENV) $(CARGO) dashboard-build
