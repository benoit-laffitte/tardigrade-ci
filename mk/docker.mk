# Container image build, smoke-check, and scan targets.

.PHONY: docker-build-server docker-build-worker docker-build docker-smoke docker-scan

docker-build-server: ## Build tardigrade server image
	$(NO_PROXY_ENV) $(DOCKER) build -f $(SERVER_DOCKERFILE) -t $(SERVER_IMAGE) .

docker-build-worker: ## Build tardigrade worker image
	$(NO_PROXY_ENV) $(DOCKER) build -f $(WORKER_DOCKERFILE) -t $(WORKER_IMAGE) .

docker-build: docker-build-server docker-build-worker ## Build all Docker images

docker-smoke: docker-build ## Execute binaries inside built images as a runtime sanity check
	@$(DOCKER) run --rm --entrypoint /usr/local/bin/tardigrade-server $(SERVER_IMAGE) --help >/dev/null 2>&1 || { \
		echo "tardigrade-server binary failed to execute inside image"; \
		exit 1; \
	}
	@$(DOCKER) run --rm --entrypoint /usr/local/bin/tardigrade-worker $(WORKER_IMAGE) --help >/dev/null 2>&1 || { \
		echo "tardigrade-worker binary failed to execute inside image"; \
		exit 1; \
	}

docker-scan: docker-build ## Scan built images (high/critical) with Trivy if available
	@if command -v "$(TRIVY)" >/dev/null 2>&1; then \
		$(NO_PROXY_ENV) $(TRIVY) image --severity HIGH,CRITICAL --exit-code 1 "$(SERVER_IMAGE)"; \
		$(NO_PROXY_ENV) $(TRIVY) image --severity HIGH,CRITICAL --exit-code 1 "$(WORKER_IMAGE)"; \
	else \
		echo "trivy not found; skipping docker-scan"; \
	fi
