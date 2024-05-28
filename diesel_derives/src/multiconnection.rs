use proc_macro2::TokenStream;
use syn::DeriveInput;

struct ConnectionVariant<'a> {
    ty: &'a syn::Type,
    name: &'a syn::Ident,
}

pub fn derive(item: DeriveInput) -> TokenStream {
    if let syn::Data::Enum(e) = item.data {
        let connection_types = e
            .variants
            .iter()
            .map(|v| match &v.fields {
                syn::Fields::Unnamed(f) if f.unnamed.len() == 1 => ConnectionVariant {
                    ty: &f.unnamed.first().unwrap().ty,
                    name: &v.ident,
                },
                _ => panic!("Only enums with on field per variant are supported"),
            })
            .collect::<Vec<_>>();
        let backend = generate_backend(&connection_types);
        let query_builder = generate_querybuilder(&connection_types);
        let bind_collector = generate_bind_collector(&connection_types);
        let row = generate_row(&connection_types);
        let connection = generate_connection_impl(&connection_types, &item.ident);

        quote::quote! {
            mod multi_connection_impl {
                use super::*;

                mod backend {
                    use super::*;
                    #backend
                }

                mod query_builder {
                    use super::*;
                    #query_builder
                }

                mod bind_collector {
                    use super::*;
                    #bind_collector
                }

                mod row {
                    use super::*;
                    #row
                }

                mod connection {
                    use super::*;
                    #connection
                }

                pub use self::backend::{MultiBackend, MultiRawValue};
                pub use self::row::{MultiRow, MultiField};
            }

            pub use self::multi_connection_impl::{MultiBackend, MultiRow, MultiRawValue, MultiField};
        }
    } else {
        panic!("Only enums are supported as multiconnection type");
    }
}

