use std::fmt::{Debug, Error, Formatter};
use std::path::{Path, PathBuf};
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

    pub fn args<I>(self, values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        values.into_iter().fold(self, |c, value| c.arg(value))
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    pub fn cd<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.cwd.push(path);
        self
    }

    pub fn run(self) -> CommandResult {
        let output = self
            .build_command()
            .output()
            .expect("failed to execute process");
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        CommandResult { output }
    }

    fn build_command(&self) -> Command {
        let mut command = Command::new(path_to_diesel_cli());
        command.args(&self.args).current_dir(&self.cwd);
        for (k, v) in self.env_vars.iter() {
            command.env(k, v);
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

    #[allow(dead_code)]
    pub fn result(self) -> Result<Self, Self> {
        if self.is_success() {
            Ok(self)
        } else {
            Err(self)
        }
    }
}

fn path_to_diesel_cli() -> PathBuf {
    if let Some(path) = env::var_os("DIESEL_TEST_BIN") {
        Path::new(&path).into()
    } else {
        Path::new(&env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .join("target")
            .join("debug")
            .join("diesel")
    }
}

impl Debug for CommandResult {
    fn fmt(&self, out: &mut Formatter) -> Result<(), Error> {
        write!(out, "stdout: {}\nstderr: {}", self.stdout(), self.stderr())
    }
}
