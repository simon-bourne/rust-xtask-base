use clap::Parser;
use xtask_base::{build_readme, ci, generate_open_source_files, run, CommonCmds, Toolchain};

#[derive(Parser)]
enum Commands {
    UpdateFiles,
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
            Commands::UpdateFiles => {
                build_readme(".")?;
                generate_open_source_files(2022)?;
            }
            Commands::Ci { fast, toolchain } => ci(fast, toolchain)?,
            Commands::Common(cmds) => cmds.run::<Commands>()?,
        }

        Ok(())
    });
}
