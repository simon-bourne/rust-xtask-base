use std::process::Output;

use handlebars::{Handlebars, RenderError};

mod handlebars_helpers {
    use std::fs;

    use handlebars::handlebars_helper;

    use super::run_process;

    handlebars_helper!(include: |file: str| { fs::read_to_string(file)? });
    handlebars_helper!(shell: |cmd: str| { run_process(cmd)? });
}

pub fn registry() -> Handlebars<'static> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);
    reg.register_helper("include", Box::new(handlebars_helpers::include));
    reg.register_helper("shell", Box::new(handlebars_helpers::shell));
    reg
}

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
