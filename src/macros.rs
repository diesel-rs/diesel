macro_rules! table {
    (
        $name:ident {
            $($column_name:ident -> $Type:ident,)+
        }
    ) => {
        mod $name {
            use {types, QuerySource, Table, Column};

            #[allow(non_camel_case_types)]
            pub struct table;

            unsafe impl QuerySource for table {
                type SqlType = ($(types::$Type),+);

                fn select_clause(&self) -> &str {
                    "*"
                }

                fn from_clause(&self) -> &str {
                    stringify!($name)
                }
            }

            unsafe impl Table for table {
                fn name(&self) -> &str {
                    stringify!($name)
                }
            }

            $(
                #[allow(non_camel_case_types)]
                pub struct $column_name;

                unsafe impl Column<types::$Type, table> for $column_name {
                    fn name(&self) -> String {
                        format!("{}.{}", table.name(), stringify!($column_name))
                    }
                }
            )+
        }
    }
}

macro_rules! queriable {
    (
        $Struct:ident {
            $($field_name:ident -> $Type:ident,)+
        }
    ) => {
        impl <ST> Queriable<ST> for $Struct where
            ST: NativeSqlType,
            ($($Type),+): types::FromSql<ST>,
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
