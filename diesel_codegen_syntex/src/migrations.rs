use diesel::migrations::search_for_migrations_directory;
use std::error::Error;
use std::env;
use std::path::{Path, PathBuf};
use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::*;
use syntax::util::small_vector::SmallVector;
use syntax::ptr::P;
use syntax::ext::build::AstBuilder;
use syntax::tokenstream;

pub fn expand_embed_migrations<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    tts: &[tokenstream::TokenTree]
) -> Box<MacResult+'cx> {
    let migrations_expr = migrations_directory_from_args(cx, sp, tts)
        .and_then(|d| migration_literals_from_path(cx, sp, &d));
    let migrations_expr = match migrations_expr {
        Err(e) => {
            cx.span_err(sp, &format!("Error reading migrations: {}", e));
            return DummyResult::expr(sp);
        }
        Ok(v) => v,
    };

    let item = quote_item!(cx, mod embedded_migrations {
        extern crate diesel;

        use self::diesel::migrations::*;
        use self::diesel::connection::SimpleConnection;
        use std::io;

        struct EmbeddedMigration {
            version: &'static str,
            up_sql: &'static str,
        }

        impl Migration for EmbeddedMigration {
            fn version(&self) -> &str {
                self.version
            }

            fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
                conn.batch_execute(self.up_sql).map_err(Into::into)
            }

            fn revert(&self, _conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
                unreachable!()
            }
        }

        const ALL_MIGRATIONS: &'static [&'static Migration] = $migrations_expr;

        pub fn run<C: MigrationConnection>(conn: &C) -> Result<(), RunMigrationsError> {
            run_with_output(conn, &mut io::sink())
        }

        pub fn run_with_output<C: MigrationConnection>(conn: &C, out: &mut io::Write)
            -> Result<(), RunMigrationsError>
        {
            run_migrations(conn, ALL_MIGRATIONS.iter().map(|v| *v), out)
        }
    }).unwrap();
    MacEager::items(SmallVector::one(item))
}

fn migrations_directory_from_args(
    cx: &mut ExtCtxt,
    sp: Span,
    tts: &[tokenstream::TokenTree],
) -> Result<PathBuf, Box<Error>> {
    let cargo_toml_directory = try!(env::var("CARGO_MANIFEST_DIR"));
    let relative_path_to_migrations = if tts.is_empty() {
        None
    } else {
        match get_single_str_from_tts(cx, sp, tts, "embed_migrations!") {
            None => return Err("Usage error".into()),
            value => value,
        }
    };
    let cargo_manifest_path = Path::new(&cargo_toml_directory);
    let migrations_path = relative_path_to_migrations.as_ref().map(Path::new);
    resolve_migrations_directory(cargo_manifest_path, migrations_path)
}

fn resolve_migrations_directory(
    cargo_manifest_dir: &Path,
    relative_path_to_migrations: Option<&Path>,
) -> Result<PathBuf, Box<Error>> {
    let result = match relative_path_to_migrations {
        Some(dir) => cargo_manifest_dir.join(dir),
        None => {
            // People commonly put their migrations in src/migrations
            // so start the search there rather than the project root
            let src_dir = cargo_manifest_dir.join("src");
            try!(search_for_migrations_directory(&src_dir))
        }
    };

    result.canonicalize().map_err(Into::into)
}

fn migration_literals_from_path(
    cx: &ExtCtxt,
    sp: Span,
    path: &Path,
) -> Result<P<ast::Expr>, Box<Error>> {
    use diesel::migrations::migration_paths_in_directory;

    let exprs = try!(migration_paths_in_directory(&path)).into_iter()
        .map(|e| migration_literal_from_path(cx, &e.path()))
        .collect();
    Ok(cx.expr_vec_slice(sp, try!(exprs)))
}

fn migration_literal_from_path(
    cx: &ExtCtxt,
    path: &Path,
) -> Result<P<ast::Expr>, Box<Error>> {
    use diesel::migrations::version_from_path;

    let version = try!(version_from_path(path));
    let sql_file = path.join("up.sql");
    let sql_file_path = sql_file.to_string_lossy();

    Ok(quote_expr!(cx, &EmbeddedMigration {
        version: $version,
        up_sql: include_str!($sql_file_path),
    }))
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use self::tempdir::TempDir;
    use std::fs;
    use std::path::Path;
    use super::resolve_migrations_directory;

    #[test]
    fn migrations_directory_resolved_relative_to_cargo_manifest_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/special_migrations")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo");
        let relative_path = Some(Path::new("special_migrations"));

        assert_eq!(
            tempdir.path().join("foo/special_migrations").canonicalize().ok(),
            resolve_migrations_directory(&cargo_manifest_dir, relative_path).ok()
        );
    }

    #[test]
    fn migrations_directory_canonicalizes_result() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations/bar")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/bar")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo/bar");
        let relative_path = Some(Path::new("../migrations/bar"));

        assert_eq!(
            tempdir.path().join("foo/migrations/bar").canonicalize().ok(),
            resolve_migrations_directory(&cargo_manifest_dir, relative_path).ok()
        );
    }

    #[test]
    fn migrations_directory_defaults_to_searching_migrations_path() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/bar")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo/bar");

        assert_eq!(
            tempdir.path().join("foo/migrations").canonicalize().ok(),
            resolve_migrations_directory(&cargo_manifest_dir, None).ok()
        );
    }

    #[test]
    fn migrations_directory_searches_src_migrations_if_present() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/src/migrations")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo");

        assert_eq!(
            tempdir.path().join("foo/src/migrations").canonicalize().ok(),
            resolve_migrations_directory(&cargo_manifest_dir, None).ok()
        );
    }

    #[test]
    fn migrations_directory_allows_no_parent_dir_for_cargo_manifest_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("special_migrations")).unwrap();
        let cargo_manifest_dir = tempdir.path().to_owned();
        let relative_path = Some(Path::new("special_migrations"));
        assert_eq!(
            tempdir.path().join("special_migrations").canonicalize().ok(),
            resolve_migrations_directory(&cargo_manifest_dir, relative_path).ok()
        );
    }
}
