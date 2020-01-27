.PHONY: all run fmt

all:
	cargo build

run:
	./target/debug/yith $*

fmt:
	find src -name '*\.rs' -exec rustfmt {} \;

