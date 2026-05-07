#!/usr/bin/env bash

set -euxo pipefail

cargo fmt --check
cargo clippy -- -D warnings
taplo format --check Cargo.toml
nixfmt --check ./**/*.nix
