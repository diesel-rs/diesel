use std::path::{Path, PathBuf};

use diesel_migrations::MigrationError;

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
    #[error(
        "The --database-url argument must be passed, or the DATABASE_URL environment variable must be set."
    )]
    DatabaseUrlMissing,
    #[error("Encountered an IO error: {0} for `{n}`", n=print_optional_path(.1))]
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
    #[error("Failed to apply patch: {0}")]
    DiffyApplyError(#[from] diffy::ApplyError),
    #[error("Column length literal can't be parsed as u64: {0}")]
    ColumnLiteralParseError(syn::Error),
    #[error("Failed to parse database url: {0}")]
    UrlParsingError(#[from] url::ParseError),
    #[error("Failed to parse CLI parameter: {0}")]
    ClapMatchesError(#[from] clap::parser::MatchesError),
    #[error("No `[print_schema.{0}]` entries in your diesel.toml")]
    NoSchemaKeyFound(String),
    #[error("Failed To Run rustfmt")]
    RustFmtFail(String),
    #[error("Failed to acquire migration folder lock: {1} for `{n}`", n=print_path(.0))]
    FailedToAcquireMigrationFolderLock(PathBuf, String),
    #[error("Tried to generate too many migrations with the same version `{1}` - Migrations folder is `{n}`", n=print_path(.0))]
    TooManyMigrations(PathBuf, String),
    #[error("Specified migration version `{1}` already exists inside `{n}`", n=print_path(.0))]
    DuplicateMigrationVersion(PathBuf, String),
    #[error("Could not resolved view: Failed to resolve relation `{n}`", n=print_relation(.0))]
    CouldNotResolveView(TableName),
    #[error("Invalid field used in view definition: `{n}`, field `{f}`", n = print_relation(.0), f=.1)]
    FieldNotFoundForView(TableName, String),
    #[error("Cyclic view definition detected: `{n}`", n=print_relation(.0))]
    CyclicViewDefinition(TableName),
    #[error("Error inferring view definitions: {0}")]
    InferError(diesel_infer_query::Error),
}

fn print_path(path: &Path) -> String {
    format!("{}", path.display())
}

fn print_optional_path(path: &Option<PathBuf>) -> String {
    path.as_ref().map(|p| print_path(p)).unwrap_or_default()
}

fn print_relation(tpl: &TableName) -> String {
    tpl.full_sql_name()
}

impl Error {
    pub fn from_migration_error<T: Into<PathBuf>>(error: MigrationError, path: Option<T>) -> Self {
        match error {
            MigrationError::IoError(error) => Self::IoError(error, path.map(Into::into)),
            _ => Self::MigrationError(Box::new(error)),
        }
    }
}

impl From<diesel_infer_query::Error> for Error {
    fn from(value: diesel_infer_query::Error) -> Self {
        match value {
            diesel_infer_query::Error::ResolverFailure { inner, .. }
                if inner.downcast_ref::<Self>().is_some() =>
            {
                *inner.downcast().expect("We checked this before")
            }
            e => Self::InferError(e),
        }
    }
}
