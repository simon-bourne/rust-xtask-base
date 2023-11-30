use crate::{
    github::actions::{
        self, cmd, install, install_rust, pull_request, push, rust_toolchain, script, Event,
        Platform, Run, Rust, Step, Workflow,
    },
    WorkflowResult,
};

pub struct CI {
    name: String,
    triggers: Vec<Event>,
    tasks: Vec<Tasks>,
}

impl CI {
    /// Create a new CI workflow called "tests", that triggers on any "push"
    /// or "pull_request".
    pub fn new() -> Self {
        Self {
            name: "tests".to_owned(),
            triggers: vec![push().into(), pull_request().into()],
            tasks: Vec::new(),
        }
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            triggers: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// `extra_workspaces` is a tuple of (name, dir).
    pub fn standard_workflow(
        versions: StandardVersions,
        extra_workspaces: &[(&str, &str)],
    ) -> Self {
        Self::new()
            .standard_tests(versions.rustc_stable_version, extra_workspaces)
            .standard_release_tests(versions.rustc_stable_version, extra_workspaces)
            .standard_lints(
                versions.rustc_nightly_version,
                versions.udeps_version,
                extra_workspaces,
            )
    }

    /// `extra_workspaces` is a tuple of (name, dir).
    pub fn standard_lints(
        self,
        rustc_version: &str,
        udeps_version: &str,
        extra_workspaces: &[(&str, &str)],
    ) -> Self {
        self.job(
            Tasks::new(
                "lints",
                Platform::UbuntuLatest,
                rust_toolchain(rustc_version).minimal().default().rustfmt(),
            )
            .lints(
                udeps_version,
                &extra_workspaces
                    .iter()
                    .copied()
                    .map(|(_name, dir)| dir)
                    .collect::<Vec<_>>(),
            ),
        )
    }

    /// `extra_workspaces` is a tuple of (name, dir).
    pub fn standard_tests(
        mut self,
        rustc_version: &str,
        extra_workspaces: &[(&str, &str)],
    ) -> Self {
        for platform in Platform::latest() {
            self.tasks.push(
                Tasks::new(
                    "tests",
                    platform,
                    rust_toolchain(rustc_version).minimal().default().clippy(),
                )
                .codegen()
                .tests(None),
            );

            for (name, workspace_dir) in extra_workspaces {
                self.tasks.push(
                    Tasks::new(
                        &format!("tests-{name}"),
                        platform,
                        rust_toolchain(rustc_version).minimal().default().clippy(),
                    )
                    .tests(Some(workspace_dir)),
                );
            }
        }

        self
    }

    /// `extra_workspaces` is a tuple of (name, dir).
    pub fn standard_release_tests(
        mut self,
        rustc_version: &str,
        extra_workspaces: &[(&str, &str)],
    ) -> Self {
        for platform in Platform::latest() {
            self.tasks.push(
                Tasks::new(
                    "release-tests",
                    platform,
                    rust_toolchain(rustc_version).minimal().default(),
                )
                .release_tests(None),
            );

            for (name, dir) in extra_workspaces {
                self.tasks.push(
                    Tasks::new(
                        &format!("release-tests-{name}"),
                        platform,
                        rust_toolchain(rustc_version).minimal().default(),
                    )
                    .release_tests(Some(dir)),
                );
            }
        }

        self
    }

    pub fn on(mut self, event: impl Into<Event>) -> Self {
        self.triggers.push(event.into());
        self
    }

    pub fn job(mut self, tasks: Tasks) -> Self {
        self.add_job(tasks);
        self
    }

    pub fn add_job(&mut self, tasks: Tasks) {
        self.tasks.push(tasks);
    }

    pub fn write(self, check: bool) -> WorkflowResult<()> {
        self.into_workflow().write(check)
    }

    pub fn execute(self) -> WorkflowResult<()> {
        for task in self.tasks {
            task.execute()?;
        }

        Ok(())
    }

    fn into_workflow(self) -> Workflow {
        let mut workflow = actions::workflow(&self.name).on(self.triggers);

        for task in self.tasks {
            workflow.add_job(
                &task.name,
                task.platform,
                task.tasks.into_iter().map(Step::from),
            );
        }

        workflow
    }
}

impl Default for CI {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StandardVersions<'a> {
    pub rustc_stable_version: &'a str,
    pub rustc_nightly_version: &'a str,
    pub udeps_version: &'a str,
}

impl Default for StandardVersions<'static> {
    fn default() -> Self {
        Self {
            rustc_stable_version: "1.73",
            rustc_nightly_version: "nightly-2023-10-14",
            udeps_version: "0.1.43",
        }
    }
}

