use std::fmt::{Debug, Formatter, Error};
use std::path::{PathBuf, Path};
use std::process::{Command, Output};
use std::{env, str};

pub struct TestCommand {
    cwd: PathBuf,
    args: Vec<String>,
    env_vars: Vec<(String, String)>,
}

impl TestCommand {
    pub fn new(cwd: &Path, subcommand: &str) -> Self {
        TestCommand {
            cwd: cwd.into(),
            args: vec![subcommand.into()],
            env_vars: Vec::new(),
        }
    }

    pub fn arg<S: Into<String>>(mut self, value: S) -> Self {
        self.args.push(value.into());
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    pub fn run(self) -> CommandResult {
        let output = self.build_command().output().unwrap();
        CommandResult {
            output: output,
        }
    }

    fn build_command(&self) -> Command {
        let mut command = Command::new(path_to_diesel_cli());
        command.args(&self.args)
            .current_dir(&self.cwd);
        for &(ref k, ref v) in self.env_vars.iter() {
            command.env(&k, &v);
        }
        command
    }
}

pub struct CommandResult {
    output: Output,
}

impl CommandResult {
    pub fn is_success(&self) -> bool {
        self.output.status.success()
    }

    pub fn stdout(&self) -> &str {
        str::from_utf8(&self.output.stdout).unwrap()
    }

    pub fn stderr(&self) -> &str {
        str::from_utf8(&self.output.stderr).unwrap()
    }

    pub fn code(&self) -> i32 {
        self.output.status.code().unwrap()
    }
}

fn path_to_diesel_cli() -> PathBuf {
    env::current_exe().unwrap()
        .parent().unwrap()
        .join("diesel")
}


impl Debug for CommandResult {
    fn fmt(&self, out: &mut Formatter) -> Result<(), Error> {
        write!(out, "stdout: {}\nstderr: {}", self.stdout(), self.stderr())
    }
}
