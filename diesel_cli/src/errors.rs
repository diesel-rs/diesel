use std::path::PathBuf;

use crate::infer_schema_internals::TableName;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("Initializing `.env` file failed: {0}")]
    DotenvError(#[from] dotenvy::Error),
    #[error("Could not connect to database via `{url}`: {error}")]
    ConnectionError {
        error: diesel::ConnectionError,
        url: String,
    },
    #[error("Invalid argument for table filtering regex: {0}")]
    TableFilterRegexInvalid(#[from] regex::Error),
    #[error("Unable to find diesel.toml or Cargo.toml in {0:?} or any parent directories.")]
    ProjectRootNotFound(PathBuf),
    #[error("The --database-url argument must be passed, or the DATABASE_URL environment variable must be set.")]
    DatabaseUrlMissing,
    #[error("Encountered an IO error: {0} {}", print_optional_path(.1))]
    IoError(#[source] std::io::Error, Option<PathBuf>),
    #[error("Failed to execute a database query: {0}")]
    QueryError(#[from] diesel::result::Error),
    #[error("Failed to run migrations: {0}")]
    MigrationError(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Failed to parse schema: {0}")]
    SynError(#[from] syn::Error),
    #[error("sqlite cannot infer schema for databases other than the main database")]
    #[cfg(feature = "sqlite")]
    InvalidSqliteSchema,
    #[error("No table with the name `{0}` exists")]
    NoTableFound(TableName),
    #[error("Unsupported type: `{0}`")]
    #[cfg(any(feature = "sqlite", feature = "mysql"))]
    UnsupportedType(String),
    #[error(
        "Diesel only supports tables with primary keys. \
             Table `{0}` has no primary key"
    )]
    NoPrimaryKeyFound(TableName),
    #[error("{0}")]
    UnsupportedFeature(String),
    #[error(
        "Command would result in changes to `{0}`. \
         Rerun the command locally, and commit the changes."
    )]
    SchemaWouldChange(String),
    #[error("Failed to parse config file: {0}")]
    InvalidConfig(#[from] toml::de::Error),
    #[error("Failed to format a string: {0}")]
    FmtError(#[from] std::fmt::Error),
    #[error("Failed to parse patch file: {0}")]
    DiffyParseError(#[from] diffy::ParsePatchError),
    #[error("Failed to apply path: {0}")]
    DiffyApplyError(#[from] diffy::ApplyError),
    #[error("Column length literal can't be parsed as u64: {0}")]
    ColumnLiteralParseError(syn::Error),
    #[error("Failed to parse database url: {0}")]
    UrlParsingError(#[from] url::ParseError),
    #[error("Failed to parse CLI parameter: {0}")]
    ClapMatchesError(#[from] clap::parser::MatchesError),
    #[error("No `[print_schema.{0}]` entries in your diesel.toml")]
    NoSchemaKeyFound(String),
}

fn print_optional_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|p| format!(" for `{}`", p.display()))
        .unwrap_or_default()
}
