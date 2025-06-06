TARGET_DIR=.target
.PHONY: all run fmt

all:
	cargo build --target-dir ${TARGET_DIR}

run: all
	${TARGET_DIR}/debug/yith $*

format:
	find src -name '*\.rs' -exec rustfmt {} \;

test:
	RUST_BACKTRACE=1 cargo test -- --nocapture

