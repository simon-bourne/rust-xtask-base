use std::{
    env, error,
    fs::{self, File},
    io,
    os::unix::prelude::PermissionsExt,
    path::Path,
    process::{self, Output},
    result,
};

use chrono::{Datelike, Utc};
use clap::{App, Arg, ArgMatches, FromArgMatches, IntoApp};
use clap_complete::Shell;
use handlebars::{Handlebars, RenderError};
use serde_json::json;

pub type WorkflowResult<T> = Result<T, Box<dyn error::Error>>;

pub fn run(f: impl FnOnce() -> WorkflowResult<()>) {
    f().unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
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

pub fn from_args<T: IntoApp + FromArgMatches>() -> T {
    let mut app = T::into_app_for_update()
        .arg(
            Arg::new(SHELL_COMPLETIONS)
                .long(SHELL_COMPLETIONS)
                .help("Generate shell completions")
                .possible_values(Shell::possible_values())
                .exclusive(true),
        )
        .subcommand(App::new(CARGO_FMT).about("Run cargo fmt"))
        .subcommand(App::new(CARGO_UDEPS).about("Run cargo udeps"))
        .subcommand(
            App::new(CARGO_EXPAND)
                .about("Run cargo expand")
                .arg(Arg::new(ARG_PACKAGE).required(true)),
        );

    let arg_matches = app
        .try_get_matches_from_mut(env::args())
        .unwrap_or_else(|e| e.exit());

    if let Ok(generator) = arg_matches.value_of_t::<Shell>(SHELL_COMPLETIONS) {
        clap_complete::generate(generator, &mut app, "./workflow", &mut io::stdout());
        process::exit(0);
    }

    try_subcmd(CARGO_FMT, &arg_matches, |_| cargo_fmt());
    try_subcmd(CARGO_UDEPS, &arg_matches, |_| cargo_udeps());
    try_subcmd(CARGO_EXPAND, &arg_matches, |args| {
        shell(&format!(
            "cargo expand --color=always --package '{}' | less -r",
            args.value_of(ARG_PACKAGE).unwrap()
        ))
    });

    T::from_arg_matches(&arg_matches).unwrap_or_else(|e| e.exit())
}

const SHELL_COMPLETIONS: &str = "shell-completions";
const CARGO_FMT: &str = "fmt";
const CARGO_UDEPS: &str = "udeps";
const CARGO_EXPAND: &str = "expand";
const ARG_PACKAGE: &str = "package";

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

pub fn generate_workflow_script() -> WorkflowResult<()> {
    let workflow_file = "workflow";

    fs::write(workflow_file, include_str!("boilerplate/workflow"))?;
    let mut perms = fs::metadata(workflow_file)?.permissions();
    perms.set_mode(0o744);
    fs::set_permissions(workflow_file, perms)?;

    fs::write(
        "bash-completions",
        include_str!("boilerplate/bash-completions"),
    )?;

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
    generate_workflow_script()?;
    generate_license_apache(start_year)?;
    generate_license_mit(start_year)?;

    Ok(())
}

pub fn cargo_nightly(cmd: &str) -> WorkflowResult<()> {
    shell(&format!("cargo +nightly {}", cmd))
}

pub fn cargo(cmd: &str) -> WorkflowResult<()> {
    shell(&format!("cargo {}", cmd))
}

fn cargo_udeps() -> WorkflowResult<()> {
    cargo_nightly("udeps --all-targets")
}

fn cargo_fmt() -> WorkflowResult<()> {
    cargo_nightly("fmt --all")
}

pub fn ci_nightly() -> WorkflowResult<()> {
    cargo_nightly("fmt --all --check")?;
    cargo_udeps()
}

pub fn ci_fast() -> WorkflowResult<()> {
    cargo("clippy --all-targets -- -D warnings -D clippy::all")?;
    cargo("test")?;
    cargo("build --all-targets")?;
    cargo("doc")?;

    Ok(())
}

pub fn ci_stable() -> WorkflowResult<()> {
    ci_fast()?;
    cargo("test --benches --tests --release")
}

pub fn ci() -> WorkflowResult<()> {
    ci_stable()?;
    ci_nightly()?;
    Ok(())
}

pub fn shell(command: &str) -> WorkflowResult<()> {
    let exit_status = execute::shell(command).status()?;

    if !exit_status.success() {
        if let Some(code) = exit_status.code() {
            process::exit(code);
        } else {
            process::exit(1);
        }
    }

    Ok(())
}
