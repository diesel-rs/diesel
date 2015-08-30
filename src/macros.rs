macro_rules! table {
    (
        $name:ident {
            $($column_name:ident -> $Type:ident,)+
        }
    ) => {
        mod $name {
            use {types, QuerySource};

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
