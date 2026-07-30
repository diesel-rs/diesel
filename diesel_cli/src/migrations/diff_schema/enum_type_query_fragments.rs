use super::ColumnTypeName;
use super::schema_parsing::{EnumInfos, SqlTypeInfo};
use crate::infer_schema_internals::{ColumnDefinition, QueryRelationData};
use diesel::QueryResult;
use diesel::query_builder::QueryFragment;

pub(crate) struct EnumType<'a> {
    pub(crate) tpe: &'a super::schema_parsing::SqlTypeInfo,
    pub(crate) variants: &'a [super::schema_parsing::EnumVariant],
}

#[cfg(feature = "postgres")]
impl QueryFragment<diesel::pg::Pg> for EnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::pg::Pg>,
    ) -> QueryResult<()> {
        let _ = self.variants;
        if let Some(pg_type) = &self.tpe.postgres_enum {
            pg_type.type_name(pass)?;
        } else {
            pass.push_sql(&self.tpe.rust_name.to_string());
        }
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<diesel::mysql::Mysql> for EnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::mysql::Mysql>,
    ) -> QueryResult<()> {
        let _ = self.tpe;
        pass.push_sql("enum(");
        let variants = self
            .variants
            .iter()
            .map(|v| format!("'{}'", v.sql_name.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        pass.push_sql(&variants);
        pass.push_sql(")");
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryFragment<diesel::sqlite::Sqlite> for EnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::sqlite::Sqlite>,
    ) -> QueryResult<()> {
        let _ = self.variants;
        pass.push_sql(&self.tpe.rust_name.to_string());
        Ok(())
    }
}

pub(crate) struct CreateEnumType<'a> {
    pub(crate) tpe: &'a super::schema_parsing::SqlTypeInfo,
    pub(crate) variants: &'a [super::schema_parsing::EnumVariant],
}

#[cfg(feature = "postgres")]
impl QueryFragment<diesel::pg::Pg> for CreateEnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::pg::Pg>,
    ) -> QueryResult<()> {
        if let Some(pg_type) = &self.tpe.postgres_enum {
            pass.push_sql("CREATE TYPE ");
            pg_type.type_name(pass.reborrow())?;
            pass.push_sql(" AS enum(");
            let variants = self
                .variants
                .iter()
                .map(|v| format!("'{}'", v.sql_name.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            pass.push_sql(&variants);
            pass.push_sql(");");
        }
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<diesel::mysql::Mysql> for CreateEnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        _pass: diesel::query_builder::AstPass<'_, 'b, diesel::mysql::Mysql>,
    ) -> QueryResult<()> {
        let _ = self.tpe;
        let _ = self.variants;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryFragment<diesel::sqlite::Sqlite> for CreateEnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        _pass: diesel::query_builder::AstPass<'_, 'b, diesel::sqlite::Sqlite>,
    ) -> QueryResult<()> {
        let _ = self.tpe;
        let _ = self.variants;
        Ok(())
    }
}

pub(crate) struct DropEnumType<'a> {
    pub(crate) tpe: &'a super::schema_parsing::SqlTypeInfo,
}

#[cfg(feature = "postgres")]
impl QueryFragment<diesel::pg::Pg> for DropEnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::pg::Pg>,
    ) -> QueryResult<()> {
        if let Some(pg_type) = &self.tpe.postgres_enum {
            pass.push_sql("DROP TYPE ");
            pg_type.type_name(pass.reborrow())?;
            pass.push_sql(";");
        }
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<diesel::mysql::Mysql> for DropEnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        _pass: diesel::query_builder::AstPass<'_, 'b, diesel::mysql::Mysql>,
    ) -> QueryResult<()> {
        let _ = self.tpe;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryFragment<diesel::sqlite::Sqlite> for DropEnumType<'_> {
    fn walk_ast<'b>(
        &'b self,
        _pass: diesel::query_builder::AstPass<'_, 'b, diesel::sqlite::Sqlite>,
    ) -> QueryResult<()> {
        let _ = self.tpe;
        Ok(())
    }
}

