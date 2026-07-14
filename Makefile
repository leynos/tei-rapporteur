.PHONY: help all clean test build release lint fmt check-fmt typecheck \
	markdownlint nixie validate-xml json-schema test-workflow-contracts \
	spelling spelling-config spelling-config-write spelling-phrase-check \
	spelling-helper-test

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
RUFF_VERSION ?= 0.15.12
PATHSPEC_VERSION ?= 1.1.1
TYPOS_VERSION ?= 1.48.0
TYPOS_CONFIG_BUILDER_COMMIT := d6da92f02240a79a945c835f69bdd08a888da1d0
TYPOS_CONFIG_BUILDER_SOURCE := git+https://github.com/leynos/typos-config-builder.git@$(TYPOS_CONFIG_BUILDER_COMMIT)
TYPOS_CONFIG_BUILDER := $(UV_ENV) $(UV) tool run --python 3.14 \
	--from "$(TYPOS_CONFIG_BUILDER_SOURCE)" typos-config-builder
SPELLING_PY_SRCS := \
	scripts/typos_rollout_check.py scripts/tests/test_typos_rollout_check.py
SPELLING_PY_TESTS := scripts/tests/test_typos_rollout_check.py
SPELLING_COVERAGE_ARGS := --cov=typos_rollout_check --cov-fail-under=90
SPELLING_HELPER_PYTEST = PYTHONPATH=scripts $(UV_ENV) $(UV) run --no-project \
	--python 3.14 --with pathspec==$(PATHSPEC_VERSION) --with pytest==9.0.2 \
	--with pytest-cov==7.0.0 python -m pytest

MDLINT_BIN := $(shell command -v $(MDLINT) 2>/dev/null || true)

build: ## Build all workspace crates in debug mode
	$(CARGO) build --workspace $(BUILD_JOBS)

release: ## Build all workspace crates in release mode
	$(CARGO) build --workspace --release $(BUILD_JOBS)

all: build spelling ## Build the workspace and enforce spelling

clean: ## Remove build artefacts
	$(CARGO) clean
	rm -rf .uv-cache .uv-tools

test: ## Run tests with warnings treated as errors
	RUSTFLAGS="-D warnings" $(CARGO) nextest run --workspace --all-targets --all-features $(BUILD_JOBS)

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

spelling: spelling-phrase-check ## Enforce en-GB-oxendict in tracked text
	@git ls-files -z | xargs -0 -r env $(UV_ENV) \
		$(UV) tool run typos@$(TYPOS_VERSION) --config typos.toml --force-exclude --hidden

spelling-phrase-check: spelling-config ## Reject prohibited spelling phrases
	@PYTHONPATH=scripts $(UV_ENV) $(UV) run --no-project --python 3.14 scripts/typos_rollout_check.py --repository .

spelling-config: spelling-helper-test ## Verify generated spelling configuration
	@git ls-files --error-unmatch typos.toml >/dev/null
	@$(TYPOS_CONFIG_BUILDER) --repository . --check

spelling-config-write: spelling-helper-test ## Generate spelling configuration
	@$(TYPOS_CONFIG_BUILDER) --repository .

spelling-helper-test: ## Validate the shared spelling-policy integration
	@$(UV_ENV) $(UV) tool run ruff@$(RUFF_VERSION) format --isolated --target-version py313 --check $(SPELLING_PY_SRCS)
	@$(UV_ENV) $(UV) tool run ruff@$(RUFF_VERSION) check --isolated --target-version py313 $(SPELLING_PY_SRCS)
	@$(SPELLING_HELPER_PYTEST) $(SPELLING_PY_TESTS) -c /dev/null --rootdir=. -p no:cacheprovider $(SPELLING_COVERAGE_ARGS)

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
