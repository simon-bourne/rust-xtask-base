# Rust Workflow Utils

[![tests](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml/badge.svg)](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml)

Utilities to create workflows for Rust projects. Just add a crate called "project-workflow" to your workspace, with a `main.rs` like:

```rust
use clap::Parser;
use xtask_base::{
    build_readme, ci, ci_fast, ci_nightly, ci_stable, generate_open_source_files, run, CommonCmds,
};

#[derive(Parser)]
enum Commands {
    UpdateFiles,
    CiNightly,
    CiFast,
    CiStable,
    Ci,
    #[clap(flatten)]
    Common(CommonCmds)
}

fn main() {
    run(|| {
        match Commands::parse() {
            Commands::UpdateFiles => {
                build_readme(".")?;
                generate_open_source_files(2022)?;
            }
            Commands::CiNightly => ci_nightly()?,
            Commands::CiFast => ci_fast()?,
            Commands::CiStable => ci_stable()?,
            Commands::Ci => ci()?,
            Commands::Common(cmds) => cmds.run::<Commands>()?
        }

        Ok(())
    });
}

```
