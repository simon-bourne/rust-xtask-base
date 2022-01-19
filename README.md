# Rust Workflow Utils

[![tests](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml/badge.svg)](https://github.com/simon-bourne/rust-project/actions/workflows/tests.yml)

Utilities for creating [cargo-xtask](https://github.com/matklad/cargo-xtask) projects. Create an `xtask` crate with a `main.rs` something like:

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
