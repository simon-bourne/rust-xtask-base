use xtask_base::{build_readme, ci::CI, generate_open_source_files, CommonCmds, WorkflowResult};

fn main() {
    CommonCmds::run(|| ci().run(), code_gen)
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
