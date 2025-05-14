use std::process::{Command, Stdio};

use cargo_metadata::MetadataCommand;

use crate::clippy::ClippyArgs;
use crate::Backend;

#[derive(Debug, clap::Args)]
pub struct TidyArgs {
    /// do not abort running if we encounter an error
    /// while running the checks
    #[clap(long = "keep-going")]
    keep_going: bool,
}

impl TidyArgs {
    pub(crate) fn run(&self) {
        let mut success = true;
        let metadata = MetadataCommand::default().exec().unwrap();
        let mut command = Command::new("cargo");

        command
            .args(["fmt", "--all", "--check"])
            .current_dir(&metadata.workspace_root);

        println!("Check code formatting with `{command:?}`");
        let status = command
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .status()
            .unwrap();

        if !status.success() {
            println!("Code format issues detected!");
            println!("\t Run `cargo fmt --all` to format the source code");
            if !self.keep_going {
                std::process::exit(1);
            } else {
                success = false;
            }
        }

        let mut command = Command::new("typos");

        command.current_dir(metadata.workspace_root);
        println!("Check source code for spelling mistakes with `{command:?}`");

        let status = command
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .status()
            .unwrap();

        if !status.success() {
            println!("Spelling issues detected!");
            println!("\t Run `typos -w` to address some of the issues");
            if !self.keep_going {
                std::process::exit(1);
            } else {
                success = false;
            }
        }

        println!("Run clippy: ");
        ClippyArgs {
            backend: Backend::All,
            keep_going: self.keep_going,
            flags: Vec::new(),
        }
        .run();

        println!();
        if success {
            println!("All checks were successful. Ready to submit the code!");
        } else {
            std::process::exit(1);
        }
    }
}
