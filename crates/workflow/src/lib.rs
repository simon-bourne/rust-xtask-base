use std::{
    env,
    error::Error,
    fs::{self, File},
    io,
    os::unix::prelude::PermissionsExt,
    path::Path,
    process::{self, Output},
};

use chrono::{Datelike, Utc};
use clap::{Arg, FromArgMatches, IntoApp};
use clap_complete::Shell;
use handlebars::{handlebars_helper, Handlebars, RenderError};
use serde_json::json;

pub fn from_args<T: IntoApp + FromArgMatches>() -> T {
    let mut app = T::into_app_for_update().arg(
        Arg::new(SHELL_COMPLETIONS)
            .long(SHELL_COMPLETIONS)
            .help("Generate shell completions")
            .possible_values(Shell::possible_values())
            .exclusive(true),
    );

    let arg_matches = app
        .try_get_matches_from_mut(env::args())
        .unwrap_or_else(|e| e.exit());

    if let Ok(generator) = arg_matches.value_of_t::<Shell>(SHELL_COMPLETIONS) {
        clap_complete::generate(generator, &mut app, "./workflow", &mut io::stdout());
        process::exit(0);
    }

    T::from_arg_matches(&arg_matches).unwrap_or_else(|e| e.exit())
}

const SHELL_COMPLETIONS: &str = "shell-completions";

handlebars_helper!(include: |file: str| { fs::read_to_string(file)? });
handlebars_helper!(shell: |cmd: str| { run_process(cmd)? });

fn run_process(cmd: &str) -> Result<String, RenderError> {
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

pub fn build_readme(dir: &str) -> Result<(), Box<dyn Error>> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);
    reg.register_helper("include", Box::new(include));
    reg.register_helper("shell", Box::new(shell));

    let dir = Path::new(dir);
    let template = fs::read_to_string(dir.join("README.tmpl.md"))?;

    reg.render_template_to_write(&template, &"{}", File::create(dir.join("README.md"))?)?;

    Ok(())
}

pub fn generate_rustfmt_config() -> Result<(), io::Error> {
    fs::write("rustfmt.toml", include_str!("boilerplate/rustfmt.toml"))
}

pub fn generate_workflow_script() -> Result<(), io::Error> {
    let workflow_file = "workflow";

    fs::write(workflow_file, include_str!("boilerplate/workflow"))?;
    let mut perms = fs::metadata(workflow_file)?.permissions();
    perms.set_mode(0o744);
    fs::set_permissions(workflow_file, perms)?;

    fs::write(
        "bash-completions",
        include_str!("boilerplate/bash-completions"),
    )
}

pub fn generate_license_apache(start_year: i32) -> Result<(), Box<dyn Error>> {
    generate_license(
        include_str!("boilerplate/LICENSE-APACHE"),
        "LICENSE-APACHE",
        start_year,
    )
}

pub fn generate_license_mit(start_year: i32) -> Result<(), Box<dyn Error>> {
    generate_license(
        include_str!("boilerplate/LICENSE-MIT"),
        "LICENSE-MIT",
        start_year,
    )
}

fn generate_license(template: &str, filename: &str, start_year: i32) -> Result<(), Box<dyn Error>> {
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

pub fn generate_open_source_files(start_year: i32) -> Result<(), Box<dyn Error>> {
    generate_rustfmt_config()?;
    generate_workflow_script()?;
    generate_license_apache(start_year)?;
    generate_license_mit(start_year)?;

    Ok(())
}
