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