fn generate_connection_impl(
    connection_types: &[ConnectionVariant],
    ident: &syn::Ident,
) -> TokenStream {
    let batch_execute_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(conn) => conn.batch_execute(query)
        }
    });

    let execute_returning_count_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            Self::#ident(conn) => {
                let query = SerializedQuery {
                    inner: source,
                    backend: MultiBackend::#ident(Default::default()),
                    query_builder: super::query_builder::MultiQueryBuilder::#ident(Default::default()),
                    p: std::marker::PhantomData::<#ty>,
                };
                conn.execute_returning_count(&query)
            }
        }
    });

    let load_impl = connection_types.iter().map(|c| {
        let variant_ident = c.name;
        let ty = &c.ty;
        quote::quote! {
            #ident::#variant_ident(conn) => {
                let query = SerializedQuery {
                    inner: source,
                    backend: MultiBackend::#variant_ident(Default::default()),
                    query_builder: super::query_builder::MultiQueryBuilder::#variant_ident(Default::default()),
                    p: std::marker::PhantomData::<#ty>,
                };
                let r = <#ty as diesel::connection::LoadConnection>::load(conn, query)?;
                Ok(super::row::MultiCursor::#variant_ident(r))
            }
        }
    });

    let instrumentation_impl = connection_types.iter().map(|c| {
        let variant_ident = c.name;
        quote::quote! {
            #ident::#variant_ident(conn) => {
                diesel::connection::Connection::set_instrumentation(conn, instrumentation);
            }
        }
    });

    let get_instrumentation_impl = connection_types.iter().map(|c| {
        let variant_ident = c.name;
        quote::quote! {
            #ident::#variant_ident(conn) => {
                diesel::connection::Connection::instrumentation(conn)
            }
        }
    });

    let establish_impls = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            if let Ok(conn) = #ty::establish(database_url) {
                return Ok(Self::#ident(conn));
            }
        }
    });

    let begin_transaction_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            Self::#ident(conn) => <#ty as Connection>::TransactionManager::begin_transaction(conn)
        }
    });

    let commit_transaction_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            Self::#ident(conn) => <#ty as Connection>::TransactionManager::commit_transaction(conn)
        }
    });

    let rollback_transaction_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            Self::#ident(conn) => <#ty as Connection>::TransactionManager::rollback_transaction(conn)
        }
    });

    let is_broken_transaction_manager_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            Self::#ident(conn) => <#ty as Connection>::TransactionManager::is_broken_transaction_manager(conn)
        }
    });

    let transaction_manager_status_mut_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            Self::#ident(conn) => <#ty as Connection>::TransactionManager::transaction_manager_status_mut(conn)
        }
    });

    let bind_param_helper_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            impl BindParamHelper for #ty {
                fn handle_inner_pass<'a, 'b: 'a>(
                    outer_collector: &mut <Self::Backend as diesel::backend::Backend>::BindCollector<'a>,
                    lookup: &mut <Self::Backend as diesel::sql_types::TypeMetadata>::MetadataLookup,
                    backend: &'b MultiBackend,
                    q: &'b impl diesel::query_builder::QueryFragment<MultiBackend>,
                ) -> diesel::QueryResult<()> {
                    use diesel::internal::derives::multiconnection::MultiConnectionHelper;

                    let mut collector = super::bind_collector::MultiBindCollector::#ident(Default::default());
                    let lookup = Self::to_any(lookup);
                    q.collect_binds(&mut collector, lookup, backend)?;
                    if let super::bind_collector::MultiBindCollector::#ident(collector) = collector {
                        *outer_collector = collector;
                    }
                    Ok(())
                }
            }
        }
    });

    let impl_migration_connection = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(conn) => {
                use diesel::migration::MigrationConnection;
                conn.setup()
            }
        }
    });

    let impl_begin_test_transaction = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(conn) => conn.begin_test_transaction()
        }
    });

    let r2d2_impl = if cfg!(feature = "r2d2") {
        let impl_ping_r2d2 = connection_types.iter().map(|c| {
            let ident = c.name;
            quote::quote! {
                Self::#ident(conn) => conn.ping()
            }
        });

        let impl_is_broken_r2d2 = connection_types.iter().map(|c| {
            let ident = c.name;
            quote::quote! {
                Self::#ident(conn) => conn.is_broken()
            }
        });
        Some(quote::quote! {
            impl diesel::r2d2::R2D2Connection for MultiConnection {
                fn ping(&mut self) -> diesel::QueryResult<()> {
                    use diesel::r2d2::R2D2Connection;
                    match self {
                        #(#impl_ping_r2d2,)*
                    }
                }

                fn is_broken(&mut self) -> bool {
                    use diesel::r2d2::R2D2Connection;
                    match self {
                        #(#impl_is_broken_r2d2,)*
                    }
                }
            }
        })
    } else {
        None
    };

    quote::quote! {
        use diesel::connection::*;
        pub(super) use super::#ident as MultiConnection;

        impl SimpleConnection for MultiConnection {
            fn batch_execute(&mut self, query: &str) -> diesel::result::QueryResult<()> {
                match self {
                    #(#batch_execute_impl,)*
                }
            }
        }

        impl diesel::internal::derives::multiconnection::ConnectionSealed for MultiConnection {}

        struct SerializedQuery<T, C> {
            inner: T,
            backend: MultiBackend,
            query_builder: super::query_builder::MultiQueryBuilder,
            p: std::marker::PhantomData<C>,
        }

        trait BindParamHelper: Connection {
            fn handle_inner_pass<'a, 'b: 'a>(
                collector: &mut <Self::Backend as diesel::backend::Backend>::BindCollector<'a>,
                lookup: &mut <Self::Backend as diesel::sql_types::TypeMetadata>::MetadataLookup,
                backend: &'b MultiBackend,
                q: &'b impl diesel::query_builder::QueryFragment<MultiBackend>,
            ) -> diesel::QueryResult<()>;
        }

        #(#bind_param_helper_impl)*

        impl<T, DB, C> diesel::query_builder::QueryFragment<DB> for SerializedQuery<T, C>
        where
            DB: diesel::backend::Backend + 'static,
            T: diesel::query_builder::QueryFragment<MultiBackend>,
            C: diesel::connection::Connection<Backend = DB> + BindParamHelper + diesel::internal::derives::multiconnection::MultiConnectionHelper,
        {
            fn walk_ast<'b>(
                &'b self,
                mut pass: diesel::query_builder::AstPass<'_, 'b, DB>,
            ) -> diesel::QueryResult<()> {
                use diesel::query_builder::QueryBuilder;
                use diesel::internal::derives::multiconnection::AstPassHelper;

                let mut query_builder = self.query_builder.duplicate();
                self.inner.to_sql(&mut query_builder, &self.backend)?;
                pass.push_sql(&query_builder.finish());
                if !self.inner.is_safe_to_cache_prepared(&self.backend)? {
                    pass.unsafe_to_cache_prepared();
                }
                if let Some((outer_collector, lookup)) = pass.bind_collector() {
                    C::handle_inner_pass(outer_collector, lookup, &self.backend, &self.inner)?;
                }
                Ok(())
            }
        }

        impl<T, C> diesel::query_builder::QueryId for SerializedQuery<T, C>
        where
            T: diesel::query_builder::QueryId,
        {
            type QueryId = <T as diesel::query_builder::QueryId>::QueryId;

            const HAS_STATIC_QUERY_ID: bool = <T as diesel::query_builder::QueryId>::HAS_STATIC_QUERY_ID;
        }

        impl<T, C> diesel::query_builder::Query for SerializedQuery<T, C>
        where
            T: diesel::query_builder::Query
        {
            // we use untyped here as this does not really matter
            // + that type is supported for all backends
            type SqlType = diesel::sql_types::Untyped;
        }

        impl Connection for MultiConnection {
            type Backend = super::MultiBackend;

            type TransactionManager = Self;

            fn establish(database_url: &str) -> diesel::ConnectionResult<Self> {
                #(#establish_impls)*
                Err(diesel::ConnectionError::BadConnection("Invalid connection url for multiconnection".into()))
            }

            fn execute_returning_count<T>(&mut self, source: &T) -> diesel::result::QueryResult<usize>
            where
                T: diesel::query_builder::QueryFragment<Self::Backend> + diesel::query_builder::QueryId,
            {
                match self {
                    #(#execute_returning_count_impl,)*
                }
            }

            fn transaction_state(
                &mut self,
            ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData {
                self
            }

            fn instrumentation(&mut self) -> &mut dyn diesel::connection::Instrumentation {
                match self {
                    #(#get_instrumentation_impl,)*
                }
            }

            fn set_instrumentation(&mut self, instrumentation: impl diesel::connection::Instrumentation) {
                match self {
                    #(#instrumentation_impl,)*
                }
            }

            fn begin_test_transaction(&mut self) -> diesel::QueryResult<()> {
                match self {
                    #(#impl_begin_test_transaction,)*
                }
            }
        }

        impl LoadConnection for MultiConnection
        {
            type Cursor<'conn, 'query> = super::row::MultiCursor<'conn, 'query>;
            type Row<'conn, 'query> = super::MultiRow<'conn, 'query>;

            fn load<'conn, 'query, T>(
                &'conn mut self,
                source: T,
            ) -> diesel::result::QueryResult<Self::Cursor<'conn, 'query>>
            where
                T: diesel::query_builder::Query + diesel::query_builder::QueryFragment<Self::Backend> + diesel::query_builder::QueryId + 'query,
                Self::Backend: diesel::expression::QueryMetadata<T::SqlType>,
            {
                match self {
                    #(#load_impl,)*
                }
            }
        }

        impl TransactionManager<MultiConnection> for MultiConnection {
            type TransactionStateData = Self;

            fn begin_transaction(conn: &mut MultiConnection) -> diesel::QueryResult<()> {
                match conn {
                    #(#begin_transaction_impl,)*
                }
            }

            fn rollback_transaction(conn: &mut MultiConnection) -> diesel::QueryResult<()> {
                match conn {
                    #(#rollback_transaction_impl,)*
                }
            }

            fn commit_transaction(conn: &mut MultiConnection) -> diesel::QueryResult<()> {
                match conn {
                    #(#commit_transaction_impl,)*
                }
            }

            fn transaction_manager_status_mut(conn: &mut MultiConnection) -> &mut diesel::connection::TransactionManagerStatus {
                match conn {
                    #(#transaction_manager_status_mut_impl,)*
                }
            }

            fn is_broken_transaction_manager(conn: &mut MultiConnection) -> bool {
                match conn {
                    #(#is_broken_transaction_manager_impl,)*
                }
            }
        }

        impl diesel::migration::MigrationConnection for MultiConnection {
            fn setup(&mut self) -> diesel::QueryResult<usize> {
                match self {
                    #(#impl_migration_connection,)*
                }
            }
        }

        #r2d2_impl
    }
}

