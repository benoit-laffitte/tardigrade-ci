# Container image build and scan targets.

.PHONY: docker-build-server docker-build-worker docker-build docker-scan

docker-build-server: ## Build tardigrade server image
	$(NO_PROXY_ENV) $(DOCKER) build -f $(SERVER_DOCKERFILE) -t $(SERVER_IMAGE) .

docker-build-worker: ## Build tardigrade worker image
	$(NO_PROXY_ENV) $(DOCKER) build -f $(WORKER_DOCKERFILE) -t $(WORKER_IMAGE) .

docker-build: docker-build-server docker-build-worker ## Build all Docker images

docker-scan: docker-build ## Scan built images (high/critical) with Trivy if available
	@if command -v "$(TRIVY)" >/dev/null 2>&1; then \
		$(NO_PROXY_ENV) $(TRIVY) image --severity HIGH,CRITICAL --exit-code 1 "$(SERVER_IMAGE)"; \
		$(NO_PROXY_ENV) $(TRIVY) image --severity HIGH,CRITICAL --exit-code 1 "$(WORKER_IMAGE)"; \
	else \
		echo "trivy not found; skipping docker-scan"; \
	fi
