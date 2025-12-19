.PHONY: help all clean test build release lint fmt check-fmt typecheck markdownlint nixie validate-xml

APP ?= tei-rapporteur
CARGO ?= cargo
BUILD_JOBS ?=
CLIPPY_FLAGS ?= --all-targets --all-features -- -D warnings
RUSTDOC_FLAGS ?= --cfg docsrs -D warnings
MDLINT ?= markdownlint-cli2
NIXIE ?= nixie
FIXTURES_DIR ?= target/fixtures

MDLINT_BIN := $(shell command -v $(MDLINT) 2>/dev/null || true)

build: ## Build all workspace crates in debug mode
	$(CARGO) build --workspace $(BUILD_JOBS)

release: ## Build all workspace crates in release mode
	$(CARGO) build --workspace --release $(BUILD_JOBS)

all: build ## Default target builds the workspace in debug mode

clean: ## Remove build artifacts
	$(CARGO) clean

test: ## Run tests with warnings treated as errors
	RUSTFLAGS="-D warnings" $(CARGO) nextest run --workspace --all-targets --all-features $(BUILD_JOBS)

lint: ## Build documentation and run Clippy with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --workspace --no-deps
	$(CARGO) clippy $(CLIPPY_FLAGS)

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	mdformat-all

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check

typecheck: ## Typecheck all workspace crates
	RUSTFLAGS="-D warnings" $(CARGO) check --workspace --all-targets --all-features $(BUILD_JOBS)

markdownlint: ## Lint Markdown files
	@if [ -n "$(MDLINT_BIN)" ]; then \
		"$(MDLINT_BIN)" '**/*.md'; \
	elif [ -x /root/.bun/bin/markdownlint-cli2 ]; then \
		/root/.bun/bin/markdownlint-cli2 '**/*.md'; \
	else \
		echo "error: markdownlint-cli2 not found; install it or set MDLINT=/path/to/markdownlint-cli2"; \
		exit 1; \
	fi

nixie: ## Validate Mermaid diagrams
	$(NIXIE) --no-sandbox

validate-xml: ## Validate XML fixtures against the Relax NG schema using jing
	@command -v jing >/dev/null 2>&1 || { echo "error: jing not found; install with 'apt-get install jing-trang' or 'brew install jing'"; exit 1; }
	$(CARGO) run --package tei-xml --bin generate-fixtures $(BUILD_JOBS) -- $(FIXTURES_DIR)
	@xml_files="$$(find $(FIXTURES_DIR) -maxdepth 1 -name '*.xml' 2>/dev/null)"; \
	if [ -z "$$xml_files" ]; then \
		echo "error: no XML fixtures found in $(FIXTURES_DIR)"; exit 1; \
	fi; \
	for xml in $$xml_files; do \
		echo "Validating $$xml..."; \
		jing $(FIXTURES_DIR)/tei-episodic-profile.rng "$$xml" || exit 1; \
	done
	@echo "All fixtures validated successfully"

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
