.PHONY: help all clean test test-doc build release lint fmt check-fmt typecheck \
	markdownlint nixie validate-xml json-schema test-workflow-contracts \
	spelling

APP ?= tei-rapporteur
CARGO ?= cargo
BUILD_JOBS ?=
CLIPPY_FLAGS ?= --all-targets --all-features -- -D warnings
RUSTDOC_FLAGS ?= --cfg docsrs -D warnings
MDLINT ?= markdownlint-cli2
NIXIE ?= nixie
WHITAKER ?= whitaker
FIXTURES_DIR ?= target/fixtures
TIME_BIN ?= /usr/bin/time
TIME_ARGS ?= -v
UV ?= uv
UV_ENV = UV_CACHE_DIR=.uv-cache UV_TOOL_DIR=.uv-tools
TYPOS_CONFIG_BUILDER_VERSION ?= v0.1.1
TYPOS_CONFIG_BUILDER = $(UV_ENV) $(UV) tool run --python 3.14 --from \
	"git+https://github.com/leynos/typos-config-builder.git@$(TYPOS_CONFIG_BUILDER_VERSION)" \
	typos-config-builder

MDLINT_BIN := $(shell command -v $(MDLINT) 2>/dev/null || true)

build: ## Build all workspace crates in debug mode
	$(CARGO) build --workspace $(BUILD_JOBS)

release: ## Build all workspace crates in release mode
	$(CARGO) build --workspace --release $(BUILD_JOBS)

all: build spelling ## Build the workspace and enforce spelling

clean: ## Remove build artefacts
	$(CARGO) clean
	rm -rf .uv-cache .uv-tools

test: test-doc ## Run tests with warnings treated as errors
	RUSTFLAGS="-D warnings" $(CARGO) nextest run --workspace --all-targets --all-features $(BUILD_JOBS)

test-doc: ## Run documentation tests (nextest does not execute them)
	# --all-features: `tei_py::test_support` compiles only under `cfg(test)`
	# or `feature = "test-support"`, and rustdoc sets neither for doctests.
	RUSTFLAGS="-D warnings" $(CARGO) test --doc --workspace --all-features $(BUILD_JOBS)

lint: ## Build documentation and run Clippy and the Whitaker Dylint suite with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --workspace --no-deps
	$(CARGO) clippy $(CLIPPY_FLAGS)
	RUSTFLAGS="-D warnings" $(WHITAKER) --all -- --all-targets --all-features

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	mdformat-all

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check

test-workflow-contracts: ## Validate the mutation-testing caller contract
	uv run --no-project --with 'pytest>=8' --with 'pyyaml>=6' pytest tests/workflow_contracts -q

typecheck: ## Typecheck all workspace crates
	RUSTFLAGS="-D warnings" $(CARGO) check --workspace --all-targets --all-features $(BUILD_JOBS)

markdownlint: spelling ## Lint Markdown files
	@if [ -n "$(MDLINT_BIN)" ]; then \
		"$(MDLINT_BIN)" "**/*.md" "#.uv-cache" "#.uv-tools"; \
	elif [ -x "$(HOME)/.bun/bin/markdownlint-cli2" ]; then \
		"$(HOME)/.bun/bin/markdownlint-cli2" "**/*.md" "#.uv-cache" "#.uv-tools"; \
	else \
		echo "error: markdownlint-cli2 not found; install it or set MDLINT=/path/to/markdownlint-cli2"; \
		exit 1; \
	fi

spelling: ## Enforce en-GB-oxendict in tracked text
	$(TYPOS_CONFIG_BUILDER) gate --repository . --scope all

nixie: ## Validate Mermaid diagrams
	$(NIXIE) --no-sandbox --max-concurrency 1

validate-xml: ## Validate XML fixtures against the Relax NG schema using jing
	@command -v jing >/dev/null 2>&1 || { echo "error: jing not found; install with 'apt-get install jing-trang' or 'brew install jing'"; exit 1; }
	$(CARGO) run --package tei-xml --bin generate-fixtures $(BUILD_JOBS) -- $(FIXTURES_DIR)
	@xml_files="$$(find $(FIXTURES_DIR) -maxdepth 1 -name '*.xml' 2>/dev/null)"; \
	if [ -z "$$xml_files" ]; then \
		echo "error: no XML fixtures found in $(FIXTURES_DIR)"; exit 1; \
	fi; \
	for xml in $$xml_files; do \
		echo "Validating $$xml…"; \
		jing $(FIXTURES_DIR)/tei-episodic-profile.rng "$$xml" || exit 1; \
	done
	@echo "All fixtures validated successfully"

json-schema: ## Generate JSON Schema snapshots for TeiDocument
	$(CARGO) run --package tei-serde --bin generate-json-schema $(BUILD_JOBS)

bench: ## Run performance benchmarks
	$(CARGO) bench --package tei-xml --features streaming $(BUILD_JOBS)

bench-memory: ## Measure peak memory usage during parsing (GNU time required; see TIME_BIN)
	$(CARGO) build --release --package tei-xml --features streaming --example bench_memory $(BUILD_JOBS)
	@echo "=== Streaming parser memory usage ==="
	@$(TIME_BIN) $(TIME_ARGS) ./target/release/examples/bench_memory streaming 2>&1 | grep -E "(Maximum resident|Command)" || true
	@echo "=== Full document parser memory usage ==="
	@$(TIME_BIN) $(TIME_ARGS) ./target/release/examples/bench_memory full 2>&1 | grep -E "(Maximum resident|Command)" || true
	@echo "Note: Requires GNU time for -v flag (Linux default). On macOS: brew install gnu-time && TIME_BIN=gtime make bench-memory"

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
