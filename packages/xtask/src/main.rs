use xtask_base::{
    build_readme,
    ci::{StandardVersions, CI},
    generate_open_source_files, CommonCmds, WorkflowResult,
};

fn main() {
    let rustc_stable_version = "1.85.1";
    CommonCmds::run(
        CI::standard_workflow(
            StandardVersions {
                rustc_stable_version,
                rustc_nightly_version: "nightly-2025-03-15",
                udeps_version: "0.1.55",
            },
            &[],
        ),
        code_gen,
    );
}

fn code_gen(check: bool) -> WorkflowResult<()> {
    build_readme(".", check)?;
    generate_open_source_files(2022, check)
}
