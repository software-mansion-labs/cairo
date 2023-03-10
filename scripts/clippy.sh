#!/bin/bash

rustup component add clippy --toolchain nightly-x86_64-unknown-linux-gnu

cargo clippy "$@" --all-targets --all-features -- -D warnings -D future-incompatible \
    -D nonstandard-style -D rust-2018-idioms -D unused
