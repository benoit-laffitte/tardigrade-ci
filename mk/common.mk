# Shared variables and command helpers for repository automation.

NO_PROXY_ENV := env -u https_proxy -u http_proxy -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u NO_PROXY -u no_proxy

CARGO ?= cargo
DOCKER ?= docker
TRIVY ?= trivy

