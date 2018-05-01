#!/usr/bin/env bash

set -eux

cd $(dirname $0)

cd ./wee_alloc
cargo check
cargo check                         --target wasm32-unknown-unknown
cargo check                         --target i686-pc-windows-gnu
cargo check --features size_classes
cargo check --features size_classes --target wasm32-unknown-unknown
cargo check --features size_classes --target i686-pc-windows-gnu
cargo check --no-default-features --features "static_array_backend"
cargo check --no-default-features --features "static_array_backend size_classes"
cd -

cd ./test
cargo check
cargo check --features size_classes
cd -

cd ./example
cargo check                         --target wasm32-unknown-unknown
cargo check --features size_classes --target wasm32-unknown-unknown
cd -
