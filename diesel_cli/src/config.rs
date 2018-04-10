use clap::ArgMatches;
use std::env;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use toml;

use super::{find_project_root, handle_error};

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub print_schema: PrintSchema,
}

impl Config {
    pub fn file_path(matches: &ArgMatches) -> PathBuf {
        matches
            .value_of("CONFIG_FILE")
            .map(PathBuf::from)
            .or_else(|| env::var_os("DIESEL_CONFIG_FILE").map(PathBuf::from))
            .unwrap_or_else(|| {
                find_project_root()
                    .unwrap_or_else(handle_error)
                    .join("diesel.toml")
            })
    }

    pub fn read(matches: &ArgMatches) -> Result<Self, Box<Error>> {
        let path = Self::file_path(matches);
        let mut bytes = Vec::new();
        fs::File::open(path)?.read_to_end(&mut bytes)?;
        toml::from_slice(&bytes).map_err(Into::into)
    }
}

#[derive(Default, Deserialize)]
pub struct PrintSchema {
    pub file: Option<PathBuf>,
}
