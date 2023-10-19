use super::actions::{
    self, install, install_rust, pull_request, push, run, rust_toolchain, Platform,
    Platform::UbuntuLatest, Workflow,
};

pub fn basic_tests(
    stable_rustc_version: &str,
    nightly_rustc_version: &str,
    udeps_version: &str,
) -> Workflow {
    let mut workflow = actions::workflow("basic-tests").on([push(), pull_request()]);

    for platform in Platform::latest() {
        workflow.add_job(
            "stable",
            platform,
            [
                install_rust(
                    rust_toolchain(stable_rustc_version)
                        .minimal()
                        .default()
                        .clippy(),
                ),
                run("cargo clippy --all-targets -- -D warnings -D clippy::all").into(),
                run("cargo test").into(),
                run("cargo build --all-targets").into(),
                run("cargo doc").into(),
                run("cargo test --benches --tests --release").into(),
            ],
        );
    }

    workflow.job(
        "nightly",
        UbuntuLatest,
        [
            install_rust(
                rust_toolchain(nightly_rustc_version)
                    .minimal()
                    .default()
                    .rustfmt(),
            ),
            run("cargo fmt --all -- --check").into(),
            install("cargo-udeps", udeps_version),
            run("cargo xtask ci nightly").into(),
            run("cargo udeps --all-targets").into(),
        ],
    )
}
