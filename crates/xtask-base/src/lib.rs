use std::{
    env, error, fs,
    path::Path,
    process::{self, Output},
    result,
};

use chrono::{Datelike, Utc};
use clap::IntoApp;
use clap_complete::Shell;
use handlebars::{Handlebars, RenderError};
use parse_display::{Display, FromStr};
use serde_json::json;
use xshell::{cmd, mkdir_p, pushd, read_file, write_file};

pub type WorkflowResult<T> = Result<T, Box<dyn error::Error>>;

// TODO: Update README
// TODO: Use "CARGO" env to get cargo binary

pub fn run(f: impl FnOnce() -> WorkflowResult<()>) {
    // TODO: Use cargo metadata to get workspace root?
    let _dir = pushd(
        Path::new(&env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf(),
    )
    .unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

    f().unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
}

#[derive(clap::Parser)]
pub enum CommonCmds {
    ShellCompletion { shell: Shell },
    Fmt,
    Udeps,
    MacroExpand { package: String },
}

impl CommonCmds {
    pub fn run<T: IntoApp>(&self) -> WorkflowResult<()> {
        match self {
            CommonCmds::ShellCompletion { shell } => {
                clap_complete::generate_to(*shell, &mut T::into_app(), "./cargo-xtask", "target")?;
                println!("Completions file generated in `target` dir");
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

mod handlebars_helpers {
    use std::fs;

    use handlebars::handlebars_helper;

    use crate::run_process;

    handlebars_helper!(include: |file: str| { fs::read_to_string(file)? });
    handlebars_helper!(shell: |cmd: str| { run_process(cmd)? });
}

fn run_process(cmd: &str) -> result::Result<String, RenderError> {
    let mut shell_cmd = execute::shell(cmd);

    let Output {
        status,
        stdout,
        stderr,
    } = shell_cmd.output()?;

    let output = String::from_utf8(stdout)?;

    if !stderr.is_empty() {
        return Err(RenderError::new(format!(
            "Stderr is not empty:\n\n{}",
            String::from_utf8(stderr)?
        )));
    }

    if !status.success() {
        return Err(RenderError::new(status.code().map_or_else(
            || "Process failed".to_owned(),
            |code| format!("Process exited with code {}", code),
        )));
    }

    Ok(output)
}

pub fn build_readme(dir: &str, check: bool) -> WorkflowResult<()> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);
    reg.register_helper("include", Box::new(handlebars_helpers::include));
    reg.register_helper("shell", Box::new(handlebars_helpers::shell));

    let dir = Path::new(dir);
    let template = fs::read_to_string(dir.join("README.tmpl.md"))?;

    update_file(
        &dir.join("README.md"),
        &reg.render_template(&template, &"{}")?,
        check,
    )
}

// TODO: Rename WorkflowResult to XTaskResult?
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
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);

    let end_year = Utc::now().year();

    let copyright_range = if start_year == end_year {
        format!("{}", start_year)
    } else {
        format!("{}-{}", start_year, end_year)
    };

    update_file(
        filename,
        &reg.render_template(template, &json!({ "copyright_range": copyright_range }))?,
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

fn cargo_udeps() -> WorkflowResult<()> {
    cmd!("cargo +nightly udeps --all-targets").run()?;
    Ok(())
}

fn cargo_fmt(check: bool) -> WorkflowResult<()> {
    let check = if check { &["--", "--check"] } else { &[][..] };
    cmd!("cargo +nightly fmt --all {check...}").run()?;
    Ok(())
}

#[derive(Display, FromStr, Debug, Eq, PartialEq, Copy, Clone)]
#[display(style = "snake_case")]
pub enum Toolchain {
    Stable,
    Nightly,
}

// TODO: Run macro, for cmd!(...).run()? and re-export duct::cmd

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
