use std::{fmt, path::PathBuf};

use crate::{update_file, WorkflowResult};

pub fn install_rust(rust: Rust) -> Step {
    Step(StepEnum::Multi(
        [checkout(), rust.into(), rust_cache()]
            .into_iter()
            .collect(),
    ))
}

#[must_use]
pub struct Workflow {
    name: String,
    triggers: Vec<Event>,
    jobs: Vec<Job>,
}

pub fn workflow(name: &str) -> Workflow {
    Workflow {
        name: name.to_string(),
        triggers: Vec::new(),
        jobs: Vec::new(),
    }
}

impl Workflow {
    pub fn on(mut self, events: impl IntoIterator<Item = Event>) -> Self {
        self.triggers.extend(events);
        self
    }

    pub fn add_job(
        &mut self,
        name: &str,
        runs_on: Platform,
        steps: impl IntoIterator<Item = impl Into<Step>>,
    ) {
        self.jobs.push(Job::new(name, runs_on, steps));
    }

    pub fn job(
        mut self,
        name: &str,
        runs_on: Platform,
        steps: impl IntoIterator<Item = impl Into<Step>>,
    ) -> Self {
        self.add_job(name, runs_on, steps);
        self
    }

    pub fn write(&self, check: bool) -> WorkflowResult<()> {
        update_file(
            [".github", "workflows", &format!("{}.yml", self.name)]
                .into_iter()
                .collect::<PathBuf>(),
            &self.to_string(),
            check,
        )
    }
}

impl fmt::Display for Workflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "name: {}", self.name)?;
        writeln!(
            f,
            "on: [{}]",
            self.triggers
                .iter()
                .map(|ev| ev.0)
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        f.write_str("jobs:\n")?;

        for job in &self.jobs {
            job.fmt(f)?;
        }
        Ok(())
    }
}

struct Job {
    name: String,
    runs_on: Platform,
    steps: Vec<Step>,
}

impl Job {
    fn new(
        name: &str,
        runs_on: Platform,
        steps: impl IntoIterator<Item = impl Into<Step>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            runs_on,
            steps: steps.into_iter().map(Into::into).collect(),
        }
    }
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let runs_on = self.runs_on.as_str();
        writeln!(f, "  {}-{}:", self.name, runs_on)?;
        writeln!(f, "    runs-on: {}", runs_on)?;
        f.write_str("    steps:\n")?;

        for step in &self.steps {
            step.fmt(f)?;
        }

        Ok(())
    }
}

pub struct Event(&'static str);

pub fn push() -> Event {
    Event("push")
}

pub fn pull_request() -> Event {
    Event("pull_request")
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Platform {
    UbuntuLatest,
    MacOSLatest,
    WindowsLatest,
}

impl Platform {
    pub fn latest() -> impl Iterator<Item = Self> {
        [
            Platform::UbuntuLatest,
            Platform::MacOSLatest,
            Platform::WindowsLatest,
        ]
        .into_iter()
    }

    fn as_str(self) -> &'static str {
        match self {
            Platform::UbuntuLatest => "ubuntu-latest",
            Platform::MacOSLatest => "macos-latest",
            Platform::WindowsLatest => "windows-latest",
        }
    }
}

pub struct Action {
    uses: String,
    with: Vec<(String, String)>,
}

impl Action {
    pub fn with(mut self, key: &str, value: impl fmt::Display) -> Self {
        self.add_with(key, value);
        self
    }

    pub fn add_with(&mut self, key: &str, value: impl fmt::Display) {
        self.with.push((key.to_string(), value.to_string()));
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "    - uses: {}", self.uses)?;

        if !self.with.is_empty() {
            f.write_str("      with:\n")?;

            for (key, value) in &self.with {
                writeln!(f, "        {key}: {value}")?;
            }
        }

        Ok(())
    }
}

pub fn action(uses: &str) -> Action {
    Action {
        uses: uses.to_string(),
        with: Vec::new(),
    }
}

pub fn checkout() -> Step {
    action("actions/checkout@v3").into()
}

