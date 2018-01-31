#!/usr/bin/env bash

set -eux

cd $(dirname $0)

cd ./test
time cargo test --release --features "extra_assertions size_classes"
time cargo test --release --features "extra_assertions"
time cargo test --release --features "size_classes"
time cargo test --release
cd -
