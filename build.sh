#!/usr/bin/env bash

set -eux

cd $(dirname $0)

cargo readme -r wee_alloc -t "$(pwd)/README.tpl" > README.md

cd ./wee_alloc
cargo build
cargo build --features size_classes
cd -

cd ./test
cargo build --release
cargo build --release --features size_classes
cd -

cd ./example

cargo build --release                         --target wasm32-unknown-unknown

wasm-gc ../target/wasm32-unknown-unknown/release/wee_alloc_example.wasm \
        ../target/wasm32-unknown-unknown/release/wee_alloc_example.gc.wasm
wasm-opt -Oz \
         ../target/wasm32-unknown-unknown/release/wee_alloc_example.gc.wasm \
         -o ../target/wasm32-unknown-unknown/release/wee_alloc_example.gc.opt.wasm

cargo build --release --features size_classes --target wasm32-unknown-unknown

wasm-gc ../target/wasm32-unknown-unknown/release/wee_alloc_example.wasm \
        ../target/wasm32-unknown-unknown/release/wee_alloc_example.size_classes.gc.wasm
wasm-opt -Oz \
         ../target/wasm32-unknown-unknown/release/wee_alloc_example.size_classes.gc.wasm \
         -o ../target/wasm32-unknown-unknown/release/wee_alloc_example.size_classes.gc.opt.wasm

wc -c ../target/wasm32-unknown-unknown/release/*.gc.opt.wasm

cd -
