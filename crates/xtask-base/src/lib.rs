use std::{error, fs, path::Path, process};

use cargo_metadata::{Metadata, MetadataCommand};
use chrono::{Datelike, Utc};
use clap::CommandFactory;
use clap_complete::Shell;
use itertools::Itertools;
use serde_json::json;
use xshell::{cmd, mkdir_p, pushd, read_file, write_file};

mod template;

pub type WorkflowResult<T> = Result<T, Box<dyn error::Error>>;

#[derive(clap::Parser)]
pub enum CommonCmds {
    /// Generate shell completions
    ShellCompletion { shell: Shell },
    /// Format all code
    Fmt,
    /// Check all dependencies are used
    Udeps,
    /// Show expanded macros
    MacroExpand { package: String },
}

impl CommonCmds {
    /// Run the subcommand for `self`
    pub fn run<T: CommandFactory>(&self, workspace: &Workspace) -> WorkflowResult<()> {
        match self {
            CommonCmds::ShellCompletion { shell } => {
                let target_dir = workspace.target_dir();
                clap_complete::generate_to(
                    *shell,
                    &mut T::command(),
                    "./cargo-xtask",
                    target_dir,
                )?;
                println!("Completions file generated in `{}`", target_dir.display());
                Ok(())
            }
            CommonCmds::Fmt => cargo_fmt(Some("+nightly"), false),
            CommonCmds::Udeps => cargo_udeps(Some("+nightly")),
            CommonCmds::MacroExpand { package } => {
                duct::cmd("cargo", ["expand", "--color=always", "--package", package])
                    .pipe(duct::cmd("less", ["-r"]))
                    .run()?;
                Ok(())
            }
        }
    }
}

/// Metadata about the cargo workspace
pub struct Workspace(Metadata);

impl Workspace {
    /// The cargo target directory
    ///
    /// This is where all generated files go.
    pub fn target_dir(&self) -> &Path {
        self.0.target_directory.as_std_path()
    }
}

/// Run a function, passing it a [Workspace]
///
/// If an error is returned, a human friendly version is output, and the process
/// exits with code 1
pub fn run(f: impl FnOnce(&Workspace) -> WorkflowResult<()>) {
    run_or_err(f).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
}

fn run_or_err(f: impl FnOnce(&Workspace) -> WorkflowResult<()>) -> WorkflowResult<()> {
    let metadata = MetadataCommand::new().exec()?;

    let _dir = pushd(&metadata.workspace_root)?;

    f(&Workspace(metadata))
}

/// Build `README.md` from `README.tmpl.md`
///
/// The template is a Handlebars template with helpers:
///
/// - `{{ include "my-file.txt" }}` will include the contents of `my-file.txt`
/// - `{{ shell "ls -l" }}` will run `ls -l` and include the contents of it's
///   `stdout`. The system shell is used to run the command.
pub fn build_readme(dir: &str, check: bool) -> WorkflowResult<()> {
    let dir = Path::new(dir);
    let template = fs::read_to_string(dir.join("README.tmpl.md"))?;

    update_file(
        &dir.join("README.md"),
        &template::registry().render_template(&template, &"{}")?,
        check,
    )
}

/// Generate Rustfmt and Cargo configs, and dual Apache 2 and MIT licenses
///
/// The follwing files are generated in the workspace root:
///
/// - `rustmt.toml`
/// - `.cargo/config`
/// - `LICENSE-APACHE`
/// - `LICENSE-MIT`
pub fn generate_open_source_files(start_year: i32, check: bool) -> WorkflowResult<()> {
    generate_rustfmt_config(check)?;
    generate_cargo_config(check)?;
    generate_license_apache(start_year, check)?;
    generate_license_mit(start_year, check)?;

    Ok(())
}

/// Generate `rustfmt.toml` in the workspace root
pub fn generate_rustfmt_config(check: bool) -> WorkflowResult<()> {
    update_file(
        "rustfmt.toml",
        include_str!("boilerplate/rustfmt.toml"),
        check,
    )?;

    Ok(())
}

/// Generate `.cargo/config` in the workspace root
///
/// It contains a single alias for `xtask`
pub fn generate_cargo_config(check: bool) -> WorkflowResult<()> {
    if !check {
        mkdir_p(".cargo")?;
    }

    update_file(
        ".cargo/config",
        include_str!("boilerplate/.cargo/config"),
        check,
    )?;

    Ok(())
}

pub fn generate_license_apache(start_year: i32, check: bool) -> WorkflowResult<()> {
    generate_license(
        include_str!("boilerplate/LICENSE-APACHE"),
        "LICENSE-APACHE",
        start_year,
        check,
    )
}

