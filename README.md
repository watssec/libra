# libra
LLVM IR Bindings for Rust-based Analyzers

## Overview

This is an umbrella repository hosting research and development effort
involving LLVM IR, including themes like
- fuzzing
- symbolic execution
- concolic execution
- formal verification

Libra takes a drastic approach in its architecture:
instead of developing LLVM passes in C++ (for analysis or instrumentation),
Libra first re-hosts the LLVM IR into its home-grown Rust representation and
performs either static analysis or dynamic interpretation from there.

## Getting Started

- Install Rust
  - Check [official guide](https://www.rust-lang.org/tools/install)

- Getting the source code
  ```bash
  git clone git@github.com:watssec/libra.git
  git submodule update --init
  ```

- Build the `binder`
  ```bash
  cd <libra>/binder
  cargo build
  ```

- Build various Libra components through `binder/builder`
  - Build LLVM from source
    ```bash
    cd <libra>/binder/builder
    cargo run -- deps llvm build
    ```
    *NOTE*: If a prior build fails or you want to force a rebuild, use
    ```bash
    cargo run -- deps llvm build --force
    ```

  - Build the `oracle` LLVM pass
    ```bash
    cd <libra>/binder/builder
    cargo run -- pass
    ```
    *NOTE*: If a prior build fails or you want to force a rebuild, use
    ```bash
    cargo run -- pass --force
    ```

- Check `binder/engine` unit tests pass
  ```bash
  cd <libra>/binder/engine
  cargo test
  ```

- Check the `binder/testsuite` pass
  - For `external` test cases
    ```bash
    cd <libra>/binder/testsuite
    cargo run -- external build
    cargo run -- external run
    ```
    *NOTE*: If a prior run fails or you want to force a re-run, use
    ```bash
    cargo run -- external build --force
    cargo run -- external run --force
    ```
    **Expected output:**
    - a vast majority of tests pass
    - a few tests are skipped
    - a small portion of tests are not supported (with a breakdown of reasons)
    - **NO** failure cases for whatever reason

  - For `internal` test cases
    ```bash
    cd <libra>/binder/testsuite
    cargo run -- internal run
    ```
    *NOTE*: If a prior run fails or you want to force a re-run, use
    ```bash
    cargo run -- internal run --force
    ```
    **Expected output:**
    - a vast majority of tests pass
    - a few tests are skipped
    - a small portion of tests are not supported (with a breakdown of reasons)
    - a few tests fail due to compilation error
    - **NO** other failure cases

### Troubleshooting

**NOTE**: Libra is currently only tested on
- Ubuntu 22.04 LTS
- MacOS 13+ with Apple Silicon

If you run into error in the building steps,
likely they are caused by missing packages / dependencies
which can usually be resolved with
`apt-get install` (for Ubuntu) or
`brew install` (for MacOS).
Read the error message and try to resolve them yourself.
If you are blocked by any message, raise a GitHub issue.

## Contributing

- The `main` branch enforces linear history and hence,
  does not take merge commits.
  Familiarize yourself with `git rebase` and learn
  how to create a linear GIT history from Google
  (e.g., [here](https://www.atlassian.com/git/tutorials/merging-vs-rebasing)
  is a good starting point)

- Use [GitHub Issues](https://github.com/watssec/libra/issues) effectively.
  If you think things are not right or would like to request a feature
  or code refactor, create an issue to discuss it and track its progress.

## Project Layout

NOTES
- The layout shows important files and directories only.
- Components marked as work-in-progress (WIP) does not work as of yet

```
<project-root>
|
|   # Third-party dependencies as GIT submodules
|-- deps/
|   |-- llvm-project
|   |-- llvm-test-suite
|
|   # An LLVM pass that serializes the LLVM IR to JSON format
|-- oracle/
|
|   # The driver program for Libra
|-- binder/
|   |
|   |   # CLI for building various components of Libra
|   |-- builder/
|   |
|   |   # Core component of the Libra engine
|   |-- engine/
|   |   |
|   |   |   # source code of the engine
|   |   |-- src
|   |   |   |
|   |   |   |   # A tower of IR in Rust ADT (i.e., enum)
|   |   |   |-- ir
|   |   |   |   |
|   |   |   |   |   # level 1: deserialized from the JSON produced by `oracle`
|   |   |   |   |-- adapter
|   |   |   |   |
|   |   |   |   |   # level 2: reduced from `adapter` after type checking
|   |   |   |   |-- bridge
|   |   |   |   |
|   |   |   |   |   # level 3: reduced from `bridge` after memory flattening (WIP)
|   |   |   |   |-- flatten
|   |   |   |
|   |   |   |   # A pipeline that drives the conversion of IR across layers
|   |   |   |-- flow
|   |   |   |   |
|   |   |   |   |   # Convert a simple program (in source code) to LLVM IR
|   |   |   |   |-- build_simple.rs
|   |   |   |   |
|   |   |   |   |   # Perform repetitive optimization until fixedpoint
|   |   |   |   |-- fixedpoint.rs
|   |   |
|   |   |   # unit tests for the Libra engine
|   |   |-- tests
|   |
|   |   # CLI for running various test suites
|   |-- testsuite/
|   |
|   |   # source code of the testing infrastructure
|   |-- src
|   |   |
|   |   |   # test against the test cases in `llvm-test-suite`
|   |   |-- llvm_external.rs
|   |   |
|   |   |   # test against the test cases in `llvm-project/llvm/test`
|   |   |-- llvm_internal.rs
|   |
|   |   # Shared functionalities
|   |-- shared/
|
|   # Dockerfiles showing how to build Libra on vanilla systems (WIP)
|-- docker/
```