use std::process::{Command, Stdio};

use cargo_metadata::{Metadata, MetadataCommand};

use crate::Backend;

#[derive(Debug, clap::Args)]
pub struct ClippyArgs {
    /// Run clippy for a specific backend
    #[clap(default_value_t = Backend::All)]
    pub backend: Backend,
    /// do not abort running if we encounter an error
    /// while running clippy for all backends
    #[clap(long = "keep-going")]
    pub keep_going: bool,
    /// additional flags passed to cargo clippy
    ///
    /// This is useful for passing custom arguments like `cargo clippy --fix`
    pub flags: Vec<String>,
}

impl ClippyArgs {
    pub(crate) fn run(&self) {
        let metadata = MetadataCommand::default().exec().unwrap();
        let failed = if matches!(self.backend, Backend::All) {
            let mut failed = false;
            for backend in Backend::ALL {
                if !self.run_for_backend(*backend, &metadata) {
                    failed = true;
                    if !self.keep_going {
                        break;
                    }
                }
            }
            failed
        } else {
            !self.run_for_backend(self.backend, &metadata)
        };
        if failed {
            std::process::exit(1);
        }
    }

    fn run_for_backend(&self, backend: Backend, metadata: &Metadata) -> bool {
        let exclude = crate::utils::get_exclude_for_backend(&backend.to_string(), metadata, false);
        let flags = [
            "-F".into(),
            format!("diesel/{backend}"),
            "-F".into(),
            format!("diesel_derives/{backend}"),
            "-F".into(),
            format!("diesel_cli/{backend}"),
            "-F".into(),
            format!("diesel_tests/{backend}"),
            "-F".into(),
            format!("diesel-dynamic-schema/{backend}"),
        ];
        let mut command = Command::new("cargo");

        command
            .args([
                "clippy",
                "--workspace",
                "--no-default-features",
                "--all-targets",
            ])
            .args(exclude)
            .args(flags)
            .args([
                "-F",
                "diesel/extras",
                "-F",
                "diesel/with-deprecated",
                "-F",
                "diesel_derives/numeric",
                "-F",
                "diesel_derives/chrono",
                "-F",
                "diesel_derives/time",
            ])
            .args(&self.flags)
            .current_dir(&metadata.workspace_root);

        println!("Run clippy via `{command:?}`");
        command
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .status()
            .unwrap()
            .success()
    }
}
