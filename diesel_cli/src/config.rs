use clap::ArgMatches;
use std::env;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::find_project_root;
use crate::print_schema;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub print_schema: PrintSchema,
    #[serde(default)]
    pub migrations_directory: Option<MigrationsDirectory>,
}

impl Config {
    pub fn file_path(matches: &ArgMatches) -> PathBuf {
        matches
            .value_of("CONFIG_FILE")
            .map(PathBuf::from)
            .or_else(|| env::var_os("DIESEL_CONFIG_FILE").map(PathBuf::from))
            .unwrap_or_else(|| find_project_root().unwrap_or_default().join("diesel.toml"))
    }

    pub fn read(matches: &ArgMatches) -> Result<Self, Box<dyn Error>> {
        let path = Self::file_path(matches);

        if path.exists() {
            let mut bytes = Vec::new();
            fs::File::open(&path)?.read_to_end(&mut bytes)?;
            let mut result = toml::from_slice::<Self>(&bytes)?;
            result.set_relative_path_base(path.parent().unwrap());
            Ok(result)
        } else {
            Ok(Self::default())
        }
    }

    fn set_relative_path_base(&mut self, base: &Path) {
        self.print_schema.set_relative_path_base(base)
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrintSchema {
    #[serde(default)]
    pub file: Option<PathBuf>,
    #[serde(default)]
    pub with_docs: bool,
    #[serde(default)]
    pub filter: print_schema::Filtering,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub patch_file: Option<PathBuf>,
    #[serde(default)]
    pub import_types: Option<Vec<String>>,
}

impl PrintSchema {
    pub fn schema_name(&self) -> Option<&str> {
        self.schema.as_ref().map(|s| &**s)
    }

    pub fn import_types(&self) -> Option<&[String]> {
        self.import_types.as_ref().map(|v| &**v)
    }

    fn set_relative_path_base(&mut self, base: &Path) {
        if let Some(ref mut file) = self.file {
            if file.is_relative() {
                *file = base.join(&file);
            }
        }

        if let Some(ref mut patch_file) = self.patch_file {
            if patch_file.is_relative() {
                *patch_file = base.join(&patch_file);
            }
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationsDirectory {
    pub dir: PathBuf,
}
