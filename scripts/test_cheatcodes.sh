#!/bin/bash

set -e

# cairo => sierra
# cargo run --bin cairo-compile -- --replace-ids ./examples/cheatcode_caller.cairo ./target/output.sierra
# sierra => casm
# cargo run --bin cairo-protostar ./target/output.sierra ./target/out.json
# run tests
cargo run --bin cairo-test -- -p ./examples