fn generate_row(connection_types: &[ConnectionVariant]) -> TokenStream {
    let row_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<#ty as diesel::connection::LoadConnection>::Row<'conn, 'query>)
        }
    });

    let field_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<<#ty as diesel::connection::LoadConnection>::Row<'conn, 'query> as diesel::row::Row<'conn, <#ty as diesel::connection::Connection>::Backend>>::Field<'query>)
        }
    });

    let field_name_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(f) => f.field_name()
        }
    });

    let field_value_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(f) => f.value().map(super::MultiRawValue::#ident)
        }
    });

    let row_index_impl = connection_types
        .iter()
        .map(|c| {
            let ident = c.name;
            quote::quote! {
                Self::#ident(r) => r.idx(idx)
            }
        })
        .collect::<Vec<_>>();
    let row_index_impl = &row_index_impl;

    let cursor_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<#ty as diesel::connection::LoadConnection>::Cursor<'conn, 'query>)
        }
    });

    let iterator_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(r) => Some(r.next()?.map(MultiRow::#ident))
        }
    });

    let field_count_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(r) => r.field_count()
        }
    });

    let get_field_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(r) => r.get(idx).map(MultiField::#ident)
        }
    });

    quote::quote! {

        pub enum MultiRow<'conn, 'query> {
            #(#row_variants,)*

        }

        impl<'conn, 'query> diesel::internal::derives::multiconnection::RowSealed for MultiRow<'conn, 'query> {}

        pub enum MultiField<'conn: 'query, 'query> {
            #(#field_variants,)*
        }

        impl<'conn, 'query> diesel::row::Field<'conn, super::MultiBackend> for MultiField<'conn, 'query> {
            fn field_name(&self) -> Option<&str> {
                use diesel::row::Field;

                match self {
                    #(#field_name_impl,)*
                }
            }

            fn value(&self) -> Option<<super::MultiBackend as diesel::backend::Backend>::RawValue<'_>> {
                use diesel::row::Field;

                match self {
                    #(#field_value_impl,)*
                }
            }
        }

        impl<'conn, 'query, 'c> diesel::row::RowIndex<&'c str> for MultiRow<'conn, 'query> {
            fn idx(&self, idx: &'c str) -> Option<usize> {
                use diesel::row::RowIndex;

                match self {
                    #(#row_index_impl,)*
                }
            }
        }

        impl<'conn, 'query> diesel::row::RowIndex<usize> for MultiRow<'conn, 'query> {
            fn idx(&self, idx: usize) -> Option<usize> {
                use diesel::row::RowIndex;

                match self {
                    #(#row_index_impl,)*
                }
            }
        }

        impl<'conn, 'query> diesel::row::Row<'conn, super::MultiBackend> for MultiRow<'conn, 'query> {
            type Field<'a> = MultiField<'a, 'a> where 'conn: 'a, Self: 'a;
            type InnerPartialRow = Self;

            fn field_count(&self) -> usize {
                use diesel::row::Row;
                match self {
                    #(#field_count_impl,)*
                }
            }

            fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
            where
                'conn: 'b,
                Self: diesel::row::RowIndex<I>,
            {
                use diesel::row::{RowIndex, Row};
                let idx = self.idx(idx)?;

                match self {
                    #(#get_field_impl,)*
                }
            }

            fn partial_row(
                &self,
                range: std::ops::Range<usize>,
            ) -> diesel::internal::derives::multiconnection::PartialRow<'_, Self::InnerPartialRow> {
                diesel::internal::derives::multiconnection::PartialRow::new(self, range)
            }
        }

        pub enum MultiCursor<'conn, 'query> {
            #(#cursor_variants,)*
        }

        impl<'conn, 'query> Iterator for MultiCursor<'conn, 'query> {
            type Item = diesel::QueryResult<MultiRow<'conn, 'query>>;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    #(#iterator_impl,)*
                }
            }
        }

    }
}

