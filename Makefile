SHELL := /bin/bash
.DEFAULT_GOAL := help

include mk/common.mk
include mk/rust.mk
include mk/dashboard.mk
# Docker targets are intentionally optional while cloud/container track is deferred.
-include mk/docker.mk
include mk/ci.mk
