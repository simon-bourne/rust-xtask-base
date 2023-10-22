# Rust Workflow Utils

[![tests](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml/badge.svg)](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml)

Utilities for creating [cargo-xtask](https://github.com/matklad/cargo-xtask) projects. Create an `xtask` crate with a `main.rs` something like:

```rust
use xtask_base::{
    build_readme,
    ci::{StandardVersions, CI},
    generate_open_source_files, CommonCmds, WorkflowResult,
};

fn main() {
    CommonCmds::run(
        CI::standard_workflow(StandardVersions::default(), &[]),
        code_gen,
    )
}

fn code_gen(check: bool) -> WorkflowResult<()> {
    build_readme(".", check)?;
    generate_open_source_files(2022, check)
}

```