pub fn generate_license_mit(start_year: i32, check: bool) -> WorkflowResult<()> {
    generate_license(
        include_str!("boilerplate/LICENSE-MIT"),
        "LICENSE-MIT",
        start_year,
        check,
    )
}

fn generate_license(
    template: &str,
    filename: &str,
    start_year: i32,
    check: bool,
) -> WorkflowResult<()> {
    let end_year = Utc::now().year();

    let copyright_range = if start_year == end_year {
        format!("{}", start_year)
    } else {
        format!("{}-{}", start_year, end_year)
    };

    update_file(
        filename,
        &template::registry()
            .render_template(template, &json!({ "copyright_range": copyright_range }))?,
        check,
    )
}

fn update_file(path: impl AsRef<Path>, contents: &str, check: bool) -> WorkflowResult<()> {
    if check {
        let existing_contents = read_file(path.as_ref())?;

        if existing_contents != contents {
            return Err(
                format!("Differences found in file \"{}\"", path.as_ref().display()).into(),
            );
        }
    } else {
        write_file(path, contents)?;
    }

    Ok(())
}

/// Run basic CI checks on stable toolchain
///
/// - `cargo [clippy, test, build, doc]`
/// - `cargo test --benches --tests --release`, except in when `fast` is `true`
pub fn ci_stable(fast: bool, toolchain: Option<&str>, features: &[&str]) -> WorkflowResult<()> {
    let cargo_toolchain = &cargo_toolchain(toolchain);

    for feature_set in features.iter().copied().powerset() {
        clippy(toolchain, &feature_set)?;

        let feature_set = feature_set.join(",");

        cmd!("cargo {cargo_toolchain...} test --features {feature_set}").run()?;
        cmd!("cargo {cargo_toolchain...} build --all-targets --features {feature_set}").run()?;
        cmd!("cargo {cargo_toolchain...} doc --features {feature_set}").run()?;

        if !fast {
            cmd!("cargo {cargo_toolchain...} test --benches --tests --release --features {feature_set}")
                .run()?;
        }
    }

    Ok(())
}

pub fn clippy(toolchain: Option<&str>, features: &[&str]) -> WorkflowResult<()> {
    let toolchain = cargo_toolchain(toolchain);
    let feature_set = features.join(",");

    cmd!("cargo {toolchain...} clippy --features {feature_set} --all-targets -- -D warnings -D clippy::all").run()?;
    Ok(())
}

fn cargo_toolchain(toolchain: Option<&str>) -> Option<String> {
    toolchain.as_ref().map(|tc| format!("+{}", tc))
}

/// Nightly only CI checks:
///
/// - `cargo fmt`
/// - `cargo udeps`
pub fn ci_nightly(toolchain: Option<&str>) -> WorkflowResult<()> {
    let toolchain = toolchain.as_ref().map(|tc| format!("+{}", tc));
    let toolchain = toolchain.as_deref();
    cargo_fmt(toolchain, true)?;
    cargo_udeps(toolchain)?;

    Ok(())
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TargetOs {
    Windows,
    MacOs,
    Ios,
    Linux,
    Android,
    FreeBsd,
    Dragonfly,
    OpenBsd,
    NetBsd,
}

pub fn target_os() -> TargetOs {
    #[cfg(target_os = "windows")]
    return TargetOs::Windows;
    #[cfg(target_os = "macos")]
    return TargetOs::MacOs;
    #[cfg(target_os = "ios")]
    return TargetOs::Ios;
    #[cfg(target_os = "linux")]
    return TargetOs::Linux;
    #[cfg(target_os = "android")]
    return TargetOs::Android;
    #[cfg(target_os = "freebsd")]
    return TargetOs::FreeBsd;
    #[cfg(target_os = "dragonfly")]
    return TargetOs::Dragonfly;
    #[cfg(target_os = "openbsd")]
    return TargetOs::OpenBsd;
    #[cfg(target_os = "netbsd")]
    return TargetOs::NetBsd;
}

fn cargo_udeps(toolchain: Option<&str>) -> WorkflowResult<()> {
    cmd!("cargo {toolchain...} udeps --all-targets").run()?;
    Ok(())
}

fn cargo_fmt(toolchain: Option<&str>, check: bool) -> WorkflowResult<()> {
    let check = if check { &["--", "--check"] } else { &[][..] };
    cmd!("cargo {toolchain...} fmt --all {check...}").run()?;
    Ok(())
}
