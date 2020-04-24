.PHONY: all run fmt

all:
	cargo build --target-dir .build

run: all
	./target/debug/yith $*

format:
	find src -name '*\.rs' -exec rustfmt {} \;

test:
	RUST_BACKTRACE=1 cargo test -- --nocapture

