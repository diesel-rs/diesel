use diesel::migrations::{MigrationError,find_migrations_directory, migrations_in_directory};
use diesel::migrations::migration::{valid_sql_migration_directory, version_from_path};
use syntax::ast;
use syntax::ext::base::*;
use syntax::codemap::Span;
use syntax::util::small_vector::SmallVector;
use std::path::{PathBuf, Path};
use std::fs::File;
use std::io::Read;
use std::env;


fn sql_from_file(path: &Path) -> Result<String, MigrationError> {
    let mut sql = String::new();
    let mut file = try!(File::open(path));
    try!(file.read_to_string(&mut sql));
    Ok(sql)
}

fn read_migration<'cx>(
    path: PathBuf,
    cx: &'cx mut ExtCtxt
) -> Result<Vec<ast::TokenTree>, MigrationError>
{
    if valid_sql_migration_directory(&path) {
        let version = try!(version_from_path(&path));
        let up = try!(sql_from_file(&path.join("up.sql")));
        let down = try!(sql_from_file(&path.join("down.sql")));
        Ok(quote_tokens!(cx, create(::diesel::migrations::migration::ProgrammaticMigration{version: &$version, up:&$up, down:&$down}), ))
    } else {
        Err(MigrationError::UnknownMigrationFormat(path))
    }
}

pub fn create_migration_module<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    tts: &[ast::TokenTree]
) -> Box<MacResult + 'cx>
{
    let path = match get_exprs_from_tts(cx, sp, tts) {
        Some(exprs) => {
            match exprs.len() {
                0 => None,
                1 => {
                    expr_to_string(cx,
                                   exprs.into_iter().next().unwrap(),
                                   "expected string literal")
                        .map(|(s, _)| s)
                }
                n => {
                    cx.span_err(sp,
                                &format!("create_migration_module! takes 0 or 1 arguments but \
                                          has {}",
                                         n));
                    return DummyResult::any(sp);
                }
            }
        }
        None => return DummyResult::any(sp),
    };
    let migrations_dir = if path.is_some() {
        let current_dir = match env::current_dir() {
            Ok(c) => c,
            Err(_) => {
                cx.span_err(sp, "could not get current directory from environment");
                return DummyResult::any(sp);
            }
        };
        let migration_path = current_dir.join(path.unwrap().to_string());
        if migration_path.is_dir() {
            Ok(migration_path)
        } else {
            cx.span_err(sp,
                        &format!("{} does not exist", migration_path.to_str().unwrap()));
            return DummyResult::any(sp);
        }
    } else {
        find_migrations_directory()
    };

    let migrations_dir = match migrations_dir {
        Ok(path) => path,
        Err(e) => {
            cx.span_err(sp, &format!("an unknown error occured: {}", e));
            return DummyResult::any(sp);
        }
    };


    let migrations = match migrations_in_directory(&migrations_dir,
                                                   |path| read_migration(path, cx)) {
        Ok(m) => m,
        Err(e) => {
            cx.span_err(sp, &format!("could not read migrations: {}", e));
            return DummyResult::any(sp);
        }
    };

    let migrations_module =
        quote_item!(cx,
                    pub mod migrations {

                        fn create(m: ::diesel::migrations::migration::ProgrammaticMigration)
                                  -> Box<::diesel::migrations::migration::Migration>
                        {
                            Box::new(m) as Box<::diesel::migrations::migration::Migration>
                        }

                        pub fn get_migrations() -> Vec<Box<::diesel::migrations::migration::Migration>>{
                            vec!($migrations)
                        }

                        pub fn run<Conn>(connection: &Conn)
                                         -> Result<Vec<String>, ::diesel::migrations::RunMigrationsError>
                            where Conn: ::diesel::migrations::MigrationConnection,
                        {
                            ::diesel::migrations::run_pending_migrations_from_iter(
                                connection, get_migrations().into_iter(), &mut ::std::io::stdout())
                        }

                        pub fn run_with_writer<Conn, W>(connection: &Conn, writer: &mut ::std::io::Write)
                                                        -> Result<Vec<String>, ::diesel::migrations::RunMigrationsError>
                            where Conn: ::diesel::migrations::MigrationConnection,
                        {
                            ::diesel::migrations::run_pending_migrations_from_iter(
                                connection, get_migrations().into_iter(), writer)
                        }


                    }).unwrap();

    MacEager::items(SmallVector::one(migrations_module))
}
