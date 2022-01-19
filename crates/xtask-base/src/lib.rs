use std::{
    error,
    fs::{self, File},
    io,
    path::Path,
    process::{self, Output},
    result,
};

use chrono::{Datelike, Utc};
use clap::{ArgMatches, IntoApp};
use clap_complete::Shell;
use handlebars::{Handlebars, RenderError};
use serde_json::json;
use xshell::{cmd, mkdir_p, write_file};

pub type WorkflowResult<T> = Result<T, Box<dyn error::Error>>;

// TODO: Update README
// TODO: Add .cargo/config with the alias for xtask
// TODO: Generate completions to target/...
// TODO: Add an alias/completion to complete from target/completions
// TODO: Add an alias to generate completions
// TODO: run via cargo xtask
// TODO: cd to cargo dir (CARGO_MANIFEST_DIR)
// TODO: Use "CARGO" env to get cargo binary

pub fn run(f: impl FnOnce() -> WorkflowResult<()>) {
    f().unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
}

#[derive(clap::Parser)]
pub enum CommonCmds {
    ShellCompletion { shell: Shell },
    Fmt,
    UDeps,
    MacroExpand { package: String },
}

impl CommonCmds {
    pub fn run<T: IntoApp>(&self) -> WorkflowResult<()> {
        match self {
            CommonCmds::ShellCompletion { shell } => {
                clap_complete::generate(
                    *shell,
                    &mut T::into_app(),
                    "./cargo-xtask",
                    &mut io::stdout(),
                );
                Ok(())
            }
            CommonCmds::Fmt => cargo_fmt(false),
            CommonCmds::UDeps => cargo_udeps(),
            CommonCmds::MacroExpand { package } => {
                duct::cmd("cargo", ["expand", "--color=always", "--package", package])
                    .pipe(duct::cmd("less", ["-r"]))
                    .run()?;
                Ok(())
            }
        }
    }
}

pub fn try_subcmd(
    name: &str,
    arg_matches: &ArgMatches,
    f: impl FnOnce(&ArgMatches) -> WorkflowResult<()>,
) {
    if let Some((subcmd, args)) = arg_matches.subcommand() {
        if name == subcmd {
            run(|| f(args));
            process::exit(0);
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

pub fn build_readme(dir: &str) -> WorkflowResult<()> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);
    reg.register_helper("include", Box::new(handlebars_helpers::include));
    reg.register_helper("shell", Box::new(handlebars_helpers::shell));

    let dir = Path::new(dir);
    let template = fs::read_to_string(dir.join("README.tmpl.md"))?;

    reg.render_template_to_write(&template, &"{}", File::create(dir.join("README.md"))?)?;

    Ok(())
}

pub fn generate_rustfmt_config() -> WorkflowResult<()> {
    fs::write("rustfmt.toml", include_str!("boilerplate/rustfmt.toml"))?;

    Ok(())
}

pub fn generate_cargo_config() -> WorkflowResult<()> {
    mkdir_p(".cargo")?;
    write_file(".cargo/config", include_str!("boilerplate/.cargo/config"))?;

    Ok(())
}

pub fn generate_license_apache(start_year: i32) -> WorkflowResult<()> {
    generate_license(
        include_str!("boilerplate/LICENSE-APACHE"),
        "LICENSE-APACHE",
        start_year,
    )
}

pub fn generate_license_mit(start_year: i32) -> WorkflowResult<()> {
    generate_license(
        include_str!("boilerplate/LICENSE-MIT"),
        "LICENSE-MIT",
        start_year,
    )
}

fn generate_license(template: &str, filename: &str, start_year: i32) -> WorkflowResult<()> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);

    let end_year = Utc::now().year();

    let copyright_range = if start_year == end_year {
        format!("{}", start_year)
    } else {
        format!("{}-{}", start_year, end_year)
    };

    reg.render_template_to_write(
        template,
        &json!({ "copyright_range": copyright_range }),
        File::create(filename)?,
    )?;

    Ok(())
}

pub fn generate_open_source_files(start_year: i32) -> WorkflowResult<()> {
    generate_rustfmt_config()?;
    generate_cargo_config()?;
    generate_license_apache(start_year)?;
    generate_license_mit(start_year)?;

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

pub fn ci_nightly() -> WorkflowResult<()> {
    cargo_fmt(true)?;
    cargo_udeps()
}

pub fn ci_fast() -> WorkflowResult<()> {
    cmd!("cargo clippy --all-targets -- -D warnings -D clippy::all").run()?;
    cmd!("cargo test").run()?;
    cmd!("cargo build --all-targets").run()?;
    cmd!("cargo doc").run()?;

    Ok(())
}

pub fn ci_stable() -> WorkflowResult<()> {
    ci_fast()?;
    cmd!("cargo test --benches --tests --release").run()?;
    Ok(())
}

pub fn ci() -> WorkflowResult<()> {
    ci_nightly()?;
    ci_stable()?;
    Ok(())
}
