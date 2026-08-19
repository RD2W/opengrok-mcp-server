# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

CARGO   := cargo
DOCKER  := docker
IMAGE   := opengrok-mcp:latest

GIT_HASH   := $(shell git rev-parse --short=8 HEAD 2>/dev/null || echo unknown)
BUILD_DATE := $(shell date -u +'%Y-%m-%dT%H:%M:%SZ')

.PHONY: help

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
	awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ── Local dev ────────────────────────────────────────────────────────────────

.PHONY: build test fmt lint check fix

build: ## Release build (binary)
	$(CARGO) build --workspace --locked --release

test: ## Run all tests
	$(CARGO) test --workspace --locked -- --test-threads=1

fmt: ## Format all sources
	$(CARGO) fmt --all

lint: ## Clippy with deny-warnings
	$(CARGO) clippy --workspace --all-targets -- -D warnings

check: fmt lint ## Format check + clippy (same as CI)

fix: ## Auto-fix clippy suggestions
	$(CARGO) clippy --fix --workspace --all-targets --allow-dirty --allow-staged

ci: fmt lint test ## Full CI simulation (fmt, clippy, tests)

# ── Docker ───────────────────────────────────────────────────────────────────

.PHONY: docker-build docker-run docker-up docker-down docker-logs

docker-build: ## Build Docker image
	$(DOCKER) build \
		--build-arg GIT_HASH=$(GIT_HASH) \
		--build-arg BUILD_DATE=$(BUILD_DATE) \
		-t $(IMAGE) \
		.

docker-run: ## Run container using docker-compose.local.yml
	$(DOCKER) compose -f docker-compose.local.yml up -d

docker-up: docker-run ## Alias: start container

docker-down: ## Stop and remove container
	$(DOCKER) compose -f docker-compose.local.yml down

docker-logs: ## Follow container logs
	$(DOCKER) compose -f docker-compose.local.yml logs -f

# ── Cleanup ──────────────────────────────────────────────────────────────────

.PHONY: clean

clean: ## Remove build artifacts
	$(CARGO) clean