fn generate_bind_collector(connection_types: &[ConnectionVariant]) -> TokenStream {
    let mut to_sql_impls = vec![
        (
            quote::quote!(diesel::sql_types::SmallInt),
            quote::quote!(i16),
        ),
        (
            quote::quote!(diesel::sql_types::Integer),
            quote::quote!(i32),
        ),
        (quote::quote!(diesel::sql_types::BigInt), quote::quote!(i64)),
        (quote::quote!(diesel::sql_types::Double), quote::quote!(f64)),
        (quote::quote!(diesel::sql_types::Float), quote::quote!(f32)),
        (quote::quote!(diesel::sql_types::Text), quote::quote!(str)),
        (
            quote::quote!(diesel::sql_types::Binary),
            quote::quote!([u8]),
        ),
        (quote::quote!(diesel::sql_types::Bool), quote::quote!(bool)),
    ];
    if cfg!(feature = "chrono") {
        to_sql_impls.push((
            quote::quote!(diesel::sql_types::Timestamp),
            quote::quote!(diesel::internal::derives::multiconnection::chrono::NaiveDateTime),
        ));
        to_sql_impls.push((
            quote::quote!(diesel::sql_types::Date),
            quote::quote!(diesel::internal::derives::multiconnection::chrono::NaiveDate),
        ));
        to_sql_impls.push((
            quote::quote!(diesel::sql_types::Time),
            quote::quote!(diesel::internal::derives::multiconnection::chrono::NaiveTime),
        ));
    }
    if cfg!(feature = "time") {
        to_sql_impls.push((
            quote::quote!(diesel::sql_types::Timestamp),
            quote::quote!(diesel::internal::derives::multiconnection::time::PrimitiveDateTime),
        ));
        to_sql_impls.push((
            quote::quote!(diesel::sql_types::Time),
            quote::quote!(diesel::internal::derives::multiconnection::time::Time),
        ));
        to_sql_impls.push((
            quote::quote!(diesel::sql_types::Date),
            quote::quote!(diesel::internal::derives::multiconnection::time::Date),
        ));
    }
    let to_sql_impls = to_sql_impls
        .into_iter()
        .map(|t| generate_to_sql_impls(t, connection_types));

    let mut from_sql_impls = vec![
        (
            quote::quote!(diesel::sql_types::SmallInt),
            quote::quote!(i16),
        ),
        (
            quote::quote!(diesel::sql_types::Integer),
            quote::quote!(i32),
        ),
        (quote::quote!(diesel::sql_types::BigInt), quote::quote!(i64)),
        (quote::quote!(diesel::sql_types::Double), quote::quote!(f64)),
        (quote::quote!(diesel::sql_types::Float), quote::quote!(f32)),
        (
            quote::quote!(diesel::sql_types::Text),
            quote::quote!(String),
        ),
        (
            quote::quote!(diesel::sql_types::Binary),
            quote::quote!(Vec<u8>),
        ),
        (quote::quote!(diesel::sql_types::Bool), quote::quote!(bool)),
    ];
    if cfg!(feature = "chrono") {
        from_sql_impls.push((
            quote::quote!(diesel::sql_types::Timestamp),
            quote::quote!(diesel::internal::derives::multiconnection::chrono::NaiveDateTime),
        ));
        from_sql_impls.push((
            quote::quote!(diesel::sql_types::Date),
            quote::quote!(diesel::internal::derives::multiconnection::chrono::NaiveDate),
        ));
        from_sql_impls.push((
            quote::quote!(diesel::sql_types::Time),
            quote::quote!(diesel::internal::derives::multiconnection::chrono::NaiveTime),
        ));
    }
    if cfg!(feature = "time") {
        from_sql_impls.push((
            quote::quote!(diesel::sql_types::Timestamp),
            quote::quote!(diesel::internal::derives::multiconnection::time::PrimitiveDateTime),
        ));
        from_sql_impls.push((
            quote::quote!(diesel::sql_types::Time),
            quote::quote!(diesel::internal::derives::multiconnection::time::Time),
        ));
        from_sql_impls.push((
            quote::quote!(diesel::sql_types::Date),
            quote::quote!(diesel::internal::derives::multiconnection::time::Date),
        ));
    }
    let from_sql_impls = from_sql_impls.into_iter().map(generate_from_sql_impls);

    let into_bind_value_bounds = connection_types.iter().map(|c| {
        let ty = c.ty;
        quote::quote! {
            diesel::serialize::ToSql<ST, <#ty as diesel::connection::Connection>::Backend>
        }
    });

    let has_sql_type_bounds = connection_types.iter().map(|c| {
        let ty = c.ty;
        quote::quote! {
            <#ty as diesel::connection::Connection>::Backend: diesel::sql_types::HasSqlType<ST>
        }
    });

    let multi_bind_collector_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<<#ty as diesel::connection::Connection>::Backend as diesel::backend::Backend>::BindCollector<'a>)
        }
    });

    let multi_bind_collector_accessor = connection_types.iter().map(|c| {
        let ident = c.name;
        let lower_ident = syn::Ident::new(&c.name.to_string().to_lowercase(), c.name.span());
        let ty = c.ty;
        quote::quote! {
            pub(super) fn #lower_ident(
                &mut self,
            ) -> &mut <<#ty as diesel::connection::Connection>::Backend as diesel::backend::Backend>::BindCollector<'a> {
                match self {
                    Self::#ident(bc) => bc,
                    _ => unreachable!(),
                }
            }

        }

    });

    let push_to_inner_collector = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            Self::#ident(ref mut bc) => {
                let out = out.inner.expect("This inner value is set via our custom `ToSql` impls");
                let callback = out.push_bound_value_to_collector;
                let value = out.value;
                <_ as PushBoundValueToCollectorDB<<#ty as diesel::Connection>::Backend>>::push_bound_value(
                     callback,
                     value,
                     bc,
                     <#ty as diesel::internal::derives::multiconnection::MultiConnectionHelper>::from_any(metadata_lookup)
                        .expect("We can downcast the metadata lookup to the right type")
                 )?
            }
        }
    });

    let push_null_to_inner_collector = connection_types
        .iter()
        .map(|c| {
            let ident = c.name;
            quote::quote! {
                (Self::#ident(ref mut bc), super::backend::MultiTypeMetadata{ #ident: Some(metadata), .. }) => {
                    bc.push_null_value(metadata)?;
                }
            }
        })
        .collect::<Vec<_>>();

    let push_bound_value_super_traits = connection_types
        .iter()
        .map(|c| {
            let ty = c.ty;
            quote::quote! {
                PushBoundValueToCollectorDB<<#ty as diesel::Connection>::Backend>
            }
        })
        .collect::<Vec<_>>();

    quote::quote! {
        pub enum MultiBindCollector<'a> {
            #(#multi_bind_collector_variants,)*
        }

        impl<'a> MultiBindCollector<'a> {
            #(#multi_bind_collector_accessor)*
        }

        trait PushBoundValueToCollectorDB<DB: diesel::backend::Backend> {
            fn push_bound_value<'a: 'b, 'b>(
                &self,
                v: InnerBindValueKind<'a>,
                collector: &mut <DB as diesel::backend::Backend>::BindCollector<'b>,
                lookup: &mut <DB as diesel::sql_types::TypeMetadata>::MetadataLookup,
            ) -> diesel::result::QueryResult<()>;
        }

        struct PushBoundValueToCollectorImpl<ST, T: ?Sized> {
            p: std::marker::PhantomData<(ST, T)>
        }

        // we need to have separate impls for Sized values and str/[u8] as otherwise
        // we need separate impls for `Sized` and `str`/`[u8]` here as
        // we cannot use `Any::downcast_ref` otherwise (which implies `Sized`)
        impl<ST, T, DB> PushBoundValueToCollectorDB<DB> for PushBoundValueToCollectorImpl<ST, T>
        where DB: diesel::backend::Backend
                  + diesel::sql_types::HasSqlType<ST>,
              T: diesel::serialize::ToSql<ST, DB> + 'static,
              Option<T>: diesel::serialize::ToSql<diesel::sql_types::Nullable<ST>, DB> + 'static,
              ST: diesel::sql_types::SqlType,
        {
            fn push_bound_value<'a: 'b, 'b>(
                &self,
                v: InnerBindValueKind<'a>,
                collector: &mut <DB as diesel::backend::Backend>::BindCollector<'b>,
                lookup: &mut <DB as diesel::sql_types::TypeMetadata>::MetadataLookup,
            ) -> diesel::result::QueryResult<()> {
                use diesel::query_builder::BindCollector;
                match v {
                    InnerBindValueKind::Sized(v) => {
                        let v = v.downcast_ref::<T>().expect("We know the type statically here");
                        collector.push_bound_value::<ST, T>(v, lookup)
                    }
                    InnerBindValueKind::Null => {
                        collector.push_bound_value::<diesel::sql_types::Nullable<ST>, Option<T>>(&None, lookup)
                    },
                    _ => unreachable!("We set the value to `InnerBindValueKind::Sized` or `InnerBindValueKind::Null`")
                }
            }
        }

        impl<DB> PushBoundValueToCollectorDB<DB> for PushBoundValueToCollectorImpl<diesel::sql_types::Text, str>
        where DB: diesel::backend::Backend + diesel::sql_types::HasSqlType<diesel::sql_types::Text>,
              str: diesel::serialize::ToSql<diesel::sql_types::Text, DB> + 'static,
        {
            fn push_bound_value<'a: 'b, 'b>(
                &self,
                v: InnerBindValueKind<'a>,
                collector: &mut <DB as diesel::backend::Backend>::BindCollector<'b>,
                lookup: &mut <DB as diesel::sql_types::TypeMetadata>::MetadataLookup,
            ) -> diesel::result::QueryResult<()> {
                use diesel::query_builder::BindCollector;
                if let InnerBindValueKind::Str(v) = v {
                    collector.push_bound_value::<diesel::sql_types::Text, str>(v, lookup)
                } else {
                    unreachable!("We set the value to `InnerBindValueKind::Str`")
                }
            }
        }

        impl<DB> PushBoundValueToCollectorDB<DB> for PushBoundValueToCollectorImpl<diesel::sql_types::Binary, [u8]>
        where DB: diesel::backend::Backend + diesel::sql_types::HasSqlType<diesel::sql_types::Binary>,
              [u8]: diesel::serialize::ToSql<diesel::sql_types::Binary, DB> + 'static,
        {
            fn push_bound_value<'a: 'b, 'b>(
                &self,
                v: InnerBindValueKind<'a>,
                collector: &mut <DB as diesel::backend::Backend>::BindCollector<'b>,
                lookup: &mut <DB as diesel::sql_types::TypeMetadata>::MetadataLookup,
            ) -> diesel::result::QueryResult<()> {
                use diesel::query_builder::BindCollector;
                if let InnerBindValueKind::Bytes(v) = v {
                    collector.push_bound_value::<diesel::sql_types::Binary, [u8]>(v, lookup)
                } else {
                    unreachable!("We set the value to `InnerBindValueKind::Binary`")
                }
            }
        }

        trait PushBoundValueToCollector: #(#push_bound_value_super_traits +)* {}

        impl<T> PushBoundValueToCollector for T
        where T: #(#push_bound_value_super_traits + )* {}

        #[derive(Default)]
        pub struct BindValue<'a> {
            // we use an option here to initialize an "empty"
            // as part of the `BindCollector` impl below
            inner: Option<InnerBindValue<'a>>
        }

        struct InnerBindValue<'a> {
            value: InnerBindValueKind<'a>,
            push_bound_value_to_collector: &'static dyn PushBoundValueToCollector
        }

        enum InnerBindValueKind<'a> {
            Sized(&'a (dyn std::any::Any + std::marker::Send + std::marker::Sync)),
            Str(&'a str),
            Bytes(&'a [u8]),
            Null,
        }

        impl<'a> From<(diesel::sql_types::Text, &'a str)> for BindValue<'a> {
            fn from((_, v): (diesel::sql_types::Text, &'a str)) -> Self {
                Self {
                    inner: Some(InnerBindValue{
                        value: InnerBindValueKind::Str(v),
                        push_bound_value_to_collector: &PushBoundValueToCollectorImpl {
                            p: std::marker::PhantomData::<(diesel::sql_types::Text, str)>
                        }
                    })
                }
            }
        }

        impl<'a> From<(diesel::sql_types::Binary, &'a [u8])> for BindValue<'a> {
            fn from((_, v): (diesel::sql_types::Binary, &'a [u8])) -> Self {
                Self {
                    inner: Some(InnerBindValue {
                        value: InnerBindValueKind::Bytes(v),
                        push_bound_value_to_collector: &PushBoundValueToCollectorImpl {
                            p: std::marker::PhantomData::<(diesel::sql_types::Binary, [u8])>
                        }
                    })
                }
            }
        }

        impl<'a, T, ST> From<(ST, &'a T)> for BindValue<'a>
        where
            T: std::any::Any #(+ #into_bind_value_bounds)* + Send + Sync + 'static,
            ST: Send + diesel::sql_types::SqlType<IsNull = diesel::sql_types::is_nullable::NotNull> + 'static,
            #(#has_sql_type_bounds,)*
        {
            fn from((_, v): (ST, &'a T)) -> Self {
                Self {
                    inner: Some(InnerBindValue{
                        value: InnerBindValueKind::Sized(v),
                        push_bound_value_to_collector: &PushBoundValueToCollectorImpl {
                            p: std::marker::PhantomData::<(ST, T)>
                        }
                    })
                }
            }
        }

        impl<'a> diesel::query_builder::BindCollector<'a, MultiBackend> for MultiBindCollector<'a> {
            type Buffer = multi_connection_impl::bind_collector::BindValue<'a>;

            fn push_bound_value<T, U>(
                &mut self,
                bind: &'a U,
                metadata_lookup: &mut (dyn std::any::Any + 'static),
            ) -> diesel::QueryResult<()>
            where
                MultiBackend: diesel::sql_types::HasSqlType<T>,
                U: diesel::serialize::ToSql<T, MultiBackend> + ?Sized + 'a,
            {
                let out = {
                    let out = multi_connection_impl::bind_collector::BindValue::default();
                    let mut out =
                        diesel::serialize::Output::<MultiBackend>::new(out, metadata_lookup);
                    let bind_is_null = bind.to_sql(&mut out).map_err(diesel::result::Error::SerializationError)?;
                    if matches!(bind_is_null, diesel::serialize::IsNull::Yes) {
                        // nulls are special and need a special handling because
                        // there is a wildcard `ToSql` impl in diesel. That means we won't
                        // set the `inner` field of `BindValue` to something for the `None`
                        // case. Therefore we need to handle that explicitly here.
                        //
                        let metadata = <MultiBackend as diesel::sql_types::HasSqlType<T>>::metadata(metadata_lookup);
                        match (self, metadata) {
                            #(#push_null_to_inner_collector)*
                            _ => {
                                unreachable!("We have matching metadata")
                            },
                        }
                       return Ok(());
                    } else {
                        out.into_inner()
                    }
                };
                match self {
                    #(#push_to_inner_collector)*
                }

                Ok(())
            }

            fn push_null_value(&mut self, metadata: super::backend::MultiTypeMetadata) -> diesel::QueryResult<()> {
                match (self, metadata) {
                    #(#push_null_to_inner_collector)*
                    _ => unreachable!("We have matching metadata"),
                }
                Ok(())
            }
        }

        #(#to_sql_impls)*
        #(#from_sql_impls)*

    }
}

