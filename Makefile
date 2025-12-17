
.PHONY: ci 
ci: clippy tarpaulin

.PHONY: clippy
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: tarpaulin
tarpaulin:
	@mkdir -p .tmp
	TEMP="$$(pwd -W)/.tmp" TMP="$$(pwd -W)/.tmp" cargo tarpaulin --config tarpaulin.toml


.PHONY: clean_tmp
clean_tmp:
	rm -rf .tmp
