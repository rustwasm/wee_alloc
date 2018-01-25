#!/usr/bin/env bash

set -eux

cd $(dirname $0)

cd ./wee_alloc
cargo check
cargo check                         --target wasm32-unknown-unknown
cargo check --features size_classes
cargo check --features size_classes --target wasm32-unknown-unknown
cd -

cd ./test
cargo check
cargo check --features size_classes
cd -

cd ./example
cargo check                         --target wasm32-unknown-unknown
cargo check --features size_classes --target wasm32-unknown-unknown
cd -
