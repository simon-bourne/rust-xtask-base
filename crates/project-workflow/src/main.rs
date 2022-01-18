use clap::Parser;
use workflow::{build_readme, generate_open_source_files, run};

#[derive(Parser)]
enum Commands {
    UpdateFiles,
}

fn main() {
    run(|commands| {
        match commands {
            Commands::UpdateFiles => {
                build_readme(".")?;
                generate_open_source_files(2021)?;
            }
        }

        Ok(())
    });
}
