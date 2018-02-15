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
cd -
