#!/usr/bin/env bash

set -eux

cd $(dirname $0)

# Generate new README.md and exit if it differs from the current one.
OLD_README=`mktemp`
cp README.md $OLD_README
cargo readme -r wee_alloc -t "$(pwd)/README.tpl" > README.md
diff $OLD_README README.md

cd ./test
time cargo test --release --features "extra_assertions size_classes"
time cargo test --release --features "extra_assertions"
time cargo test --release --features "size_classes"
time cargo test --release

export WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES=$((1024 * 1024 * 1024))

time cargo test --release --features "static_array_backend extra_assertions size_classes"
time cargo test --release --features "static_array_backend extra_assertions"
time cargo test --release --features "static_array_backend size_classes"
time cargo test --release --features "static_array_backend"
cd -
