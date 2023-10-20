use clap::Parser;
use xtask_base::{
    build_readme, ci::CI, generate_open_source_files, in_workspace, CommonCmds, WorkflowResult,
};

#[derive(Parser)]
enum Commands {
    /// Generate derived files. Existing content will be overritten.
    Codegen {
        #[clap(long)]
        check: bool,
    },
    /// Run CI checks
    Ci,
    #[clap(flatten)]
    Common(CommonCmds),
}

fn main() {
    in_workspace(|workspace| {
        match Commands::parse() {
            Commands::Codegen { check } => code_gen(check)?,
            Commands::Ci => ci().run()?,
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
    ci().write(check)
}

fn ci() -> CI {
    CI::standard_workflow()
}