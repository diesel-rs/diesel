use std::fmt::Display;
use std::process::{Command, Stdio};

use cargo_metadata::MetadataCommand;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum SemverType {
    Patch,
    Minor,
    Major,
}

impl Display for SemverType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemverType::Patch => write!(f, "patch"),
            SemverType::Minor => write!(f, "minor"),
            SemverType::Major => write!(f, "major"),
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct SemverArgs {
    /// Type of the next release
    #[clap(long = "type", default_value_t = SemverType::Minor)]
    tpe: SemverType,
    /// Baseline version to check against. If that's not
    /// set the version is inferred from the current package version
    #[clap(long = "baseline-version")]
    baseline_version: Option<String>,
}

impl SemverArgs {
    pub fn run(&self) {
        let metadata = MetadataCommand::default().exec().unwrap();
        self.run_semver_checks_for(
            &metadata,
            "diesel",
            &["sqlite", "postgres", "mysql", "extras"],
        );
        self.run_semver_checks_for(&metadata, "diesel_migrations", &[]);
        self.run_semver_checks_for(
            &metadata,
            "diesel_dynamic_schema",
            &["postgres", "mysql", "sqlite"],
        );
    }

    fn run_semver_checks_for(
        &self,
        metadata: &cargo_metadata::Metadata,
        crate_name: &str,
        features: &[&str],
    ) {
        let baseline_diesel_version = if let Some(ref baseline_version) = self.baseline_version {
            baseline_version.clone()
        } else {
            let mut baseline_diesel_version = metadata
                .packages
                .iter()
                .find_map(|c| (c.name == "diesel").then_some(&c.version))
                .unwrap()
                .clone();
            baseline_diesel_version.patch = 0;
            baseline_diesel_version.to_string()
        };
        let mut command = Command::new("cargo");
        command
            .args([
                "semver-checks",
                "-p",
                crate_name,
                "--only-explicit-features",
                "--baseline-version",
                &baseline_diesel_version,
                "--release-type",
                &self.tpe.to_string(),
            ])
            .current_dir(&metadata.workspace_root);
        for f in features {
            command.args(["--features", f]);
        }
        println!("Run cargo semver-checks via `{command:?}`");
        let res = command
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .status()
            .unwrap()
            .success();

        if !res {
            eprintln!("Cargo semver check failed");
            std::process::exit(1);
        }
    }
}
