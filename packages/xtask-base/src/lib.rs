use std::{
    env::{current_dir, set_current_dir},
    error,
    ffi::OsString,
    fs,
    path::Path,
    process,
};

use cargo_metadata::{Metadata, MetadataCommand};
use chrono::{Datelike, Utc};
use ci::CI;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use duct::IntoExecutablePath;
use itertools::Itertools;
use scopeguard::defer;
use serde_json::json;

mod template;

pub mod ci;
pub mod github;

pub type WorkflowResult<T> = Result<T, Box<dyn error::Error>>;

#[derive(Parser)]
pub enum CommonCmds {
    /// Run CI checks
    Ci,
    /// Generate derived files. Existing content will be overritten.
    Codegen {
        /// Check the files wouldn't change. Don't actually generate them.
        #[clap(long)]
        check: bool,
    },
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
    /// Run common commands
    pub fn run(ci: CI, codegen: impl FnOnce(bool) -> WorkflowResult<()>) {
        in_workspace(|workspace| Self::parse().sub_command::<Self>(workspace, [], ci, codegen));
    }

    /// Run the subcommand for `self`
    pub fn sub_command<'a, T: CommandFactory>(
        &self,
        workspace: &Workspace,
        extra_workspace_dirs: impl IntoIterator<Item = &'a str>,
        ci: CI,
        codegen: impl FnOnce(bool) -> WorkflowResult<()>,
    ) -> WorkflowResult<()> {
        match self {
            CommonCmds::Ci => ci.execute(),
            CommonCmds::Codegen { check } => {
                generate_cargo_config(*check)?;
                ci.write(*check)?;
                codegen(*check)
            }
            CommonCmds::ShellCompletion { shell } => {
                let target_dir = workspace.target_dir();
                clap_complete::generate_to(*shell, &mut T::command(), "./cargo-xtask", target_dir)?;
                println!("Completions file generated in `{}`", target_dir.display());
                Ok(())
            }
            CommonCmds::Fmt => fmt(extra_workspace_dirs),
            CommonCmds::Udeps => cmd("cargo", ["+nightly", "udeps", "--all-targets"]),
            CommonCmds::MacroExpand { package } => {
                duct::cmd("cargo", ["expand", "--color=always", "--package", package])
                    .pipe(duct::cmd("less", ["-r"]))
                    .run()?;
                Ok(())
            }
        }
    }
}

fn fmt<'a>(extra_workspace_dirs: impl IntoIterator<Item = &'a str>) -> WorkflowResult<()> {
    for dir in extra_workspace_dirs {
        duct::cmd("cargo", ["+nightly", "fmt", "--all"])
            .dir(dir)
            .run()?;
    }

    cmd("cargo", ["+nightly", "fmt", "--all"])
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
pub fn in_workspace(f: impl FnOnce(&Workspace) -> WorkflowResult<()>) {
    try_in_workspace(f).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
}

fn try_in_workspace(f: impl FnOnce(&Workspace) -> WorkflowResult<()>) -> WorkflowResult<()> {
    let metadata = MetadataCommand::new().exec()?;

    let dir = current_dir()?;
    set_current_dir(&metadata.workspace_root)?;
    defer! {set_current_dir(dir).expect("Failed to reset current directory to {dir}")}

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
        dir.join("README.md"),
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
        fs::create_dir_all(".cargo")?;
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
    let path = path.as_ref();

    if check {
        // Ignore windows line endings
        let existing_contents = fs::read_to_string(path)?.lines().join("\n");

        if existing_contents != contents.lines().join("\n") {
            return Err(format!("Differences found in file \"{}\"", path.display()).into());
        }
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, contents)?;
    }

    Ok(())
}

fn cmd<T, U>(program: T, args: U) -> WorkflowResult<()>
where
    T: IntoExecutablePath,
    U: IntoIterator,
    U::Item: Into<OsString>,
{
    duct::cmd(program, args).run()?;
    Ok(())
}
