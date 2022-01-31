use clap::Parser;
use xtask_base::{build_readme, ci, generate_open_source_files, run, CommonCmds};

#[derive(Parser)]
enum Commands {
    /// Generate derived files. Existing content will be overritten.
    Codegen {
        #[clap(long)]
        check: bool,
    },
    /// Run CI checks
    Ci {
        #[clap(long)]
        fast: bool,
        toolchain: Option<String>,
    },
    #[clap(flatten)]
    Common(CommonCmds),
}

fn main() {
    run(|workspace| {
        match Commands::parse() {
            Commands::Codegen { check } => {
                build_readme(".", check)?;
                generate_open_source_files(2022, check)?;
            }
            Commands::Ci { fast, toolchain } => {
                build_readme(".", true)?;
                generate_open_source_files(2022, true)?;
                ci(fast, &toolchain)?;
            }
            Commands::Common(cmds) => cmds.run::<Commands>(workspace)?,
        }

        Ok(())
    });
}
