table! {
    __diesel_schema_migrations (version) {
        version -> VarChar,
        run_on -> Timestamp,
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NewMigration<'a>(pub &'a str);
Insertable! {
    (__diesel_schema_migrations)
    pub struct NewMigration<'a>(
        #[column_name(version)]
        pub &'a str,
    );
}
