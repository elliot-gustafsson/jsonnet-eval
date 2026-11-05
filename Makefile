# jsonnet-eval
SHELL := /bin/bash

# target-cpu=native is set in .cargo/config.toml and applies to every profile.

.PHONY: run build build-profiling build-release-debug test perf perf-report

run:
	cargo run --release

build:
	cargo build --release

# Release codegen + line tables, for perf and flamegraphs.
build-profiling:
	cargo build --profile profiling

# Release codegen + debug info + debug assertions + overflow checks.
build-release-debug:
	cargo build --profile release-debug

test:
	cargo test

# DWARF call graphs resolve inlined frames without needing frame pointers.
# Pass program arguments with ARGS="...".
perf: build-profiling
	perf record --call-graph dwarf -o perf.data ./target/profiling/jsonnet-eval $(ARGS)

perf-report:
	perf report -i perf.data
