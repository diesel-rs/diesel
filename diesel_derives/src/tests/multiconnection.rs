#[cfg(all(feature = "r2d2", feature = "chrono", feature = "time"))]
#[test]
pub(crate) fn multiconnection_1() {
    let input = quote::quote! {
        enum DbConnection {
            Pg(PgConnection),
            Sqlite(diesel::SqliteConnection),
        }
    };

    super::expand_with(
        &crate::derive_multiconnection_inner as &dyn Fn(_) -> _,
        input,
        super::derive(syn::parse_quote!(#[derive(MultiConnection)])),
        "multiconnection_1",
    );
}

#[cfg(all(feature = "r2d2", feature = "chrono", feature = "time"))]
#[test]
pub(crate) fn multiconnection_2() {
    let input = quote::quote! {
        #[diesel_async]
        enum DbConnection {
            Pg(diesel_async::AsyncPgConnection),
            Sqlite(diesel_async::AsyncMysqlConnection),
        }
    };

    super::expand_with(
        &crate::derive_multiconnection_inner as &dyn Fn(_) -> _,
        input,
        super::derive(syn::parse_quote!(#[derive(MultiConnection)])),
        "multiconnection_2",
    );
}
