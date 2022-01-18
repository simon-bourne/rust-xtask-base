# Rust Workflow Utils

Utilities to create workflows for Rust projects. For example:

```rust
use std::error::Error;

use clap::Parser;
use workflow::{from_args, build_readme};

#[derive(Parser)]
enum Commands {
    BuildReadme,
}

fn main() -> Result<(), Box<dyn Error>> {
    match from_args::<Commands>() {
        Commands::BuildReadme => build_readme("."),
    }
}

```
