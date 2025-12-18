.PHONY: ci ci-verbose clippy tarpaulin sloc sloc-strict clean_tmp test

# 静默模式
ci:
	@mkdir -p .tmp
	@TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo tarpaulin --config tarpaulin.toml 2>&1 | tail -n 30
	@cargo clippy --all-targets --all-features -q -- -D warnings
	@mkdir -p .tmp
	@TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo run -q -- check src 

# 正常模式
ci-verbose: tarpaulin clippy sloc-strict

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

tarpaulin:
	mkdir -p .tmp
	TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo tarpaulin --config tarpaulin.toml

sloc:
	mkdir -p .tmp
	TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo run -- check src

sloc-strict:
	mkdir -p .tmp
	TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo run -- check --strict src

clean_tmp:
	rm -rf .tmp

test:
	cargo test

fmt:
	cargo fmt --all