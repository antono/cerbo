#!/usr/bin/env bash
# Run the cerbo-core test suite.
cd "$(dirname "$0")/core"
cargo test