impl From<Action> for Step {
    fn from(value: Action) -> Self {
        Step(StepEnum::Action(value))
    }
}

pub struct Step(StepEnum);

pub fn multi_step(steps: impl IntoIterator<Item = impl Into<Step>>) -> Step {
    Step(StepEnum::Multi(steps.into_iter().map(Into::into).collect()))
}

impl Step {
    pub fn if_failed(self) -> Self {
        self
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            StepEnum::Empty => (),
            StepEnum::Multi(steps) => {
                for step in steps {
                    step.fmt(f)?;
                }
            }
            StepEnum::Action(action) => action.fmt(f)?,
            StepEnum::Run(run) => run.fmt(f)?,
        }

        Ok(())
    }
}

enum StepEnum {
    Empty,
    Multi(Vec<Step>),
    Action(Action),
    Run(Run),
}

pub fn upload_artifact(name: &str, path: &str) -> Step {
    action("actions/upload-artifact@v3")
        .with("name", name)
        .with("path", path)
        .into()
}

pub fn rust_cache() -> Step {
    action("Swatinem/rust-cache@v2").into()
}

pub fn install(crate_name: &str, version: &str) -> Step {
    action("actions-rs/install@v0.1")
        .with("crate", crate_name)
        .with("version", version)
        .into()
}

pub struct Rust {
    toolchain: String,
    profile: Option<&'static str>,
    default: bool,
    components: Vec<&'static str>,
    target: Option<String>,
}

pub fn rust_toolchain(version: &str) -> Rust {
    Rust {
        toolchain: version.to_string(),
        profile: None,
        default: false,
        components: Vec::new(),
        target: None,
    }
}

impl Rust {
    pub fn wasm(mut self) -> Self {
        self.target = Some("wasm32-unknown-unknown".to_string());
        self
    }

    pub fn minimal(mut self) -> Self {
        self.profile = Some("minimal");
        self
    }

    pub fn default(mut self) -> Self {
        self.default = true;
        self
    }

    pub fn clippy(mut self) -> Self {
        self.components.push("clippy");
        self
    }

    pub fn rustfmt(mut self) -> Self {
        self.components.push("rustfmt");
        self
    }
}

impl From<Rust> for Step {
    fn from(value: Rust) -> Self {
        let mut action = action("actions-rs/toolchain@v1").with("toolchain", value.toolchain);

        if let Some(profile) = value.profile {
            action.add_with("profile", profile);
        }

        if value.default {
            action.add_with("default", value.default);
        }

        if !value.components.is_empty() {
            action.add_with("components", value.components.join(", "));
        }

        if let Some(target) = value.target {
            action.add_with("target", target);
        }

        Step(StepEnum::Action(action))
    }
}

pub struct Run {
    script: RunEnum,
    directory: Option<String>,
}

pub fn run(cmd: &str) -> Run {
    Run {
        script: RunEnum::Single(cmd.into()),
        directory: None,
    }
}

impl Run {
    pub fn lines(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Run {
            script: RunEnum::Multi(lines.into_iter().map(Into::into).collect()),
            directory: None,
        }
    }

    pub fn in_directory(mut self, directory: &str) -> Self {
        self.directory = Some(directory.to_string());
        self
    }
}

impl fmt::Display for Run {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(directory) = &self.directory {
            writeln!(f, "    - working-directory: {directory}")?;
        }

        match &self.script {
            RunEnum::Single(cmd) => writeln!(f, "      run: {cmd}")?,
            RunEnum::Multi(multi) => {
                f.write_str("      run: |\n")?;

                for cmd in multi {
                    writeln!(f, "        {cmd}")?;
                }
            }
        }

        Ok(())
    }
}

impl From<Run> for Step {
    fn from(value: Run) -> Self {
        Self(StepEnum::Run(value))
    }
}

enum RunEnum {
    Single(String),
    Multi(Vec<String>),
}

pub fn when(condition: bool, step: impl Into<Step>) -> Step {
    if condition {
        step.into()
    } else {
        Step(StepEnum::Empty)
    }
}
