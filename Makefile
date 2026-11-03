.PHONY: build
SHELL := /bin/bash
# MAKEFLAGS += --no-print-directory

# target-cpu=x86-64-v3"

run:
	RUSTFLAGS="-C target-cpu=native" cargo run --release

build:
	RUSTFLAGS="-C target-cpu=native" cargo build --release
