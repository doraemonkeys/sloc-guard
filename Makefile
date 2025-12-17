.PHONY: ci ci-verbose clippy tarpaulin sloc sloc-strict clean_tmp

# 静默模式
ci:
	@cargo clippy --all-targets --all-features -q -- -D warnings
	@mkdir -p .tmp
	@TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo tarpaulin --config tarpaulin.toml 2>&1 | tail -n 30
	@mkdir -p .tmp
	@TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo run -q -- check --strict src 

# 详细模式
ci-verbose: clippy tarpaulin sloc-strict

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
