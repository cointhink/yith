.PHONY: all run fmt

all:
	cargo build

run: all
	./target/debug/yith $*

fmt:
	find src -name '*\.rs' -exec rustfmt {} \;

