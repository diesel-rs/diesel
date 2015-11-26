// FIXME(https://github.com/rust-lang/rust/issues/19630) Remove this work-around
#[macro_export]
macro_rules! yaqb_internal_expr_conversion {
    ($e:expr) => { $e }
}

#[macro_export]
macro_rules! column {
    ($($table:ident)::*, $column_name:ident -> $Type:ty) => {
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy)]
        pub struct $column_name;

        impl $crate::expression::Expression for $column_name {
            type SqlType = $Type;

            fn to_sql(&self, out: &mut $crate::query_builder::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                try!(out.push_identifier($($table)::*::name()));
                out.push_sql(".");
                out.push_identifier(stringify!($column_name))
            }
        }

        impl $crate::expression::SelectableExpression<$($table)::*> for $column_name {}

        impl<'a, ST, Left, Right> SelectableExpression<
            $crate::WithQuerySource<'a, Left, Right>, ST> for $column_name where
            ST: NativeSqlType,
            $column_name: SelectableExpression<Left, ST>
        {
        }

        impl $crate::expression::NonAggregate for $column_name {}

        impl $crate::query_source::Column for $column_name {
            type Table = $($table)::*;

            fn name() -> &'static str {
                stringify!($column_name)
            }
        }
    }
}

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
        table! {
            $name ($pk) {
                $($column_name -> $Type,)+
            } no select {}
        }
    };
    (
        $name:ident {
            $($column_name:ident -> $Type:ty,)+
        } no select {
            $($no_select_column_name:ident -> $no_select_type:ty,)*
        }
    ) => {
        table! {
            $name (id) {
                $($column_name -> $Type,)+
            } no select {
                $($no_select_column_name -> $no_select_type,)*
            }
        }
    };
    (
        $name:ident ($pk:ident) {
            $($column_name:ident -> $Type:ty,)+
        } no select {
            $($no_select_column_name:ident -> $no_select_type:ty,)*
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

            #[allow(non_upper_case_globals, dead_code)]
            pub const all_columns: ($($column_name),+) = ($($column_name),+);

            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            pub struct table;

            impl table {
                #[allow(dead_code)]
                pub fn star(&self) -> star {
                    star
                }
            }

            pub type SqlType = ($($Type),+);

            impl QuerySource for table {
                fn from_clause(&self, out: &mut QueryBuilder) -> BuildQueryResult {
                    out.push_identifier(stringify!($name))
                }
            }

            impl AsQuery for table {
                type SqlType = SqlType;
                type Query = SelectStatement<SqlType, ($($column_name),+), Self>;

                fn as_query(self) -> Self::Query {
                    SelectStatement::simple(all_columns, self)
                }
            }

            impl Table for table {
                type PrimaryKey = columns::$pk;
                type AllColumns = ($($column_name),+);

                fn name() -> &'static str {
                    stringify!($name)
                }

                fn primary_key(&self) -> Self::PrimaryKey {
                    columns::$pk
                }

                fn all_columns() -> Self::AllColumns {
                    ($($column_name),+)
                }
            }

            pub mod columns {
                use super::table;
                use $crate::{Table, Column, Expression, SelectableExpression};
                use $crate::query_builder::{QueryBuilder, BuildQueryResult};
                use $crate::types::*;

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy)]
                pub struct star;

                impl Expression for star {
                    type SqlType = ();

                    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
                        try!(out.push_identifier(table::name()));
                        out.push_sql(".*");
                        Ok(())
                    }
                }

                impl SelectableExpression<table> for star {}

                $(column!(table, $column_name -> $Type);)+
                $(column!(table, $no_select_column_name -> $no_select_type);)*
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
                $($field_name, $field_name -> $Type,)+
            }
        }
    };
    (
        $Struct:ty => $table_mod:ident {
            $($field_table_name:ident, $field_name:tt -> $Type:ty,)+
        }
    ) => {
        impl<'a: 'insert, 'insert> $crate::persistable::Insertable<$table_mod::table>
            for &'insert $Struct
        {
            type Columns = ($($table_mod::$field_table_name),+);
            type Values = $crate::expression::grouped::Grouped<($(
                $crate::expression::helper_types::AsExpr<&'insert $Type, $table_mod::$field_table_name>
            ),+)>;

            fn columns() -> Self::Columns {
                ($($table_mod::$field_table_name),+)
            }

            fn values(self) -> Self::Values {
                use $crate::expression::AsExpression;
                use $crate::expression::grouped::Grouped;
                Grouped(($(AsExpression::<
                   <$table_mod::$field_table_name as $crate::expression::Expression>::SqlType>
                   ::as_expression(yaqb_internal_expr_conversion!(&self.$field_name))
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
            fn join_sql(&self, out: &mut $crate::query_builder::QueryBuilder)
                -> $crate::query_builder::BuildQueryResult
            {
                try!($parent::table.from_clause(out));
                out.push_sql(" ON ");

                $child::$source.eq($parent::$target).to_sql(out)
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
macro_rules! one_to_many {
    (
        $parent_table:ident ($parent_struct:ty) ->
        $child_table:ident ($child_struct:ty) on
        ($foreign_key:ident = $primary_key:ident)
    ) => {
        one_to_many!($child_table: $parent_table ($parent_struct) ->
                     $child_table ($child_struct) on ($foreign_key = $primary_key));
    };
    (
        $association_name:ident -> $association_type:ident :
        $parent_table:ident ($parent_struct:ty) ->
        $child_table:ident ($child_struct:ty) on
        ($foreign_key:ident = $primary_key:ident)
    ) => {
        pub type $association_type = $crate::helper_types::FindBy<
            $child_table::table,
            $child_table::$foreign_key,
            i32,
        >;
        one_to_many!($association_name: $parent_table ($parent_struct) ->
                     $child_table ($child_struct) on ($foreign_key = $primary_key));
    };
    (
        $association_name:ident :
        $parent_table:ident ($parent_struct:ty) ->
        $child_table:ident ($child_struct:ty) on
        ($foreign_key:ident = $primary_key:ident)
    ) => {
        impl $parent_struct {
            pub fn $association_name(&self) -> $crate::helper_types::FindBy<
                $child_table::table,
                $child_table::$foreign_key,
                i32,
            > {
                $child_table::table.filter($child_table::$foreign_key.eq(self.$primary_key))
            }
        }

        joinable!($child_table -> $parent_table ($foreign_key = $primary_key));
    };
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

#[macro_export]
macro_rules! join_through {
    ($parent:ident -> $through:ident -> $child:ident) => {
        impl $crate::JoinTo<$child::table> for $parent::table {
            fn join_sql(&self, out: &mut $crate::query_builder::QueryBuilder)
                -> $crate::query_builder::BuildQueryResult
            {
                try!($crate::JoinTo::<$through::table>::join_sql(&$parent::table, out));
                out.push_sql(" INNER JOIN ");
                $crate::JoinTo::<$child::table>::join_sql(&$through::table, out)
            }
        }
    }
}
