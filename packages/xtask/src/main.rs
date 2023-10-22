use xtask_base::{
    build_readme,
    ci::{StandardVersions, CI},
    generate_open_source_files, CommonCmds, WorkflowResult,
};

fn main() {
    CommonCmds::run(
        CI::standard_workflow(StandardVersions::default(), &[]),
        code_gen,
    )
}

fn code_gen(check: bool) -> WorkflowResult<()> {
    build_readme(".", check)?;
    generate_open_source_files(2022, check)
}
