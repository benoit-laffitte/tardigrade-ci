# Frontend/dashboard targets powered directly by npm from dashboard/.

.PHONY: dashboard-install dashboard-lint dashboard-build dashboard-dev

NPM ?= npm
DASHBOARD_DIR := dashboard
# Keep npm independent from user machine proxy/Nexus overrides for reproducible installs/builds.
NPM_ENV := npm_config_userconfig=/dev/null npm_config_registry=https://registry.npmjs.org/ npm_config_proxy= npm_config_https_proxy= npm_config_strict_ssl=false

dashboard-install: ## Install dashboard dependencies
	cd $(DASHBOARD_DIR) && $(NO_PROXY_ENV) $(NPM_ENV) $(NPM) install

dashboard-lint: dashboard-install ## Lint dashboard frontend
	cd $(DASHBOARD_DIR) && $(NO_PROXY_ENV) $(NPM_ENV) $(NPM) run lint

dashboard-build: dashboard-install ## Build dashboard frontend assets
	cd $(DASHBOARD_DIR) && $(NO_PROXY_ENV) $(NPM_ENV) $(NPM) run build

dashboard-dev: dashboard-install ## Run dashboard dev server
	cd $(DASHBOARD_DIR) && $(NO_PROXY_ENV) $(NPM_ENV) $(NPM) run dev