fn generate_has_sql_type_impls(sql_type: TokenStream) -> TokenStream {
    quote::quote! {
        impl diesel::sql_types::HasSqlType<#sql_type> for super::MultiBackend {
            fn metadata(lookup: &mut Self::MetadataLookup) -> Self::TypeMetadata {
                Self::lookup_sql_type::<#sql_type>(lookup)
            }
        }
    }
}

fn generate_from_sql_impls((sql_type, tpe): (TokenStream, TokenStream)) -> TokenStream {
    quote::quote! {
        impl diesel::deserialize::FromSql<#sql_type, super::MultiBackend> for #tpe {
            fn from_sql(
                bytes: <super::MultiBackend as diesel::backend::Backend>::RawValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                bytes.from_sql::<Self, #sql_type>()
            }
        }

    }
}

fn generate_to_sql_impls(
    (sql_type, tpe): (TokenStream, TokenStream),
    _connection_types: &[ConnectionVariant],
) -> TokenStream {
    quote::quote! {
        impl diesel::serialize::ToSql<#sql_type, super::MultiBackend> for #tpe {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, super::MultiBackend>,
            ) -> diesel::serialize::Result {
                out.set_value((#sql_type, self));
                Ok(diesel::serialize::IsNull::No)
            }
        }
    }
}

fn generate_queryfragment_impls(
    trait_def: TokenStream,
    query_fragment_bounds: &[TokenStream],
) -> TokenStream {
    quote::quote! {
        impl #trait_def
        where
            Self: #(#query_fragment_bounds+)*
        {
            fn walk_ast<'b>(
                &'b self,
                pass: diesel::query_builder::AstPass<'_, 'b, MultiBackend>,
            ) -> diesel::QueryResult<()> {
                super::backend::MultiBackend::walk_variant_ast(self, pass)
            }
        }
    }
}

