use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempdir::TempDir;

use super::command::TestCommand;

pub fn project(name: &str) -> ProjectBuilder {
    ProjectBuilder::new(name)
}

pub struct ProjectBuilder {
    name: String,
    folders: Vec<String>,
}

impl ProjectBuilder {
    fn new(name: &str) -> Self {
        ProjectBuilder {
            name: name.into(),
            folders: Vec::new(),
        }
    }

    pub fn folder(mut self, name: &str) -> Self {
        self.folders.push(name.into());
        self
    }

    pub fn build(self) -> Project {
        let tempdir = TempDir::new(&self.name).unwrap();

        File::create(tempdir.path().join("Cargo.toml")).unwrap();

        for folder in self.folders {
            fs::create_dir(tempdir.path().join(folder))
                .unwrap();
        }

        Project {
            directory: tempdir,
            name: self.name,
        }
    }
}

pub struct Project {
    directory: TempDir,
    name: String,
}

impl Project {
    pub fn command(&self, name: &str) -> TestCommand {
        TestCommand::new(self.directory.path(), name)
            .env("DATABASE_URL", &self.database_url())
    }

    pub fn migrations(&self) -> Vec<Migration> {
        self.directory.path().join("migrations")
            .read_dir().expect("Error reading directory")
            .map(|e| Migration {
                path: e.expect("error reading entry").path().into(),
            })
            .collect()
    }

    #[cfg(feature = "postgres")]
    pub fn database_url(&self) -> String {
        format!("postgres://localhost/{}", self.name)
    }

    #[cfg(feature = "sqlite")]
    pub fn database_url(&self) -> String {
        self.directory.path().join(&self.name)
            .into_os_string()
            .into_string().unwrap()
    }
}

#[cfg(feature = "postgres")]
impl Drop for Project {
    fn drop(&mut self) {
        use std::io::{self, Write};
        use std::thread;

        let result = self.command("database").arg("drop").run();
        if !result.is_success() {
            if thread::panicking() {
                writeln!(io::stderr(), "Couldn't drop database: {:?}", result).unwrap();
            } else {
                panic!("Couldn't drop database: {:?}", result);
            }
        }
    }
}

pub struct Migration {
    path: PathBuf,
}

impl Migration {
    pub fn version(&self) -> &str {
        &self.file_name()[..14]
    }

    pub fn name(&self) -> &str {
        &self.file_name()[15..]
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn file_name(&self) -> &str {
        self.path.file_name().expect("migration should have a file name")
            .to_str().expect("Directory was not valid UTF-8")
    }
}
