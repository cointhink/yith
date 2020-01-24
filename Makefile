.PHONY: all

all:
	cargo build

run:
	./target/debug/yith $*