fn generate_querybuilder(connection_types: &[ConnectionVariant]) -> TokenStream {
    let variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<<#ty as diesel::Connection>::Backend as diesel::backend::Backend>::QueryBuilder)
        }
    });

    let push_sql_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(q) => q.push_sql(sql)
        }
    });

    let push_identifier_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(q) => q.push_identifier(identifier)
        }
    });

    let push_bind_param_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(q) => q.push_bind_param()
        }
    });

    let finish_impl = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(q) => q.finish()
        }
    });

    let into_variant_functions = connection_types.iter().map(|c|{
        let ty = c.ty;
        let ident = c.name;
        let lower_ident = syn::Ident::new(&ident.to_string().to_lowercase(), ident.span());
        quote::quote! {
            pub(super) fn #lower_ident(&mut self) -> &mut <<#ty as diesel::Connection>::Backend as diesel::backend::Backend>::QueryBuilder {
                match self {
                    Self::#ident(qb) => qb,
                    _ => unreachable!(),
                }
            }
        }
    });

    let query_fragment_bounds = connection_types.iter().map(|c| {
        let ty = c.ty;
        quote::quote! {
            diesel::query_builder::QueryFragment<<#ty as diesel::connection::Connection>::Backend>
        }
    }).collect::<Vec<_>>();

    let duplicate_query_builder = connection_types.iter().map(|c| {
        let ident = c.name;
        quote::quote! {
            Self::#ident(_) => Self::#ident(Default::default())
        }
    });

    let query_fragment = quote::quote! {
        diesel::query_builder::QueryFragment<super::backend::MultiBackend>
    };

    let query_fragment_impls = IntoIterator::into_iter([
        quote::quote!{
            <L, O> #query_fragment for diesel::internal::derives::multiconnection::LimitOffsetClause<L, O>
        },
        quote::quote! {
            <L, R> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiConcatClauseSyntax>
                for diesel::internal::derives::multiconnection::Concat<L, R>
        },
        quote::quote! {
            <T, U> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiArrayComparisonSyntax>
                for diesel::internal::derives::multiconnection::array_comparison::In<T, U>
        },
        quote::quote! {
            <T, U> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiArrayComparisonSyntax>
                for diesel::internal::derives::multiconnection::array_comparison::NotIn<T, U>
        },
        quote::quote! {
            <ST, I> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiArrayComparisonSyntax>
                for diesel::internal::derives::multiconnection::array_comparison::Many<ST, I>
        },
        quote::quote! {
            <T> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiExistsSyntax>
                for diesel::internal::derives::multiconnection::Exists<T>
        },
        quote::quote! {
            diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiEmptyFromClauseSyntax>
                for diesel::internal::derives::multiconnection::NoFromClause
        },
        quote::quote! {
            diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiDefaultValueClauseForInsert>
                for diesel::internal::derives::multiconnection::DefaultValues
        },
        quote::quote! {
            <Expr> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiReturningClause>
                for diesel::internal::derives::multiconnection::ReturningClause<Expr>
        },
        quote::quote! {
            <Expr> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiInsertWithDefaultKeyword>
                for diesel::insertable::DefaultableColumnInsertValue<Expr>
        },
        quote::quote! {
            <Tab, V, QId, const HAS_STATIC_QUERY_ID: bool> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiBatchInsertSupport>
                for diesel::internal::derives::multiconnection::BatchInsert<V, Tab, QId, HAS_STATIC_QUERY_ID>
        },
        quote::quote! {
            <S> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiAliasSyntax>
                for diesel::query_source::Alias<S>
        }
    ])
    .map(|t| generate_queryfragment_impls(t, &query_fragment_bounds));

    let insert_values_impl_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let lower_ident = syn::Ident::new(&ident.to_string().to_lowercase(), c.name.span());
        let ty = c.ty;
        quote::quote! {
            super::backend::MultiBackend::#ident(_) => {
                <Self as diesel::insertable::InsertValues<<#ty as diesel::connection::Connection>::Backend, Col::Table>>::column_names(
                    &self,
                    out.cast_database(
                        super::bind_collector::MultiBindCollector::#lower_ident,
                        super::query_builder::MultiQueryBuilder::#lower_ident,
                        super::backend::MultiBackend::#lower_ident,
                        |l| {
                            <#ty as diesel::internal::derives::multiconnection::MultiConnectionHelper>::from_any(l)
                                .expect("It's possible to downcast the metadata lookup type to the correct type")
                        },
                    ),
                )
            }
        }
    });

    let insert_values_backend_bounds = connection_types.iter().map(|c| {
        let ty = c.ty;
        quote::quote! {
            diesel::insertable::DefaultableColumnInsertValue<diesel::insertable::ColumnInsertValue<Col, Expr>>: diesel::insertable::InsertValues<<#ty as diesel::connection::Connection>::Backend, Col::Table>
        }
    });

    quote::quote! {
        pub enum MultiQueryBuilder {
            #(#variants,)*
        }

        impl MultiQueryBuilder {
            pub(super) fn duplicate(&self) -> Self {
                match self {
                    #(#duplicate_query_builder,)*
                }
            }
        }

        impl MultiQueryBuilder {
            #(#into_variant_functions)*
        }

        impl diesel::query_builder::QueryBuilder<super::MultiBackend> for MultiQueryBuilder {
            fn push_sql(&mut self, sql: &str) {
                match self {
                    #(#push_sql_impl,)*
                }
            }

            fn push_identifier(&mut self, identifier: &str) -> diesel::QueryResult<()> {
                match self {
                    #(#push_identifier_impl,)*
                }
            }

            fn push_bind_param(&mut self) {
                match self {
                    #(#push_bind_param_impl,)*
                }
            }

            fn finish(self) -> String {
                match self {
                    #(#finish_impl,)*
                }
            }
        }

        #(#query_fragment_impls)*

        impl<F, S, D, W, O, LOf, G, H, LC>
            diesel::query_builder::QueryFragment<
                super::backend::MultiBackend,
                super::backend::MultiSelectStatementSyntax,
            >
            for diesel::internal::derives::multiconnection::SelectStatement<
                F,
                S,
                D,
                W,
                O,
                LOf,
                G,
                H,
                LC,
            >
        where
            S: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            F: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            D: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            W: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            O: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            LOf: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            G: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            H: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
            LC: diesel::query_builder::QueryFragment<super::backend::MultiBackend>,
        {
            fn walk_ast<'b>(
                &'b self,
                mut out: diesel::query_builder::AstPass<'_, 'b, MultiBackend>,
            ) -> diesel::QueryResult<()> {
                use diesel::internal::derives::multiconnection::SelectStatementAccessor;

                out.push_sql("SELECT ");
                self.distinct_clause().walk_ast(out.reborrow())?;
                self.select_clause().walk_ast(out.reborrow())?;
                self.from_clause().walk_ast(out.reborrow())?;
                self.where_clause().walk_ast(out.reborrow())?;
                self.group_by_clause().walk_ast(out.reborrow())?;
                self.having_clause().walk_ast(out.reborrow())?;
                self.order_clause().walk_ast(out.reborrow())?;
                self.limit_offset_clause().walk_ast(out.reborrow())?;
                self.locking_clause().walk_ast(out.reborrow())?;
                Ok(())
            }
        }

        impl<'a, ST, QS, GB>
            diesel::query_builder::QueryFragment<
            super::backend::MultiBackend,
            super::backend::MultiSelectStatementSyntax,
        >
            for diesel::internal::derives::multiconnection::BoxedSelectStatement<
                'a,
                ST,
                QS,
                super::backend::MultiBackend,
                GB,
            >
        where
            QS: diesel::query_builder::QueryFragment<super::backend::MultiBackend>
        {
            fn walk_ast<'b>(
                &'b self,
                pass: diesel::query_builder::AstPass<'_, 'b, MultiBackend>,
            ) -> diesel::QueryResult<()> {
                use diesel::internal::derives::multiconnection::BoxedQueryHelper;
                self.build_query(pass, |where_clause, pass| where_clause.walk_ast(pass))
            }
        }

        impl diesel::query_builder::QueryFragment<super::backend::MultiBackend>
            for diesel::internal::derives::multiconnection::BoxedLimitOffsetClause<
                '_,
                super::backend::MultiBackend,
            >
        {
            fn walk_ast<'b>(
                &'b self,
                mut pass: diesel::query_builder::AstPass<'_, 'b, MultiBackend>,
            ) -> diesel::QueryResult<()> {
                if let Some(ref limit) = self.limit {
                    limit.walk_ast(pass.reborrow())?;
                }
                if let Some(ref offset) = self.offset {
                    offset.walk_ast(pass.reborrow())?;
                }
                Ok(())
            }
        }

        impl<'a> diesel::query_builder::IntoBoxedClause<'a, super::multi_connection_impl::backend::MultiBackend>
            for diesel::internal::derives::multiconnection::LimitOffsetClause<diesel::internal::derives::multiconnection::NoLimitClause, diesel::internal::derives::multiconnection::NoOffsetClause>
        {
            type BoxedClause = diesel::internal::derives::multiconnection::BoxedLimitOffsetClause<'a, super::multi_connection_impl::backend::MultiBackend>;

            fn into_boxed(self) -> Self::BoxedClause {
                diesel::internal::derives::multiconnection::BoxedLimitOffsetClause {
                    limit: None,
                    offset: None,
                }
            }
        }
        impl<'a, L> diesel::query_builder::IntoBoxedClause<'a, super::multi_connection_impl::backend::MultiBackend>
            for diesel::internal::derives::multiconnection::LimitOffsetClause<diesel::internal::derives::multiconnection::LimitClause<L>, diesel::internal::derives::multiconnection::NoOffsetClause>
        where diesel::internal::derives::multiconnection::LimitClause<L>: diesel::query_builder::QueryFragment<super::backend::MultiBackend> + Send + 'static,
        {
            type BoxedClause = diesel::internal::derives::multiconnection::BoxedLimitOffsetClause<'a, super::multi_connection_impl::backend::MultiBackend>;
            fn into_boxed(self) -> Self::BoxedClause {
                diesel::internal::derives::multiconnection::BoxedLimitOffsetClause {
                    limit: Some(Box::new(self.limit_clause)),
                    offset: None,
                }
            }
        }
        impl<'a, O> diesel::query_builder::IntoBoxedClause<'a, super::multi_connection_impl::backend::MultiBackend>
            for diesel::internal::derives::multiconnection::LimitOffsetClause<diesel::internal::derives::multiconnection::NoLimitClause, diesel::internal::derives::multiconnection::OffsetClause<O>>
        where diesel::internal::derives::multiconnection::OffsetClause<O>: diesel::query_builder::QueryFragment<super::backend::MultiBackend> + Send + 'static,

        {
            type BoxedClause = diesel::internal::derives::multiconnection::BoxedLimitOffsetClause<'a, super::multi_connection_impl::backend::MultiBackend>;
            fn into_boxed(self) -> Self::BoxedClause {
                diesel::internal::derives::multiconnection::BoxedLimitOffsetClause {
                    limit: None,
                    offset: Some(Box::new(self.offset_clause)),
                }
            }
        }
        impl<'a, L, O> diesel::query_builder::IntoBoxedClause<'a, super::multi_connection_impl::backend::MultiBackend>
            for diesel::internal::derives::multiconnection::LimitOffsetClause<diesel::internal::derives::multiconnection::LimitClause<L>, diesel::internal::derives::multiconnection::OffsetClause<O>>
        where diesel::internal::derives::multiconnection::LimitClause<L>: diesel::query_builder::QueryFragment<super::backend::MultiBackend> + Send + 'static,
              diesel::internal::derives::multiconnection::OffsetClause<O>: diesel::query_builder::QueryFragment<super::backend::MultiBackend> + Send + 'static,
        {
            type BoxedClause = diesel::internal::derives::multiconnection::BoxedLimitOffsetClause<'a, super::multi_connection_impl::backend::MultiBackend>;
            fn into_boxed(self) -> Self::BoxedClause {
                diesel::internal::derives::multiconnection::BoxedLimitOffsetClause {
                    limit: Some(Box::new(self.limit_clause)),
                    offset: Some(Box::new(self.offset_clause)),
                }
            }
        }

        impl<Col, Expr> diesel::insertable::InsertValues<super::multi_connection_impl::backend::MultiBackend, Col::Table>
            for diesel::insertable::DefaultableColumnInsertValue<diesel::insertable::ColumnInsertValue<Col, Expr>>
        where
            Col: diesel::prelude::Column,
            Expr: diesel::prelude::Expression<SqlType = Col::SqlType>,
            Expr: diesel::prelude::AppearsOnTable<diesel::internal::derives::multiconnection::NoFromClause>,
            Self: diesel::query_builder::QueryFragment<super::multi_connection_impl::backend::MultiBackend>,
            #(#insert_values_backend_bounds,)*
        {
            fn column_names(
                &self,
                mut out: diesel::query_builder::AstPass<'_, '_, super::multi_connection_impl::backend::MultiBackend>
            ) -> QueryResult<()> {
                use diesel::internal::derives::multiconnection::AstPassHelper;

                match out.backend() {
                    #(#insert_values_impl_variants,)*
                }
            }
        }
    }
}