pub(crate) struct AddEnumVariants<'a> {
    pub(crate) added_variants: &'a [String],
    pub(crate) all_variants: &'a [String],
    pub(crate) column_info: Option<(&'a str, &'a ColumnDefinition)>,
    pub(crate) tpe: &'a super::schema_parsing::SqlTypeInfo,
}

#[cfg(feature = "postgres")]
impl QueryFragment<diesel::pg::Pg> for AddEnumVariants<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::pg::Pg>,
    ) -> QueryResult<()> {
        let _ = self.all_variants;
        let _ = self.column_info;
        if let Some(pg_type) = &self.tpe.postgres_enum {
            for added_variant in self.added_variants {
                pass.push_sql("ALTER TYPE ");
                pg_type.type_name(pass.reborrow())?;
                pass.push_sql(" ADD VALUE '");
                pass.push_sql(&added_variant.replace('\'', "''"));
                pass.push_sql("';");
            }
        }
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<diesel::mysql::Mysql> for AddEnumVariants<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::mysql::Mysql>,
    ) -> QueryResult<()> {
        let _ = self.added_variants;
        let _ = self.tpe;
        if let Some((table_infos, column_info)) = self.column_info {
            use diesel_infer_query::SchemaField;

            pass.push_sql("ALTER TABLE ");
            pass.push_identifier(table_infos)?;
            pass.push_sql(" MODIFY COLUMN ");
            pass.push_identifier(&column_info.sql_name)?;
            pass.push_sql(" enum(");
            let variants = self
                .all_variants
                .iter()
                .map(|v| format!("'{}'", v.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            pass.push_sql(&variants);
            pass.push_sql(")");
            if !column_info.is_nullable() {
                pass.push_sql(" NOT NULL");
            }
            pass.push_sql(";");
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryFragment<diesel::sqlite::Sqlite> for AddEnumVariants<'_> {
    fn walk_ast<'b>(
        &'b self,
        _pass: diesel::query_builder::AstPass<'_, 'b, diesel::sqlite::Sqlite>,
    ) -> QueryResult<()> {
        let _ = self.all_variants;
        let _ = self.added_variants;
        let _ = self.column_info;
        let _ = self.tpe;
        Ok(())
    }
}

pub(crate) struct MigrateEnumData<'a> {
    pub(crate) affected_tables: &'a [(ColumnDefinition, QueryRelationData)],
    pub(crate) tpe: &'a super::schema_parsing::SqlTypeInfo,
    pub(crate) column_defs: Vec<ColumnTypeName<'a>>,
    create_enum: CreateEnumType<'a>,
}

impl<'a> MigrateEnumData<'a> {
    pub(crate) fn new(
        affected_tables: &'a [(ColumnDefinition, QueryRelationData)],
        tpe: &'a super::schema_parsing::SqlTypeInfo,
        enum_sql_types: &'a [(SqlTypeInfo, EnumInfos)],
        infos: &'a EnumInfos,
    ) -> Self {
        let column_defs = affected_tables
            .iter()
            .map(|(c, t)| {
                ColumnTypeName::new(
                    &c.ty,
                    &c.sql_name,
                    true,
                    enum_sql_types,
                    &t.table_name().rust_name,
                )
            })
            .collect();

        Self {
            affected_tables,
            tpe,
            column_defs,
            create_enum: CreateEnumType {
                tpe,
                variants: &infos.variants,
            },
        }
    }
}

#[cfg(feature = "postgres")]
impl QueryFragment<diesel::pg::Pg> for MigrateEnumData<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::pg::Pg>,
    ) -> QueryResult<()> {
        if let Some(pg_type) = &self.tpe.postgres_enum {
            // rename the type:
            pass.push_sql("-- Rename the old type\n");
            pass.push_sql("ALTER TYPE ");
            pg_type.type_name(pass.reborrow())?;
            pass.push_sql(" RENAME TO ");
            let mut t = pg_type.clone();
            t.name += "_tmp";
            t.type_name(pass.reborrow())?;
            pass.push_sql(";");
            pass.push_sql("\n");
            pass.push_sql("-- Create a new type definition\n");
            // create a new type
            self.create_enum.walk_ast(pass.reborrow())?;
            pass.push_sql("\n\n");

            pass.push_sql("-- Convert existing data\n");
            for ((c, t), col_def) in self.affected_tables.iter().zip(&self.column_defs) {
                // rename the column
                pass.push_sql("-- Rename the existing column\n");
                pass.push_sql("ALTER TABLE ");
                pass.push_identifier(&t.table_name().sql_name)?;
                pass.push_sql(" RENAME COLUMN ");
                pass.push_identifier(&c.sql_name)?;
                pass.push_sql(" TO ");
                pass.push_identifier(&format!("{}_tmp", c.sql_name))?;
                pass.push_sql(";\n");

                // add a new column with the new type
                pass.push_sql("-- Add a new column with the new type\n");
                pass.push_sql("ALTER TABLE ");
                pass.push_identifier(&t.table_name().sql_name)?;
                pass.push_sql(" ADD COLUMN ");
                pass.push_identifier(&c.sql_name)?;
                col_def.walk_ast(pass.reborrow())?;
                pass.push_sql(";\n\n");

                pass.push_sql("-- Add a section here to migrate your data to not contain\n");
                pass.push_sql("-- any of the removed variants anymore\n\n");

                // migrate the values
                pass.push_sql("-- Restore the existing data\n");
                pass.push_sql("UPDATE ");
                pass.push_identifier(&t.table_name().sql_name)?;
                pass.push_sql(" SET ");
                pass.push_identifier(&c.sql_name)?;
                pass.push_sql(" = ");
                pass.push_identifier(&format!("{}_tmp", c.sql_name))?;
                pass.push_sql("::text::");
                pg_type.type_name(pass.reborrow())?;
                pass.push_sql(";\n");

                if !c.ty.is_nullable {
                    // mark the column as not null
                    pass.push_sql("-- Mark the column as NOT NULL again\n");
                    pass.push_sql("ALTER TABLE ");
                    pass.push_identifier(&t.table_name().sql_name)?;
                    pass.push_sql(" ALTER COLUMN ");
                    pass.push_identifier(&c.sql_name)?;
                    pass.push_sql(" SET NOT NULL;\n");
                }

                // drop the column with the old data
                pass.push_sql("-- Drop the column with the old enum type\n");
                pass.push_sql("ALTER TABLE ");
                pass.push_identifier(&t.table_name().sql_name)?;
                pass.push_sql(" DROP COLUMN ");
                pass.push_identifier(&format!("{}_tmp", c.sql_name))?;
                pass.push_sql(";\n\n");
            }

            // finally drop the type
            pass.push_sql("-- Finally drop the old enum type\n");
            pass.push_sql("DROP TYPE ");
            let mut t = pg_type.clone();
            t.name += "_tmp";
            t.type_name(pass.reborrow())?;
            pass.push_sql(";");
            pass.push_sql("\n\n");
        }
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<diesel::mysql::Mysql> for MigrateEnumData<'_> {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::mysql::Mysql>,
    ) -> QueryResult<()> {
        let _ = self.tpe;
        let _ = self.column_defs;
        for (col, table) in self.affected_tables {
            pass.push_sql("ALTER TABLE ");
            pass.push_identifier(&table.table_name().sql_name)?;
            pass.push_sql(" MODIFY COLUMN ");
            pass.push_identifier(&col.sql_name)?;
            pass.push_sql(" enum(");
            let variants = self
                .create_enum
                .variants
                .iter()
                .map(|v| format!("'{}'", v.sql_name.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            pass.push_sql(&variants);
            pass.push_sql(")");
            if !col.ty.is_nullable {
                pass.push_sql(" NOT NULL");
            }
            pass.push_sql(";");
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryFragment<diesel::sqlite::Sqlite> for MigrateEnumData<'_> {
    fn walk_ast<'b>(
        &'b self,
        _pass: diesel::query_builder::AstPass<'_, 'b, diesel::sqlite::Sqlite>,
    ) -> QueryResult<()> {
        let _ = self.affected_tables;
        let _ = self.column_defs;
        let _ = self.create_enum;
        let _ = self.tpe;
        Ok(())
    }
}
