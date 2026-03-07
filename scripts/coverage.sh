#!/usr/bin/env bash

set -euo pipefail

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov is not installed."
    echo "Install with: cargo install cargo-llvm-cov"
    exit 1
fi

mode="${1:-summary}"

case "$mode" in
summary)
    ENV_TEST=1 cargo llvm-cov --workspace --all-features
    ;;
html)
    ENV_TEST=1 cargo llvm-cov --workspace --all-features --html
    echo "HTML report: target/llvm-cov/html/index.html"
    ;;
lcov)
    mkdir -p coverage
    ENV_TEST=1 cargo llvm-cov --workspace --all-features --lcov --output-path coverage/lcov.info
    echo "LCOV report: coverage/lcov.info"
    ;;
*)
    echo "Usage: scripts/coverage.sh [summary|html|lcov]"
    exit 2
    ;;
esac
