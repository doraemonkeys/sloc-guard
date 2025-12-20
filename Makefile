.PHONY: ci verify clippy tarpaulin sloc sloc-strict clean_tmp test fmt

SHELL := /bin/bash
.SHELLFLAGS := -o pipefail -c

# 跨平台临时目录设置
ifeq ($(OS),Windows_NT)
    # Windows (Git Bash / MSYS2)
    TMP_DIR := $(shell pwd -W)/.tmp
else
    # Linux / macOS
    TMP_DIR := $(CURDIR)/.tmp
endif

TEMP_ENV := TEMP="$(TMP_DIR)" TMP="$(TMP_DIR)"

# 静默模式
ci:
	@mkdir -p .tmp
	@set -o pipefail && $(TEMP_ENV) cargo tarpaulin --config tarpaulin.toml 2>&1 | tail -n 30
	@cargo clippy --all-targets --all-features -q -- -D warnings
	@$(TEMP_ENV) cargo run -q -- check src

# 正常模式
verify: tarpaulin clippy sloc

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

tarpaulin:
	mkdir -p .tmp
	$(TEMP_ENV) cargo tarpaulin --config tarpaulin.toml

sloc:
	mkdir -p .tmp
	$(TEMP_ENV) cargo run -- check src

sloc-strict:
	mkdir -p .tmp
	$(TEMP_ENV) cargo run -- check --strict src

clean_tmp:
	rm -rf .tmp

test:
	cargo test

fmt:
	cargo fmt --all