use std::{error, fs, path::Path, process};

use cargo_metadata::{Metadata, MetadataCommand};
use chrono::{Datelike, Utc};
use clap::IntoApp;
use clap_complete::Shell;
use parse_display::{Display, FromStr};
use serde_json::json;
use xshell::{cmd, mkdir_p, pushd, read_file, write_file};

mod template;

pub type WorkflowResult<T> = Result<T, Box<dyn error::Error>>;

#[derive(clap::Parser)]
pub enum CommonCmds {
    /// Generate shell completions
    ShellCompletion {
        shell: Shell,
    },
    Fmt,
    Udeps,
    MacroExpand {
        package: String,
    },
}

impl CommonCmds {
    pub fn run<T: IntoApp>(&self, workspace: &Workspace) -> WorkflowResult<()> {
        match self {
            CommonCmds::ShellCompletion { shell } => {
                let target_dir = workspace.target_dir();
                clap_complete::generate_to(
                    *shell,
                    &mut T::into_app(),
                    "./cargo-xtask",
                    target_dir,
                )?;
                println!("Completions file generated in `{}`", target_dir.display());
                Ok(())
            }
            CommonCmds::Fmt => cargo_fmt(false),
            CommonCmds::Udeps => cargo_udeps(),
            CommonCmds::MacroExpand { package } => {
                duct::cmd("cargo", ["expand", "--color=always", "--package", package])
                    .pipe(duct::cmd("less", ["-r"]))
                    .run()?;
                Ok(())
            }
        }
    }
}

pub struct Workspace(Metadata);

impl Workspace {
    pub fn target_dir(&self) -> &Path {
        self.0.target_directory.as_std_path()
    }
}

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

pub fn build_readme(dir: &str, check: bool) -> WorkflowResult<()> {
    let dir = Path::new(dir);
    let template = fs::read_to_string(dir.join("README.tmpl.md"))?;

    update_file(
        &dir.join("README.md"),
        &template::registry().render_template(&template, &"{}")?,
        check,
    )
}

pub fn generate_open_source_files(start_year: i32, check: bool) -> WorkflowResult<()> {
    generate_rustfmt_config(check)?;
    generate_cargo_config(check)?;
    generate_license_apache(start_year, check)?;
    generate_license_mit(start_year, check)?;

    Ok(())
}

pub fn generate_rustfmt_config(check: bool) -> WorkflowResult<()> {
    update_file(
        "rustfmt.toml",
        include_str!("boilerplate/rustfmt.toml"),
        check,
    )?;

    Ok(())
}

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

#[derive(Display, FromStr, Debug, Eq, PartialEq, Copy, Clone)]
#[display(style = "kebab-case")]
pub enum Toolchain {
    Stable,
    Nightly,
}

pub fn ci(fast: bool, toolchain: Option<Toolchain>) -> WorkflowResult<()> {
    if toolchain.map_or(true, |tc| tc == Toolchain::Nightly) {
        cargo_fmt(true)?;
        cargo_udeps()?;
    }

    if toolchain.map_or(true, |tc| tc == Toolchain::Stable) {
        cmd!("cargo clippy --all-targets -- -D warnings -D clippy::all").run()?;
        cmd!("cargo test").run()?;
        cmd!("cargo build --all-targets").run()?;
        cmd!("cargo doc").run()?;

        if !fast {
            cmd!("cargo test --benches --tests --release").run()?;
        }
    }

    Ok(())
}

fn cargo_udeps() -> WorkflowResult<()> {
    cmd!("cargo +nightly udeps --all-targets").run()?;
    Ok(())
}

fn cargo_fmt(check: bool) -> WorkflowResult<()> {
    let check = if check { &["--", "--check"] } else { &[][..] };
    cmd!("cargo +nightly fmt --all {check...}").run()?;
    Ok(())
}
