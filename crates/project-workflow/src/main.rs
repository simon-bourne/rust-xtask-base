use clap::Parser;
use workflow::{
    build_readme, cargo_fmt, cargo_udeps, ci, ci_fast, ci_nightly, ci_stable, from_args,
    generate_open_source_files, run,
};

#[derive(Parser)]
enum Commands {
    UpdateFiles,
    Udeps,
    Fmt,
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
            Commands::Udeps => cargo_udeps()?,
            Commands::Fmt => cargo_fmt()?,
            Commands::CiNightly => ci_nightly()?,
            Commands::CiFast => ci_fast()?,
            Commands::CiStable => ci_stable()?,
            Commands::Ci => ci()?,
        }

        Ok(())
    });
}
