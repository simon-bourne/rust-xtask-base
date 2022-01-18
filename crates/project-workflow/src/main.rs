use std::error::Error;

use clap::Parser;
use workflow::{build_readme, from_args, generate_open_source_files};

#[derive(Parser)]
enum Commands {
    UpdateFiles,
}

fn main() -> Result<(), Box<dyn Error>> {
    match from_args::<Commands>() {
        Commands::UpdateFiles => {
            build_readme(".")?;
            generate_open_source_files(2021)?;
        }
    }

    Ok(())
}
