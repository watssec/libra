#!/bin/bash

# exit when any command fails
set -e

# build the `binder`
cd /project/binder
cargo build

# build LLVM from source
cd builder
cargo run -- deps llvm build
cd -

# build pass from source
cd builder
cargo run -- pass
cd -

