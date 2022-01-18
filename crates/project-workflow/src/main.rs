use clap::Parser;
use workflow::{
    build_readme, ci, ci_fast, ci_nightly, ci_stable, from_args, generate_open_source_files, run,
};

#[derive(Parser)]
enum Commands {
    UpdateFiles,
    CiNightly,
    CiFast,
    CiStable,
    Ci,
}

fn main() {
    run(|| {
        match from_args() {
            Commands::UpdateFiles => {
                build_readme(".")?;
                generate_open_source_files(2021)?;
            }
            Commands::CiNightly => ci_nightly()?,
            Commands::CiFast => ci_fast()?,
            Commands::CiStable => ci_stable()?,
            Commands::Ci => ci()?,
        }

        Ok(())
    });
}
