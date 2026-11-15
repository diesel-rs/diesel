use std::path::{Path, PathBuf};

use diesel_migrations::MigrationError;

use crate::infer_schema_internals::TableName;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("Initializing `.env` file failed")]
    DotenvError(#[from] dotenvy::Error),
    #[error("Could not connect to database via `{url}`")]
    ConnectionError {
        #[source]
        error: diesel::ConnectionError,
        url: String,
    },
    #[error("Invalid argument for table filtering regex")]
    TableFilterRegexInvalid(#[from] regex::Error),
    #[error("Unable to find diesel.toml or Cargo.toml in {0:?} or any parent directories.")]
    ProjectRootNotFound(PathBuf),
    #[error(
        "The --database-url argument must be passed, or the DATABASE_URL environment variable must be set."
    )]
    DatabaseUrlMissing,
    #[error("Encountered an IO error for `{n}`", n=print_optional_path(.1))]
    IoError(#[source] std::io::Error, Option<PathBuf>),
    #[error("Failed to execute a database query")]
    QueryError(#[from] diesel::result::Error),
    #[error("Failed to run migrations")]
    MigrationError(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Failed to parse schema")]
    SynError(#[from] syn::Error),
    #[error("sqlite cannot infer schema for databases other than the main database")]
    #[cfg(feature = "sqlite")]
    InvalidSqliteSchema,
    #[error("No table with the name `{0}` exists")]
    NoTableFound(TableName),
    #[error("Unsupported type: `{0}`")]
    #[cfg(any(feature = "sqlite", feature = "mysql", feature = "mariadb"))]
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
    #[error("Failed to parse config file")]
    InvalidConfig(#[from] toml::de::Error),
    #[error("Failed to format a string")]
    FmtError(#[from] std::fmt::Error),
    #[error("Failed to parse patch file")]
    DiffyParseError(#[from] diffy::ParsePatchError),
    #[error("Failed to apply patch")]
    DiffyApplyError(#[from] diffy::ApplyError),
    #[error("Column length literal can't be parsed as u64")]
    ColumnLiteralParseError(#[source] syn::Error),
    #[error("Failed to parse database url")]
    UrlParsingError(#[from] url::ParseError),
    #[error("Failed to parse CLI parameter")]
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
    #[error("Error inferring view definitions")]
    InferError(#[source] diesel_infer_query::Error),
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

/// Renders an error followed by all of its sources, separated by `: `
pub fn format_error(error: &dyn std::error::Error) -> String {
    std::iter::successors(Some(error), |error| error.source())
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join(": ")
}

#[cfg(test)]
mod tests {
    use super::{Error, format_error};
    use std::path::PathBuf;

    #[test]
    fn query_error_exposes_source() {
        let error = Error::from(diesel::result::Error::NotFound);

        assert_eq!(error.to_string(), "Failed to execute a database query");
        assert_eq!(
            format_error(&error),
            "Failed to execute a database query: Record not found"
        );
    }

    #[test]
    fn io_error_exposes_source() {
        let error = Error::IoError(
            std::io::Error::new(std::io::ErrorKind::NotFound, "entity not found"),
            Some(PathBuf::from("migrations")),
        );

        assert_eq!(
            error.to_string(),
            "Encountered an IO error for `migrations`"
        );
        assert_eq!(
            format_error(&error),
            "Encountered an IO error for `migrations`: entity not found"
        );
    }

    #[test]
    fn connection_error_exposes_source() {
        let error = Error::ConnectionError {
            error: diesel::ConnectionError::BadConnection("connection refused".into()),
            url: "postgres://localhost/diesel".into(),
        };

        assert_eq!(
            error.to_string(),
            "Could not connect to database via `postgres://localhost/diesel`"
        );
        assert_eq!(
            format_error(&error),
            "Could not connect to database via `postgres://localhost/diesel`: connection refused"
        );
    }

    #[test]
    fn migration_error_exposes_the_complete_source_chain() {
        let integer_error = u8::try_from(256_i32).unwrap_err();
        let error = Error::MigrationError(Box::new(diesel::result::Error::IntegerConversion(
            integer_error,
        )));

        assert_eq!(error.to_string(), "Failed to run migrations");
        assert_eq!(
            format_error(&error),
            format!(
                "Failed to run migrations: Internal integer conversion failed: {integer_error}"
            )
        );
    }

    #[test]
    fn infer_error_exposes_source() {
        let error = Error::from(diesel_infer_query::Error::UnnamedField);

        assert_eq!(error.to_string(), "Error inferring view definitions");
        assert_eq!(
            format_error(&error),
            "Error inferring view definitions: could not get a field name for an expression"
        );
    }
}
