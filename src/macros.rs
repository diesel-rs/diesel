#[macro_export]
macro_rules! table {
    (
        $name:ident {
            $($column_name:ident -> $Type:ty,)+
        }
    ) => {
        table! {
            $name (id) {
                $($column_name -> $Type,)+
            }
        }
    };
    (
        $name:ident ($pk:ident) {
            $($column_name:ident -> $Type:ty,)+
        }
    ) => {
        pub mod $name {
            use $crate::*;
            use $crate::query_builder::*;
            use $crate::types::*;
            pub use self::columns::*;

            pub mod dsl {
                pub use super::columns::{$($column_name),+};
                pub use super::table as $name;
            }

            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            pub struct table;

            pub type SqlType = ($($Type),+);

            impl QuerySource for table {
                fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
                    out.push_identifier(stringify!($name))
                }
            }

            impl AsQuery for table {
                type SqlType = SqlType;
                type Query = SelectStatement<SqlType, star, Self>;

                fn as_query(self) -> Self::Query {
                    SelectStatement::simple(star, self)
                }
            }

            impl Table for table {
                type PrimaryKey = columns::$pk;
                type Star = star;

                fn name(&self) -> &str {
                    stringify!($name)
                }

                fn primary_key(&self) -> Self::PrimaryKey {
                    columns::$pk
                }

                fn star(&self) -> Self::Star {
                    star
                }
            }

            pub mod columns {
                use super::table;
                use $crate::{Table, Column, Expression};
                use $crate::query_builder::{QueryBuilder, BuildQueryResult};
                use $crate::types::*;

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy)]
                pub struct star;

                impl Expression for star {
                    type SqlType = super::SqlType;

                    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
                        try!(out.push_identifier(table.name()));
                        out.push_sql(".*");
                        Ok(())
                    }
                }

                impl Column for star {
                    type Table = table;

                    fn name() -> &'static str {
                        "*"
                    }
                }

                $(#[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy)]
                pub struct $column_name;

                impl Expression for $column_name {
                    type SqlType = $Type;

                    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
                        try!(out.push_identifier(table.name()));
                        out.push_sql(".");
                        out.push_identifier(stringify!($column_name))
                    }
                }

                impl Column for $column_name {
                    type Table = table;

                    fn name() -> &'static str {
                        stringify!($column_name)
                    }
                }

                addable_expr!($column_name);
                )+
            }
        }
    }
}

#[macro_export]
macro_rules! queriable {
    (
        $Struct:ident {
            $($field_name:ident -> $Type:ty,)+
        }
    ) => {
        impl<ST> $crate::Queriable<ST> for $Struct where
            ST: $crate::types::NativeSqlType,
            ($($Type),+): $crate::types::FromSqlRow<ST>,
        {
            type Row = ($($Type),+);

            fn build(row: Self::Row) -> Self {
                let ($($field_name),+) = row;
                $Struct {
                    $($field_name: $field_name),+
                }
            }
        }
    }
}

#[macro_export]
macro_rules! insertable {
    (
        $Struct:ty => $table_mod:ident {
            $($field_name:ident -> $Type:ty,)+
        }
    ) => {
        insertable! {
            $Struct => $table_mod {
                $($table_mod, $field_name -> $Type,)+
            }
        }
    };
    (
        $Struct:ty => $table_mod:ident {
            $($field_table_name:ident, $field_name:ident -> $Type:ty,)+
        }
    ) => {
        impl<'a: 'insert, 'insert> $crate::persistable::Insertable<$table_mod::table>
            for &'insert $Struct
        {
            type Columns = ($($table_mod::$field_name),+);
            type Values = $crate::expression::grouped::Grouped<($(
                <&'insert $Type as $crate::expression::AsExpression<
                    <$table_mod::$field_name as $crate::expression::Expression>::SqlType
                >>::Expression
            ),+)>;

            fn columns() -> Self::Columns {
                ($($table_mod::$field_name),+)
            }

            fn values(self) -> Self::Values {
                use $crate::expression::AsExpression;
                use $crate::expression::grouped::Grouped;
                Grouped(($(AsExpression::<
                   <$table_mod::$field_name as $crate::expression::Expression>::SqlType>
                   ::as_expression(&self.$field_name)
               ),+))
            }
        }
    };
}

#[macro_export]
macro_rules! changeset {
    (
        $Struct:ty => $table_mod:ident {
            $($field_name:ident -> $Type:ty,)+
        }
    ) => {
        impl<'a: 'update, 'update> $crate::query_builder::AsChangeset
            for &'update $Struct
        {
            type Changeset = ($(
                $crate::expression::predicates::Eq<
                    $table_mod::$field_name,
                    $crate::expression::bound::Bound<
                        <$table_mod::$field_name as $crate::expression::Expression>::SqlType,
                        &'update $Type,
                    >,
                >
            ),+);

            fn as_changeset(self) -> Self::Changeset {
                use $crate::expression::Expression;

                ($(
                    $table_mod::$field_name.eq(&self.$field_name)
                ),+)
            }
        }
    };
}

#[macro_export]
macro_rules! joinable {
    ($child:ident -> $parent:ident ($source:ident = $target:ident)) => {
        joinable_inner!($child -> $parent ($source = $target));
        joinable_inner!($parent -> $child ($target = $source));
    }
}

#[macro_export]
macro_rules! joinable_inner {
    ($child:ident -> $parent:ident ($source:ident = $target:ident)) => {
        impl $crate::JoinTo<$parent::table> for $child::table {
            type Predicate = $crate::expression::predicates::Eq<$child::$source, $parent::$target>;

            fn join_expression(&self) -> Self::Predicate {
                use $crate::Expression;
                $child::$source.eq($parent::$target)
            }
        }
    }
}

#[macro_export]
macro_rules! select_column_workaround {
    ($parent:ident -> $child:ident ($($column_name:ident),+)) => {
        $(select_column_inner!($parent -> $child $column_name);)+
        select_column_inner!($parent -> $child star);
    }
}

#[macro_export]
macro_rules! select_column_inner {
    ($parent:ident -> $child:ident $column_name:ident) => {
        impl $crate::expression::SelectableExpression<
            $crate::query_source::InnerJoinSource<$child::table, $parent::table>,
        > for $parent::$column_name
        {
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::InnerJoinSource<$parent::table, $child::table>,
        > for $parent::$column_name
        {
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::LeftOuterJoinSource<$child::table, $parent::table>,
            $crate::types::Nullable<
                <$parent::$column_name as $crate::Expression>::SqlType>,
        > for $parent::$column_name
        {
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::LeftOuterJoinSource<$parent::table, $child::table>,
        > for $parent::$column_name
        {
        }
    }
}
