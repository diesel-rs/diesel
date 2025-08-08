use std::collections::HashMap;
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
        let false_positives_for_diesel = HashMap::from([
            (
                "inherent_method_missing",
                // this method was not supposed to be public at all
                &["SerializedDatabase::new"] as &[_],
            ),
            (
                "trait_added_supertrait",
                // false positive as cargo semver-checks does not perform trait solving
                // https://github.com/obi1kenobi/cargo-semver-checks/issues/1265
                &["trait diesel::connection::Instrumentation gained Downcast"] as &[_],
            ),
            (
                "trait_no_longer_dyn_compatible",
                // That's technically true, but
                // noone is able to meaningful use these
                // traits as trait object as they are only marker
                // traits, so it's "fine" to break that
                &[
                    "trait SqlOrd",
                    "trait Foldable",
                    "trait SqlType",
                    "trait SingleValue",
                ] as &[_],
            ),
        ]);
        self.run_semver_checks_for(
            &metadata,
            "diesel",
            &["sqlite", "postgres", "mysql", "extras", "with-deprecated"],
            false_positives_for_diesel,
        );
        self.run_semver_checks_for(&metadata, "diesel_migrations", &[], HashMap::new());
        self.run_semver_checks_for(
            &metadata,
            "diesel-dynamic-schema",
            &["postgres", "mysql", "sqlite"],
            HashMap::new(),
        );
    }

    fn run_semver_checks_for(
        &self,
        metadata: &cargo_metadata::Metadata,
        crate_name: &str,
        features: &[&str],
        allow_list: HashMap<&str, &[&str]>,
    ) {
        let baseline_diesel_version = if let Some(ref baseline_version) = self.baseline_version {
            baseline_version.clone()
        } else {
            let mut baseline_diesel_version = metadata
                .packages
                .iter()
                .find_map(|c| (c.name == crate_name).then_some(&c.version))
                .unwrap()
                .clone();
            if baseline_diesel_version.major != 0 {
                baseline_diesel_version.patch = 0;
            }
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
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        let std_out = String::from_utf8(res.stdout).expect("Valid UTF-8");
        let std_err = String::from_utf8(res.stderr).expect("Valid UTF-8");
        let mut failed = false;
        let mut std_out_out = String::new();
        // That's all here as we want to "patch" the output of cargo-semver-checks
        // to be able to ignore specific instances of lint violations because:
        //
        // * There are a lot of false positives
        // * Diesel is complex
        // * Sometimes we want to do things that are technically breaking changes
        for lint in std_out.split("\n---") {
            if lint.trim().is_empty() {
                continue;
            }
            let (lint, content) = lint
                .trim()
                .strip_prefix("failure")
                .unwrap_or(lint)
                .trim()
                .split_once(':')
                .expect("Two parts exist");
            let ignore_list = allow_list.get(lint).copied().unwrap_or_default();

            let failures = content
                .lines()
                .skip_while(|l| !l.trim().starts_with("Failed in:"))
                .skip(1)
                .filter(|l| ignore_list.iter().all(|e| !l.trim().starts_with(e)))
                .collect::<Vec<_>>();
            let content = content
                .lines()
                .take_while(|l| !l.trim().starts_with("Failed in:"));
            if !failures.is_empty() {
                failed = true;
                if !std_out_out.is_empty() {
                    std_out_out += "\n";
                }
                std_out_out += "--- failure ";
                std_out_out += lint;
                std_out_out += ":";
                for l in content {
                    std_out_out += l;
                    std_out_out += "\n";
                }
                std_out_out += "Failed in:";
                for failure in failures {
                    std_out_out += "\n";
                    std_out_out += failure;
                }
            }
        }
        let (front, back) = std_err.split_once("\n\n").unwrap_or((&std_err, ""));
        eprintln!("{front}\n");
        if failed {
            eprintln!("Cargo semver check failed");
            println!("{std_out_out}");
            println!();
        }
        eprintln!("{back}");
        if failed {
            std::process::exit(1);
        }
    }
}
