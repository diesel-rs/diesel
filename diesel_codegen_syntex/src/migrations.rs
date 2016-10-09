use diesel_codegen_shared::migrations::resolve_migrations_directory;
use std::error::Error;
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
    let callsite_file = try!(::std::env::var("CARGO_MANIFEST_DIR"));
    let relative_path_to_migrations = if tts.is_empty() {
        None
    } else {
        match get_single_str_from_tts(cx, sp, tts, "embed_migrations!") {
            None => return Err("Usage error".into()),
            value => value,
        }
    };
    let callsite_path = Path::new(&callsite_file);
    let migrations_path = relative_path_to_migrations.as_ref().map(Path::new);
    resolve_migrations_directory(callsite_path, migrations_path)
        .map_err(|_| {
            "Migration dir not found".into()
        })
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


