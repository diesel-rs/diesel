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
                    outer_collector: &mut diesel::backend::BindCollector<'a, Self::Backend>,
                    lookup: &mut <Self::Backend as diesel::sql_types::TypeMetadata>::MetadataLookup,
                    backend: &'b MultiBackend,
                    q: &'b impl diesel::query_builder::QueryFragment<MultiBackend>,
                ) -> diesel::QueryResult<()> {
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

        impl<'conn, 'query> ConnectionGatWorkaround<'conn, 'query, super::MultiBackend> for MultiConnection {
            type Cursor = super::row::MultiCursor<'conn, 'query>;
            type Row = super::MultiRow<'conn, 'query>;
        }

        struct SerializedQuery<T, C> {
            inner: T,
            backend: MultiBackend,
            query_builder: super::query_builder::MultiQueryBuilder,
            p: std::marker::PhantomData<C>,
        }

        trait BindParamHelper: Connection {
            fn handle_inner_pass<'a, 'b: 'a>(
                collector: &mut diesel::backend::BindCollector<'a, Self::Backend>,
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
        }

        #[doc(hidden)]
        pub trait Helper {
            fn load<'conn, 'query, T>(
                conn: &'conn mut MultiConnection,
                source: T,
            ) -> diesel::result::QueryResult<LoadRowIter<'conn, 'query, MultiConnection, super::MultiBackend, DefaultLoadingMode>>
            where
                T: diesel::query_builder::Query + diesel::query_builder::QueryFragment<super::MultiBackend> + diesel::query_builder::QueryId + 'query,
                super::MultiBackend: diesel::expression::QueryMetadata<T::SqlType>;
        }

        impl Helper for ()
        where
            for<'b> super::MultiBackend: diesel::backend::HasBindCollector<'b, BindCollector = super::bind_collector::MultiBindCollector<'b>>,
        {
            fn load<'conn, 'query, T>(
                conn: &'conn mut MultiConnection,
                source: T,
            ) -> diesel::result::QueryResult<LoadRowIter<'conn, 'query, MultiConnection, super::MultiBackend, DefaultLoadingMode>>
            where
                T: diesel::query_builder::Query + diesel::query_builder::QueryFragment<super::MultiBackend> + diesel::query_builder::QueryId + 'query,
                super::MultiBackend: diesel::expression::QueryMetadata<T::SqlType>,
            {

                match conn {
                    #(#load_impl,)*
                }
            }
        }

        impl LoadConnection for MultiConnection
        where
            (): Helper,
        {
            fn load<'conn, 'query, T>(
                &'conn mut self,
                source: T,
            ) -> diesel::result::QueryResult<LoadRowIter<'conn, 'query, Self, Self::Backend, DefaultLoadingMode>>
            where
                T: diesel::query_builder::Query + diesel::query_builder::QueryFragment<Self::Backend> + diesel::query_builder::QueryId + 'query,
                Self::Backend: diesel::expression::QueryMetadata<T::SqlType>,
            {
                <() as Helper>::load(self, source)
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
            #ident(<#ty as diesel::connection::ConnectionGatWorkaround<'conn, 'query, <#ty as diesel::Connection>::Backend>>::Row)
        }
    });

    let field_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<<#ty as diesel::connection::ConnectionGatWorkaround<'conn, 'query, <#ty as diesel::Connection>::Backend>>::Row as diesel::row::RowGatWorkaround<'conn, <#ty as diesel::Connection>::Backend>>::Field)
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

    let cursor_variants  = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote! {
            #ident(<#ty as diesel::connection::ConnectionGatWorkaround<'conn, 'query, <#ty as diesel::Connection>::Backend>>::Cursor)
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

        pub enum MultiField<'conn, 'query> {
            #(#field_variants,)*
        }

        impl<'conn, 'query> diesel::row::Field<'conn, super::MultiBackend> for MultiField<'conn, 'query> {
            fn field_name(&self) -> Option<&str> {
                use diesel::row::Field;

                match self {
                    #(#field_name_impl,)*
                }
            }

            fn value(&self) -> Option<diesel::backend::RawValue<'_, super::MultiBackend>> {
                use diesel::row::Field;

                match self {
                    #(#field_value_impl,)*
                }
            }
        }

        impl<'a, 'conn, 'query> diesel::row::RowGatWorkaround<'a, super::MultiBackend> for MultiRow<'conn, 'query> {
            type Field = MultiField<'a, 'a>;
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
            type InnerPartialRow = Self;

            fn field_count(&self) -> usize {
                use diesel::row::Row;
                match self {
                    #(#field_count_impl,)*
                }
            }

            fn get<'b, I>(&'b self, idx: I) -> Option<diesel::row::FieldRet<'b, Self, super::MultiBackend>>
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
            ) -> diesel::row::PartialRow<'_, Self::InnerPartialRow> {
                diesel::row::PartialRow::new(self, range)
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
    let to_sql_impls = vec![
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
    ]
    .into_iter()
    .map(|t| generate_to_sql_impls(t, connection_types));

    let from_sql_impls = vec![
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
            quote::quote!(*const str),
        ),
        (
            quote::quote!(diesel::sql_types::Binary),
            quote::quote!(*const [u8]),
        ),
    ]
    .into_iter()
    .map(generate_from_sql_impls);

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
            #ident(<<#ty as diesel::connection::Connection>::Backend as diesel::backend::HasBindCollector<'a>>::BindCollector)
        }
    });

    let multi_bind_collector_accessor = connection_types.iter().map(|c| {
        let ident = c.name;
        let lower_ident = syn::Ident::new(&c.name.to_string().to_lowercase(), c.name.span());
        let ty = c.ty;
        quote::quote! {
            pub(super) fn #lower_ident(
                &mut self,
            ) -> &mut <<#ty as diesel::connection::Connection>::Backend as diesel::backend::HasBindCollector<'a>>::BindCollector {
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
                let out = out.inner.unwrap();
                let callback = out.push_bound_value_to_collector;
                let value = out.value;
                <_ as PushBoundValueToCollectorDB<<#ty as diesel::Connection>::Backend>>::push_bound_value(
                     callback,
                     value,
                     bc,
                     <#ty as diesel::internal::derives::multiconnection::MultiConnectionHelper>::from_any(metadata_lookup).unwrap()
                 )?
            }
        }
    });

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
                collector: &mut diesel::backend::BindCollector<'b, DB>,
                lookup: &mut <DB as diesel::sql_types::TypeMetadata>::MetadataLookup,
            ) -> diesel::result::QueryResult<()>;
        }

        struct PushBoundValueToCollectorImpl<ST, T: ?Sized> {
            p: std::marker::PhantomData<(ST, T)>
        }

        // we need to have seperate impls for Sized values and str/[u8] as otherwise
        // we need seperate impls for `Sized` and `str`/`[u8]` here as
        // we cannot use `Any::downcast_ref` otherwise (which implies `Sized`)
        impl<ST, T, DB> PushBoundValueToCollectorDB<DB> for PushBoundValueToCollectorImpl<ST, T>
        where DB: diesel::backend::Backend
                  + diesel::sql_types::HasSqlType<ST>,
              T: diesel::serialize::ToSql<ST, DB> + 'static,
        {
            fn push_bound_value<'a: 'b, 'b>(
                &self,
                v: InnerBindValueKind<'a>,
                collector: &mut diesel::backend::BindCollector<'b, DB>,
                lookup: &mut <DB as diesel::sql_types::TypeMetadata>::MetadataLookup,
            ) -> diesel::result::QueryResult<()> {
                use diesel::query_builder::BindCollector;
                if let InnerBindValueKind::Sized(v) = v {
                    let v = v.downcast_ref::<T>().expect("We know the type statically here");
                    collector.push_bound_value::<ST, T>(v, lookup)
                } else {
                    unreachable!("We set the value to `InnerBindValueKind::Sized`")
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
                collector: &mut diesel::backend::BindCollector<'b, DB>,
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
                collector: &mut diesel::backend::BindCollector<'b, DB>,
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
            ST: Send + 'static,
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

                    bind.to_sql(&mut out).unwrap();
                    out.into_inner()
                };
                match self {
                    #(#push_to_inner_collector)*
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
                bytes: diesel::backend::RawValue<'_, super::MultiBackend>,
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
            #query_fragment for diesel::query_builder::BoxedLimitOffsetClause<'_, super::backend::MultiBackend>
        },
        quote::quote!{
            <L, O> #query_fragment for diesel::query_builder::LimitOffsetClause<L, O>
        },
        quote::quote! {
            <F, S, D, W, O, LOf, G, H, LC> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiSelectStatementSyntax>
                for diesel::query_builder::SelectStatement<F, S, D, W, O, LOf, G, H, LC>
        },
        quote::quote! {
            <'a, ST, QS, GB> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiSelectStatementSyntax>
                for diesel::query_builder::BoxedSelectStatement<'a, ST, QS, super::backend::MultiBackend, GB>
        },
        quote::quote! {
            <L, R> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiConcatClauseSyntax>
                for diesel::expression::Concat<L, R>
        },
        quote::quote! {
            <T, U> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiArrayComparisonSyntax>
                for diesel::expression::array_comparison::In<T, U>
        },
        quote::quote! {
            <T, U> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiArrayComparisonSyntax>
                for diesel::expression::array_comparison::NotIn<T, U>
        },
        quote::quote! {
            <ST, I> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiArrayComparisonSyntax>
                for diesel::expression::array_comparison::Many<ST, I>
        },
        quote::quote! {
            <T> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiExistsSyntax>
                for diesel::expression::exists::Exists<T>
        },
        quote::quote! {
            diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiEmptyFromClauseSyntax>
                for diesel::query_builder::NoFromClause
        },
        quote::quote! {
            diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiDefaultValueClauseForInsert>
                for diesel::query_builder::DefaultValues
        },
        quote::quote! {
            <Expr> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiReturningClause>
                for diesel::query_builder::ReturningClause<Expr>
        },
        quote::quote! {
            <Expr> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiInsertWithDefaultKeyword>
                for diesel::insertable::DefaultableColumnInsertValue<Expr>
        },
        quote::quote! {
            <Tab, V, QId, const HAS_STATIC_QUERY_ID: bool> diesel::query_builder::QueryFragment<super::backend::MultiBackend, super::backend::MultiBatchInsertSupport>
                for diesel::query_builder::BatchInsert<V, Tab, QId, HAS_STATIC_QUERY_ID>
        }
    ])
    .map(|t| generate_queryfragment_impls(t, &query_fragment_bounds));

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
        quote::quote!{
            #ident(<<#ty as diesel::Connection>::Backend as diesel::backend::HasRawValue<'a>>::RawValue)
        }
    });

    let type_metadata_variants = connection_types.iter().map(|c| {
        let ident = c.name;
        let ty = c.ty;
        quote::quote!{
            #ident(<<#ty as diesel::Connection>::Backend as diesel::sql_types::TypeMetadata>::TypeMetadata)
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
    ]
    .into_iter()
    .map(generate_has_sql_type_impls);

    let into_variant_functions = connection_types.iter().map(|c| {
        let ty = c.ty;
        let ident = c.name;
        let lower_ident = syn::Ident::new(&ident.to_string().to_lowercase(), ident.span());
        quote::quote! {
            fn #lower_ident(&self) -> &<#ty as diesel::Connection>::Backend {
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
                        |l| <#ty as diesel::internal::derives::multiconnection::MultiConnectionHelper>::from_any(l).unwrap(),
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
                return MultiTypeMetadata::#name(<<#ty as diesel::Connection>::Backend as diesel::sql_types::HasSqlType<ST>>::metadata(lookup));
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
                #(#lookup_impl)*
                unreachable!()
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

        impl<'a> diesel::backend::HasRawValue<'a> for MultiBackend {
            type RawValue = MultiRawValue<'a>;
        }


        impl diesel::backend::Backend for MultiBackend {
            type QueryBuilder = super::query_builder::MultiQueryBuilder;
        }

        impl<'a> diesel::backend::HasBindCollector<'a> for MultiBackend {
            type BindCollector = super::bind_collector::MultiBindCollector<'a>;
        }

        pub enum MultiTypeMetadata {
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

        impl diesel::backend::SqlDialect for MultiBackend {
            type ReturningClause = MultiReturningClause;
            // no on conflict support is also the default
            type OnConflictClause = diesel::backend::sql_dialect::on_conflict_clause::DoesNotSupportOnConflictClause;
            type InsertWithDefaultKeyword = MultiInsertWithDefaultKeyword;
            type BatchInsertSupport = MultiBatchInsertSupport;
            type DefaultValueClauseForInsert = MultiDefaultValueClauseForInsert;
            type EmptyFromClauseSyntax = MultiEmptyFromClauseSyntax;
            type ExistsSyntax = MultiExistsSyntax;
            type ArrayComparison = MultiArrayComparisonSyntax;
            type ConcatClause = MultiConcatClauseSyntax;
            type SelectStatementSyntax = MultiSelectStatementSyntax;
        }

        impl diesel::backend::TrustedBackend for MultiBackend {}
        impl diesel::backend::DieselReserveSpecialization for MultiBackend {}

        #(#has_sql_type_impls)*
    }
}
