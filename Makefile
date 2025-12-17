.PHONY: ci ci-verbose clippy tarpaulin sloc clean_tmp

# 静默模式
ci:
	@cargo clippy --all-targets --all-features -q -- -D warnings
	@mkdir -p .tmp
	@TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo tarpaulin --config tarpaulin.toml 2>&1 | tail -n 30
	@cargo run -q -- check src

# 详细模式
ci-verbose:
	cargo clippy --all-targets --all-features -- -D warnings
	mkdir -p .tmp
	TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo tarpaulin --config tarpaulin.toml
	cargo run -- check src

clippy:
	@cargo clippy --all-targets --all-features -q -- -D warnings

tarpaulin:
	@mkdir -p .tmp
	@TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo tarpaulin --config tarpaulin.toml 2>&1 | tail -n 30

sloc:
	@cargo run -q -- check src

clean_tmp:
	rm -rf .tmp
