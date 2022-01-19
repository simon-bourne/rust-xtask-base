# Rust Workflow Utils

[![tests](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml/badge.svg)](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml)

Utilities to create workflows for Rust projects. Just add a crate called "project-workflow" to your workspace, with a `main.rs` like:

```rust
use clap::Parser;
use xtask_base::{build_readme, ci, generate_open_source_files, run, CommonCmds, Toolchain};

#[derive(Parser)]
enum Commands {
    Codegen {
        #[clap(long)]
        check: bool,
    },
    Ci {
        #[clap(long)]
        fast: bool,
        toolchain: Option<Toolchain>,
    },
    #[clap(flatten)]
    Common(CommonCmds),
}

fn main() {
    run(|| {
        match Commands::parse() {
            Commands::Codegen { check } => {
                build_readme(".", check)?;
                generate_open_source_files(2022, check)?;
            }
            Commands::Ci { fast, toolchain } => {
                build_readme(".", true)?;
                generate_open_source_files(2022, true)?;
                ci(fast, toolchain)?;
            }
            Commands::Common(cmds) => cmds.run::<Commands>()?,
        }

        Ok(())
    });
}

```
