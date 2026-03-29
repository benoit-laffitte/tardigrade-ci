# Shared variables and command helpers for repository automation.

NO_PROXY_ENV := env -u https_proxy -u http_proxy -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u NO_PROXY -u no_proxy -u PXY_FAB_FONC

CARGO ?= cargo
DOCKER ?= docker
TRIVY ?= trivy

SERVER_DOCKERFILE ?= Dockerfile.server
WORKER_DOCKERFILE ?= Dockerfile.worker
SERVER_IMAGE ?= tardigrade-server:local
WORKER_IMAGE ?= tardigrade-worker:local
