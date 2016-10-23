macro_rules! str_consts {
    ($name:ident [$($key:ident => $val:expr),+]) => {
        pub mod $name {
            $(
                pub const $key: &'static str = $val;
            )+
        }
    };

    ($name:ident [$($key:ident => $val:expr),+], $all:ident) => {
        str_consts! {
            $name [
                $($key => $val),+
            ]
        }

        pub const $all: &'static [&'static str] = &[
            $(
                $name::$key,
            )+
        ];
    }
}

str_consts! {
    custom_derives [
        AS_CHANGESET => "AsChangeset",
        ASSOCIATIONS => "Associations",
        IDENTIFIABLE => "Identifiable",
        INSERTABLE   => "Insertable",
        QUERYABLE    => "Queryable"
    ],

    KNOWN_CUSTOM_DERIVES
}

str_consts! {
    custom_attrs [
        BELONGS_TO        => "belongs_to",
        CHANGESET_OPTIONS => "changeset_options",
        HAS_MANY          => "has_many",
        TABLE_NAME        => "table_name"
    ],

    KNOWN_CUSTOM_ATTRS
}

str_consts! {
    field_attrs [
        COLUMN_NAME => "column_name"
    ],

    KNOWN_FIELD_ATTRS
}

str_consts! {
    attrs [
        DERIVE  => "derive",
        OPTIONS => "options"
    ]
}

str_consts! {
    custom_attr_options [
        FOREIGN_KEY        => "foreign_key",
        DATABASE_URL       => "database_url",
        TABLE_NAME         => "table_name",
        TREAT_NONE_AS_NULL => "treat_none_as_null"
    ]
}

str_consts! {
    macro_options [
        MIGRATIONS_PATH => "migrations_path"
    ]
}

str_consts! {
    values [
        TRUE => "true"
    ]
}

str_consts! {
    syntax [
        ID          => "id",
        OPTION_TY   => "Option",
        LIFETIME_A  => "'a'",
        COLUMN_NAME => "column_name",
        FIELD_KIND  => "field_kind",
        FIELD_NAME  => "field_name",
        FIELD_TY    => "field_ty"
    ]
}

str_consts! {
    field_types [
        BARE    => "bare",
        OPTION  => "option",
        REGULAR => "regular"
    ]
}
