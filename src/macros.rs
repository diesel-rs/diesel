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
            use $crate::{QuerySource, Table, Column};
            use $crate::types::*;
            pub use self::columns::*;

            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            pub struct table;

            pub type SqlType = ($($Type),+);

            impl QuerySource for table {
                type SqlType = SqlType;

                fn select_clause(&self) -> String {
                    star.qualified_name()
                }

                fn from_clause(&self) -> String {
                    stringify!($name).to_string()
                }
            }

            impl Table for table {
                type PrimaryKey = columns::$pk;

                fn name(&self) -> &str {
                    stringify!($name)
                }

                fn primary_key(&self) -> Self::PrimaryKey {
                    columns::$pk
                }
            }

            pub mod columns {
                use super::table;
                use $crate::{Table, Column};
                use $crate::types::*;

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug)]
                pub struct star;

                impl Column for star {
                    type Table = table;
                    type SqlType = super::SqlType;

                    fn name(&self) -> String {
                        "*".to_string()
                    }

                    fn qualified_name(&self) -> String {
                        format!("{}.*", table.name())
                    }
                }

                $(#[allow(non_camel_case_types, dead_code)]
                #[derive(Debug)]
                pub struct $column_name;

                impl Column for $column_name {
                    type Table = table;
                    type SqlType = $Type;

                    fn name(&self) -> String {
                        stringify!($column_name).to_string()
                    }

                    fn qualified_name(&self) -> String {
                        format!("{}.{}", table.name(), stringify!($column_name))
                    }
                })+
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
        impl<'a, 'b, 'c> $crate::persistable::Insertable<$table_mod::table>
            for $Struct
        {
            type Columns = ($($table_mod::$field_name),+);
            type Values = ($($Type),+);

            fn columns() -> Self::Columns {
                ($($table_mod::$field_name),+)
            }

            fn values(self) -> Self::Values {
                ($(self.$field_name),+)
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
            fn join_sql(&self) -> String {
                use $crate::Column;
                format!("{} = {}", $child::$source.qualified_name(), $parent::$target.qualified_name())
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
                <$parent::$column_name as $crate::Column>::SqlType>,
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
