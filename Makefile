SHELL := /bin/bash
.DEFAULT_GOAL := help

include mk/common.mk
include mk/rust.mk
include mk/dashboard.mk
include mk/docker.mk
include mk/ci.mk
