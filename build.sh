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

set +x

function dis_does_not_contain {
    local matches=$(wasm-dis "$1" | grep "$2")
    if [[ "$matches" != "" ]]; then
        echo "ERROR! found $2 in $1:"
        echo
        echo "$matches"
        echo
        echo "wee_alloc should never pull in the panicking infrastructure"
        exit 1
    fi

}

function no_panic {
    dis_does_not_contain $1 "panic"
}

function no_fmt {
    dis_does_not_contain $1 "fmt"
}

function no_write {
    dis_does_not_contain $1 "Write"
}

for x in ../target/wasm32-unknown-unknown/release/*.gc.wasm; do
    no_panic "$x"
    no_fmt "$x"
    no_write "$x"
done

cd -
