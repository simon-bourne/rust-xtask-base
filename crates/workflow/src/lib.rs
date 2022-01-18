use std::{
    error::Error,
    fs::{self, File},
    path::Path,
    process::Output,
};

use chrono::{Datelike, Utc};
use clap::{FromArgMatches, IntoApp};

pub fn from_args<T: IntoApp + FromArgMatches>() -> T {
    T::from_arg_matches(&T::into_app().get_matches()).unwrap()
}

use handlebars::{handlebars_helper, Handlebars, RenderError};
use serde_json::json;

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

pub fn generate_rustfmt_config() -> Result<(), Box<dyn Error>> {
    let rustfmt = include_str!("boilerplate/rustfmt.toml");
    fs::write("rustfmt.toml", rustfmt)?;

    Ok(())
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
    generate_license_apache(start_year)?;
    generate_license_mit(start_year)?;

    Ok(())
}
