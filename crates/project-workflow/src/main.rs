use std::error::Error;

use clap::Parser;
use workflow::{from_args, build_readme, generate_open_source_files};

#[derive(Parser)]
enum Commands {
    BuildReadme,
    GenerateFiles
}

fn main() -> Result<(), Box<dyn Error>> {
    match from_args::<Commands>() {
        Commands::BuildReadme => build_readme("."),
        Commands::GenerateFiles => generate_open_source_files(2021),
    }
}