fn generate_backend(connection_types: &[ConnectionVariant]) -> TokenStream {
    let backend_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<#ty as diesel::Connection>::Backend)
        }
    });

    let value_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<<#ty as diesel::Connection>::Backend as diesel::backend::Backend>::RawValue<'a>)
        }
    });

    let type_metadata_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            pub(super) #ident: Option<<<#ty as diesel::Connection>::Backend as diesel::sql_types::TypeMetadata>::TypeMetadata>
        }
    });

    let has_sql_type_impls = vec![
        quote::quote!(diesel::sql_types::SmallInt),
        quote::quote!(diesel::sql_types::Integer),
        quote::quote!(diesel::sql_types::BigInt),
        quote::quote!(diesel::sql_types::Double),
        quote::quote!(diesel::sql_types::Float),
        quote::quote!(diesel::sql_types::Text),
        quote::quote!(diesel::sql_types::Binary),
        quote::quote!(diesel::sql_types::Date),
        quote::quote!(diesel::sql_types::Time),
        quote::quote!(diesel::sql_types::Timestamp),
        quote::quote!(diesel::sql_types::Bool),
    ]
    .into_iter()
    .map(generate_has_sql_type_impls);

    let into_variant_functions = connection_types.iter().map(|c| {
        let ty = c.ty;
        let ident = c.name;
        let lower_ident = syn::Ident::new(&ident.to_string().to_lowercase(), ident.span());
        quote::quote! {
            pub(super) fn #lower_ident(&self) -> &<#ty as diesel::Connection>::Backend {
                match self {
                    Self::#ident(b) => b,
                    _ => unreachable!(),
                }
            }
        }
    });

    let from_sql_match_arms = connection_types.iter().map(|v| {
        let ident = v.name;
        let ty = v.ty;
        quote::quote!{
            Self::#ident(b) => {
                <T as diesel::deserialize::FromSql<ST, <#ty as diesel::Connection>::Backend>>::from_sql(b)
            }
        }
    });

    let backend_from_sql_bounds = connection_types.iter().map(|v| {
        let ty = v.ty;
        quote::quote! {
            T: diesel::deserialize::FromSql<ST, <#ty as diesel::Connection>::Backend>
        }
    });

    let query_fragment_impl_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let lower_ident = syn::Ident::new(&ident.to_string().to_lowercase(), c.name.span());
        let ty = c.ty;
        quote::quote! {
            super::backend::MultiBackend::#ident(_) => {
                <T as diesel::query_builder::QueryFragment<<#ty as diesel::connection::Connection>::Backend>>::walk_ast(
                    ast_node,
                    pass.cast_database(
                        super::bind_collector::MultiBindCollector::#lower_ident,
                        super::query_builder::MultiQueryBuilder::#lower_ident,
                        super::backend::MultiBackend::#lower_ident,
                        |l| {
                            <#ty as diesel::internal::derives::multiconnection::MultiConnectionHelper>::from_any(l)
                                .expect("It's possible to downcast the metadata lookup type to the correct type")
                        },
                    ),
                )
            }
        }
    });

    let query_fragment_impl_bounds = connection_types.iter().map(|c| {
        let ty = c.ty;

        quote::quote! {
            T: diesel::query_builder::QueryFragment<<#ty as diesel::Connection>::Backend>
        }
    });

    let lookup_impl = connection_types.iter().map(|v| {
        let name = v.name;
        let ty = v.ty;

        quote::quote!{
            if let Some(lookup) = <#ty as diesel::internal::derives::multiconnection::MultiConnectionHelper>::from_any(lookup) {
                ret.#name = Some(<<#ty as diesel::Connection>::Backend as diesel::sql_types::HasSqlType<ST>>::metadata(lookup));
            }
        }

    });

    let lookup_sql_type_bounds = connection_types.iter().map(|c| {
        let ty = c.ty;
        quote::quote! {
            <#ty as diesel::Connection>::Backend: diesel::sql_types::HasSqlType<ST>
        }
    });

    quote::quote! {
        pub enum MultiBackend {
            #(#backend_variants,)*
        }

        impl MultiBackend {
            #(#into_variant_functions)*

            pub fn lookup_sql_type<ST>(lookup: &mut dyn std::any::Any) -> MultiTypeMetadata
            where #(#lookup_sql_type_bounds,)*
            {
                let mut ret = MultiTypeMetadata::default();
                #(#lookup_impl)*
                ret
            }
        }

        impl MultiBackend {
            pub fn walk_variant_ast<'b, T>(
                ast_node: &'b T,
                pass: diesel::query_builder::AstPass<'_, 'b, Self>,
            ) -> diesel::QueryResult<()>
            where #(#query_fragment_impl_bounds,)*
            {
                use diesel::internal::derives::multiconnection::AstPassHelper;
                match pass.backend() {
                    #(#query_fragment_impl_variants,)*
                }
            }
        }

        pub enum MultiRawValue<'a> {
            #(#value_variants,)*
        }

        impl MultiRawValue<'_> {
            pub fn from_sql<T, ST>(self) -> diesel::deserialize::Result<T>
            where #(#backend_from_sql_bounds,)*
            {
                match self {
                    #(#from_sql_match_arms,)*
                }
            }
        }

        impl diesel::backend::Backend for MultiBackend {
            type QueryBuilder = super::query_builder::MultiQueryBuilder;
            type RawValue<'a> = MultiRawValue<'a>;
            type BindCollector<'a> = super::bind_collector::MultiBindCollector<'a>;
        }

        #[derive(Default)]
        #[allow(non_snake_case)]
        pub struct MultiTypeMetadata {
            #(#type_metadata_variants,)*
        }

        impl diesel::sql_types::TypeMetadata for MultiBackend {
            type TypeMetadata = MultiTypeMetadata;

            type MetadataLookup = dyn std::any::Any;
        }

        pub struct MultiReturningClause;
        pub struct MultiInsertWithDefaultKeyword;
        pub struct MultiBatchInsertSupport;
        pub struct MultiDefaultValueClauseForInsert;
        pub struct MultiEmptyFromClauseSyntax;
        pub struct MultiExistsSyntax;
        pub struct MultiArrayComparisonSyntax;
        pub struct MultiConcatClauseSyntax;
        pub struct MultiSelectStatementSyntax;
        pub struct MultiAliasSyntax;

        impl diesel::backend::SqlDialect for MultiBackend {
            type ReturningClause = MultiReturningClause;
            // no on conflict support is also the default
            type OnConflictClause = diesel::internal::derives::multiconnection::sql_dialect::on_conflict_clause::DoesNotSupportOnConflictClause;
            type InsertWithDefaultKeyword = MultiInsertWithDefaultKeyword;
            type BatchInsertSupport = MultiBatchInsertSupport;
            type DefaultValueClauseForInsert = MultiDefaultValueClauseForInsert;
            type EmptyFromClauseSyntax = MultiEmptyFromClauseSyntax;
            type ExistsSyntax = MultiExistsSyntax;
            type ArrayComparison = MultiArrayComparisonSyntax;
            type ConcatClause = MultiConcatClauseSyntax;
            type SelectStatementSyntax = MultiSelectStatementSyntax;
            type AliasSyntax = MultiAliasSyntax;
        }

        impl diesel::internal::derives::multiconnection::TrustedBackend for MultiBackend {}
        impl diesel::internal::derives::multiconnection::DieselReserveSpecialization for MultiBackend {}

        #(#has_sql_type_impls)*
    }
}
