#[macro_export]
/// This macro can only be used in combination with the `diesel_codegen` or
/// `diesel_codegen_syntex` crates. It will not work on its own.
///
/// FIXME: Oh look we have a place to actually document this now.
macro_rules! infer_schema {
    ($database_url: expr) => {
        #[derive(InferSchema)]
        #[options(database_url=$database_url)]
        struct _Dummy;
    }
}

#[macro_export]
/// This macro can only be used in combination with the `diesel_codegen` or
/// `diesel_codegen_syntex` crates. It will not work on its own.
///
/// FIXME: Oh look we have a place to actually document this now.
macro_rules! infer_table_from_schema {
    ($database_url: expr, $table_name: expr) => {
        #[derive(InferTableFromSchema)]
        #[options(database_url=$database_url, table_name=$table_name)]
        struct _Dummy;
    }
}

#[macro_export]
/// This macro can only be used in combination with the `diesel_codegen` or
/// `diesel_codegen_syntex` crates. It will not work on its own.
///
/// FIXME: Oh look we have a place to actually document this now.
macro_rules! embed_migrations {
    () => {
        #[derive(EmbedMigrations)]
        struct _Dummy;
    };

    ($migrations_path: expr) => {
        #[derive(EmbedMigrations)]
        #[options(migrations_path=$migrations_path)]
        struct _Dummy;
    }
}