pub struct Tasks {
    name: String,
    platform: Platform,
    is_nightly: bool,
    tasks: Vec<Task>,
}

impl Tasks {
    pub fn new(name: impl Into<String>, platform: Platform, rust: Rust) -> Self {
        Self {
            name: name.into(),
            platform,
            is_nightly: rust.is_nightly(),
            tasks: Vec::new(),
        }
        .step(install_rust(rust))
    }

    pub fn execute(self) -> WorkflowResult<()> {
        if self.platform.is_current() {
            for task in self.tasks.into_iter() {
                if let Task::Run(cmd) = task {
                    cmd.rustup_run(self.is_nightly)?;
                }
            }
        }

        Ok(())
    }

    pub fn step(mut self, step: impl Into<Step>) -> Self {
        self.add_step(step);
        self
    }

    pub fn add_step(&mut self, step: impl Into<Step>) {
        self.tasks.push(Task::Install(step.into()));
    }

    pub fn step_when(self, condition: bool, step: impl Into<Step>) -> Self {
        self.when(condition, Self::step, step)
    }

    pub fn run(mut self, run: impl Into<Run>) -> Self {
        self.add_run(run);
        self
    }

    pub fn add_run(&mut self, run: impl Into<Run>) {
        self.tasks.push(Task::Run(run.into()))
    }

    pub fn run_when(self, condition: bool, run: impl Into<Run>) -> Self {
        self.when(condition, Self::run, run)
    }

    pub fn cmd(
        self,
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        self.run(cmd(program, args))
    }

    pub fn add_cmd(
        &mut self,
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) {
        self.add_run(cmd(program, args));
    }

    pub fn script<Cmds, Cmd, Arg>(self, cmds: Cmds) -> Self
    where
        Cmds: IntoIterator<Item = Cmd>,
        Cmd: IntoIterator<Item = Arg>,
        Arg: AsRef<str>,
    {
        self.run(script(cmds))
    }

    pub fn add_script<Cmds, Cmd, Arg>(&mut self, cmds: Cmds)
    where
        Cmds: IntoIterator<Item = Cmd>,
        Cmd: IntoIterator<Item = Arg>,
        Arg: AsRef<str>,
    {
        self.add_run(script(cmds));
    }

    pub fn apply<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }

    pub fn codegen(self) -> Self {
        self.cmd("cargo", ["xtask", "codegen", "--check"])
    }

    pub fn tests(mut self, workspace_dir: Option<&str>) -> Self {
        let tests = || {
            [
                cmd(
                    "cargo",
                    [
                        "clippy",
                        "--all-targets",
                        "--",
                        "-D",
                        "warnings",
                        "-D",
                        "clippy::all",
                    ],
                ),
                cmd("cargo", ["test"]),
                cmd("cargo", ["build", "--all-targets"]),
                cmd("cargo", ["doc"]),
            ]
        };

        if let Some(dir) = workspace_dir {
            tests().map(|run| self.add_run(run.dir(dir)));
        } else {
            tests().map(|run| self.add_run(run));
        }

        self
    }

    pub fn release_tests(mut self, workspace_dir: Option<&str>) -> Self {
        let test = || cmd("cargo", ["test", "--benches", "--tests", "--release"]);

        if let Some(dir) = workspace_dir {
            self.add_run(test().dir(dir));
        } else {
            self.add_run(test());
        }

        self
    }

    pub fn lints(mut self, udeps_version: &str, extra_workspace_dirs: &[&str]) -> Self {
        let fmt = || cmd("cargo", ["fmt", "--all", "--", "--check"]);
        let udeps = || cmd("cargo", ["udeps", "--all-targets"]);

        self.add_run(fmt());

        for dir in extra_workspace_dirs {
            self.add_run(fmt().dir(dir));
        }

        self.add_step(install("cargo-udeps", udeps_version));

        self.add_run(udeps());

        for dir in extra_workspace_dirs {
            self.add_run(udeps().dir(dir));
        }

        self
    }

    fn when<T>(self, condition: bool, f: impl FnOnce(Self, T) -> Self, x: T) -> Self {
        if condition {
            f(self, x)
        } else {
            self
        }
    }
}

enum Task {
    Install(Step),
    Run(Run),
}

impl From<Task> for Step {
    fn from(value: Task) -> Self {
        match value {
            Task::Install(step) => step,
            Task::Run(cmd) => cmd.into(),
        }
    }
}
