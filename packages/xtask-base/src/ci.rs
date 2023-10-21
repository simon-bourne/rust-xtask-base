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
    /// Create a new CI workflow called "ci-tests", that triggers on any "push"
    /// or "pull_request".
    pub fn new() -> Self {
        Self {
            name: "ci-tests".to_owned(),
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

    pub fn standard_workflow(extra_workspace_dirs: &[&str]) -> Self {
        let rustc_stable_version = "1.73";
        let rustc_nightly_version = "nightly-2023-10-14";
        let udeps_version = "0.1.43";

        Self::new()
            .standard_tests(rustc_stable_version, extra_workspace_dirs)
            .standard_release_tests(rustc_stable_version, extra_workspace_dirs)
            .standard_lints(rustc_nightly_version, udeps_version, extra_workspace_dirs)
    }

    pub fn standard_lints(
        self,
        rustc_version: &str,
        udeps_version: &str,
        extra_workspace_dirs: &[&str],
    ) -> Self {
        self.job(
            Tasks::new(
                "lints",
                Platform::UbuntuLatest,
                rust_toolchain(rustc_version).minimal().default().rustfmt(),
            )
            .lints(udeps_version, extra_workspace_dirs),
        )
    }

    pub fn standard_tests(mut self, rustc_version: &str, extra_workspace_dirs: &[&str]) -> Self {
        for platform in Platform::latest() {
            self.tasks.push(
                Tasks::new(
                    "tests",
                    platform,
                    rust_toolchain(rustc_version).minimal().default().clippy(),
                )
                .tests(extra_workspace_dirs),
            );
        }

        self
    }

    pub fn standard_release_tests(
        mut self,
        rustc_version: &str,
        extra_workspace_dirs: &[&str],
    ) -> Self {
        for platform in Platform::latest() {
            self.tasks.push(
                Tasks::new(
                    "release-tests",
                    platform,
                    rust_toolchain(rustc_version).minimal().default(),
                )
                .release_tests(extra_workspace_dirs),
            );
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

pub struct Tasks {
    name: String,
    platform: Platform,
    is_nightly: bool,
    tasks: Vec<Task>,
}

impl Tasks {
    pub fn new(name: impl Into<String>, platform: Platform, rust: Rust) -> Self {
        Self::empty(name, platform, rust.is_nightly()).step(install_rust(rust))
    }

    // TODO: The `is-nightly` flag is a bit horrible.
    pub fn empty(name: impl Into<String>, platform: Platform, is_nightly: bool) -> Self {
        Self {
            name: name.into(),
            platform,
            is_nightly,
            tasks: Vec::new(),
        }
    }

    pub fn execute(self) -> WorkflowResult<()> {
        if self.platform.is_current() {
            for task in self.tasks.into_iter() {
                if let Task::Run(cmd) = task {
                    cmd.run(self.is_nightly)?;
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

    pub fn run(mut self, run: impl Into<Run>) -> Self {
        self.add_run(run);
        self
    }

    pub fn add_run(&mut self, run: impl Into<Run>) {
        self.tasks.push(Task::Run(run.into()))
    }

    pub fn cmd(
        self,
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.run(cmd(program, args))
    }

    pub fn add_cmd(
        &mut self,
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) {
        self.add_run(cmd(program, args));
    }

    pub fn script<Cmds, Cmd, Arg>(self, cmds: Cmds) -> Self
    where
        Cmds: IntoIterator<Item = Cmd>,
        Cmd: IntoIterator<Item = Arg>,
        Arg: Into<String>,
    {
        self.run(script(cmds))
    }

    pub fn add_script<Cmds, Cmd, Arg>(&mut self, cmds: Cmds)
    where
        Cmds: IntoIterator<Item = Cmd>,
        Cmd: IntoIterator<Item = Arg>,
        Arg: Into<String>,
    {
        self.add_run(script(cmds));
    }

    pub fn tests(mut self, extra_workspace_dirs: &[&str]) -> Self {
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

        self.add_cmd("cargo", ["xtask", "codegen", "--check"]);
        tests().map(|run| self.add_run(run));

        for dir in extra_workspace_dirs {
            tests().map(|run| self.add_run(run.in_directory(dir)));
        }

        self
    }

    pub fn release_tests(mut self, extra_workspace_dirs: &[&str]) -> Self {
        let test = || cmd("cargo", ["test", "--benches", "--tests", "--release"]);
        self.add_run(test());

        for dir in extra_workspace_dirs {
            self.add_run(test().in_directory(dir));
        }

        self
    }

    pub fn lints(mut self, udeps_version: &str, extra_workspace_dirs: &[&str]) -> Self {
        let fmt = || cmd("cargo", ["fmt", "--all", "--", "--check"]);
        let udeps = || cmd("cargo", ["udeps", "--all-targets"]);

        self.add_run(fmt());

        for dir in extra_workspace_dirs {
            self.add_run(fmt().in_directory(dir));
        }

        self.add_step(install("cargo-udeps", udeps_version));

        self.add_run(udeps());

        for dir in extra_workspace_dirs {
            self.add_run(udeps().in_directory(dir));
        }

        self
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
