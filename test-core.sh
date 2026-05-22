#!/bin/bash
# Run core tests sequentially to avoid state pollution
cd "$(dirname "$0")/core"
cargo test -- --test-threads=1
