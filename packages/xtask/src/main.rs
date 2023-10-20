use clap::{Parser, Subcommand};
use xtask_base::{
    build_readme, ci::CI, ci_nightly, generate_open_source_files, run, CommonCmds, WorkflowResult,
};

#[derive(Parser)]
enum Commands {
    /// Generate derived files. Existing content will be overritten.
    Codegen {
        #[clap(long)]
        check: bool,
    },
    /// Run CI checks
    Ci {
        #[clap(subcommand)]
        command: Option<CiCommand>,
    },
    #[clap(flatten)]
    Common(CommonCmds),
}

#[derive(Subcommand, PartialEq, Eq)]
enum CiCommand {
    Stable {
        #[clap(long)]
        fast: bool,
        toolchain: Option<String>,
    },
    Nightly {
        toolchain: Option<String>,
    },
}

fn main() {
    run(|workspace| {
        match Commands::parse() {
            Commands::Codegen { check } => code_gen(check)?,
            Commands::Ci { command } => {
                if let Some(command) = command {
                    match command {
                        CiCommand::Stable { fast, toolchain } => {
                            ci_stable(fast, toolchain.as_deref())?;
                        }
                        CiCommand::Nightly { toolchain } => ci_nightly(toolchain.as_deref())?,
                    }
                } else {
                    ci_stable(false, None)?;
                    ci_nightly(Some("nightly"))?;
                }
            }
            Commands::Common(cmds) => cmds.run::<Commands>(workspace)?,
        }

        Ok(())
    });
}

fn code_gen(check: bool) -> WorkflowResult<()> {
    build_readme(".", check)?;
    generate_open_source_files(2022, check)?;
    github_actions(check)
}

fn github_actions(check: bool) -> WorkflowResult<()> {
    CI::standard_workflow().write(check)
}

fn ci_stable(fast: bool, toolchain: Option<&str>) -> WorkflowResult<()> {
    code_gen(true)?;
    xtask_base::ci_stable(fast, toolchain, &[])?;
    Ok(())
}
