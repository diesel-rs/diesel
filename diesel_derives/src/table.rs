use diesel_table_macro_syntax::{ColumnDef, TableDecl};
use proc_macro2::TokenStream;
use syn::parse_quote;
use syn::Ident;

const DEFAULT_PRIMARY_KEY_NAME: &str = "id";

pub(crate) fn expand(input: TableDecl) -> TokenStream {
    if input.column_defs.len() > super::diesel_for_each_tuple::MAX_TUPLE_SIZE as usize {
        let txt = if input.column_defs.len() > 128 {
            "You reached the end. Diesel does not support tables with \
             more than 128 columns. Consider using less columns."
        } else if input.column_defs.len() > 64 {
            "Table contains more than 64 columns. Consider enabling the \
             `128-column-tables` feature to enable diesels support for \
             tables with more than 64 columns."
        } else if input.column_defs.len() > 32 {
            "Table contains more than 32 columns. Consider enabling the \
             `64-column-tables` feature to enable diesels support for \
             tables with more than 32 columns."
        } else {
            "Table contains more than 16 columns. Consider enabling the \
             `32-column-tables` feature to enable diesels support for \
             tables with more than 16 columns."
        };
        return quote::quote! {
            compile_error!(#txt);
        };
    }

    let meta = &input.meta;
    let table_name = &input.table_name;
    let imports = if input.use_statements.is_empty() {
        vec![parse_quote!(
            use diesel::sql_types::*;
        )]
    } else {
        input.use_statements.clone()
    };
    let column_names = input
        .column_defs
        .iter()
        .map(|c| &c.column_name)
        .collect::<Vec<_>>();
    let column_names = &column_names;
    let primary_key: TokenStream = match input.primary_keys.as_ref() {
        None if column_names.contains(&&syn::Ident::new(
            DEFAULT_PRIMARY_KEY_NAME,
            proc_macro2::Span::call_site(),
        )) =>
        {
            let id = syn::Ident::new(DEFAULT_PRIMARY_KEY_NAME, proc_macro2::Span::call_site());
            parse_quote! {
                #id
            }
        }
        None => {
            let mut message = format!(
                "Neither an explicit primary key found nor does an `id` column exist.\n\
                 Consider explicitly defining a primary key. \n\
                 For example for specifying `{}` as primary key:\n\n\
                 table! {{\n",
                column_names[0],
            );
            message += &format!("\t{table_name} ({}) {{\n", &column_names[0]);
            for c in &input.column_defs {
                let tpe = c
                    .tpe
                    .path
                    .segments
                    .iter()
                    .map(|p| p.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                message += &format!("\t\t{} -> {tpe},\n", c.column_name);
            }
            message += "\t}\n}";

            let span = input.table_name.span();
            return quote::quote_spanned! {span=>
                compile_error!(#message);
            };
        }
        Some(a) if a.keys.len() == 1 => {
            let k = a.keys.first().unwrap();
            parse_quote! {
                #k
            }
        }
        Some(a) => {
            let keys = a.keys.iter();

            parse_quote! {
                (#(#keys,)*)
            }
        }
    };

    let column_defs = input.column_defs.iter().map(expand_column_def);
    let column_ty = input.column_defs.iter().map(|c| &c.tpe);
    let valid_grouping_for_table_columns = generate_valid_grouping_for_table_columns(&input);

    let sql_name = &input.sql_name;
    let static_query_fragment_impl_for_table = if let Some(schema) = input.schema {
        let schema_name = schema.to_string();
        quote::quote! {
            impl diesel::internal::table_macro::StaticQueryFragment for table {
                type Component = diesel::internal::table_macro::InfixNode<
                        diesel::internal::table_macro::Identifier<'static>,
                    diesel::internal::table_macro::Identifier<'static>,
                    &'static str
                        >;
                const STATIC_COMPONENT: &'static Self::Component = &diesel::internal::table_macro::InfixNode::new(
                    diesel::internal::table_macro::Identifier(#schema_name),
                    diesel::internal::table_macro::Identifier(#sql_name),
                    "."
                );
            }
        }
    } else {
        quote::quote! {
            impl diesel::internal::table_macro::StaticQueryFragment for table {
                type Component = diesel::internal::table_macro::Identifier<'static>;
                const STATIC_COMPONENT: &'static Self::Component = &diesel::internal::table_macro::Identifier(#sql_name);
            }
        }
    };

    let reexport_column_from_dsl = input.column_defs.iter().map(|c| {
        let column_name = &c.column_name;
        if c.column_name == *table_name {
            let span = c.column_name.span();
            let message = format!(
                "Column `{column_name}` cannot be named the same as it's table.\n\
                 You may use `#[sql_name = \"{column_name}\"]` to reference the table's \
                 `{column_name}` column \n\
                 Docs available at: `https://docs.diesel.rs/master/diesel/macro.table.html`\n"
            );
            quote::quote_spanned! { span =>
                compile_error!(#message);
            }
        } else {
            quote::quote! {
                pub use super::columns::#column_name;
            }
        }
    });

    let backend_specific_table_impls = if cfg!(feature = "postgres") {
        Some(quote::quote! {
            impl<S> diesel::JoinTo<diesel::query_builder::Only<S>> for table
            where
                diesel::query_builder::Only<S>: diesel::JoinTo<table>,
            {
                type FromClause = diesel::query_builder::Only<S>;
                type OnClause = <diesel::query_builder::Only<S> as diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::query_builder::Only<S>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::query_builder::Only::<S>::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl diesel::query_source::AppearsInFromClause<diesel::query_builder::Only<table>>
                for table
            {
                type Count = diesel::query_source::Once;
            }

            impl diesel::query_source::AppearsInFromClause<table>
                for diesel::query_builder::Only<table>
            {
                type Count = diesel::query_source::Once;
            }

            impl<S, TSM> diesel::JoinTo<diesel::query_builder::Tablesample<S, TSM>> for table
            where
                diesel::query_builder::Tablesample<S, TSM>: diesel::JoinTo<table>,
                TSM: diesel::internal::table_macro::TablesampleMethod
            {
                type FromClause = diesel::query_builder::Tablesample<S, TSM>;
                type OnClause = <diesel::query_builder::Tablesample<S, TSM> as diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::query_builder::Tablesample<S, TSM>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::query_builder::Tablesample::<S, TSM>::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<TSM> diesel::query_source::AppearsInFromClause<diesel::query_builder::Tablesample<table, TSM>>
                for table
                    where
                TSM: diesel::internal::table_macro::TablesampleMethod
            {
                type Count = diesel::query_source::Once;
            }

            impl<TSM> diesel::query_source::AppearsInFromClause<table>
                for diesel::query_builder::Tablesample<table, TSM>
                    where
                TSM: diesel::internal::table_macro::TablesampleMethod
            {
                type Count = diesel::query_source::Once;
            }
        })
    } else {
        None
    };

    let imports_for_column_module = imports.iter().map(fix_import_for_submodule);

    quote::quote! {
        #(#meta)*
        #[allow(unused_imports, dead_code, unreachable_pub)]
        pub mod #table_name {
            use ::diesel;
            pub use self::columns::*;
            #(#imports)*

            /// Re-exports all of the columns of this table, as well as the
            /// table struct renamed to the module name. This is meant to be
            /// glob imported for functions which only deal with one table.
            pub mod dsl {
                #(#reexport_column_from_dsl)*
                pub use super::table as #table_name;
            }

            #[allow(non_upper_case_globals, dead_code)]
            /// A tuple of all of the columns on this table
            pub const all_columns: (#(#column_names,)*) = (#(#column_names,)*);

            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId, Default)]
            /// The actual table struct
            ///
            /// This is the type which provides the base methods of the query
            /// builder, such as `.select` and `.filter`.
            pub struct table;

            impl table {
                #[allow(dead_code)]
                /// Represents `table_name.*`, which is sometimes necessary
                /// for efficient count queries. It cannot be used in place of
                /// `all_columns`
                pub fn star(&self) -> star {
                    star
                }
            }

            /// The SQL type of all of the columns on this table
            pub type SqlType = (#(#column_ty,)*);

            /// Helper type for representing a boxed query from this table
            pub type BoxedQuery<'a, DB, ST = SqlType> = diesel::internal::table_macro::BoxedSelectStatement<'a, ST, diesel::internal::table_macro::FromClause<table>, DB>;

            impl diesel::QuerySource for table {
                type FromClause = diesel::internal::table_macro::StaticQueryFragmentInstance<table>;
                type DefaultSelection = <Self as diesel::Table>::AllColumns;

                fn from_clause(&self) -> Self::FromClause {
                    diesel::internal::table_macro::StaticQueryFragmentInstance::new()
                }

                fn default_selection(&self) -> Self::DefaultSelection {
                    use diesel::Table;
                    Self::all_columns()
                }
            }

            impl<DB> diesel::query_builder::QueryFragment<DB> for table where
                DB: diesel::backend::Backend,
                <table as diesel::internal::table_macro::StaticQueryFragment>::Component: diesel::query_builder::QueryFragment<DB>
            {
                fn walk_ast<'b>(&'b self, __diesel_internal_pass: diesel::query_builder::AstPass<'_, 'b, DB>) -> diesel::result::QueryResult<()> {
                    <table as diesel::internal::table_macro::StaticQueryFragment>::STATIC_COMPONENT.walk_ast(__diesel_internal_pass)
                }
            }

            #static_query_fragment_impl_for_table

            impl diesel::query_builder::AsQuery for table {
                type SqlType = SqlType;
                type Query = diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<Self>>;

                fn as_query(self) -> Self::Query {
                    diesel::internal::table_macro::SelectStatement::simple(self)
                }
            }

            impl diesel::Table for table {
                type PrimaryKey = #primary_key;
                type AllColumns = (#(#column_names,)*);

                fn primary_key(&self) -> Self::PrimaryKey {
                    #primary_key
                }

                fn all_columns() -> Self::AllColumns {
                    (#(#column_names,)*)
                }
            }

            impl diesel::associations::HasTable for table {
                type Table = Self;

                fn table() -> Self::Table {
                    table
                }
            }

            impl diesel::query_builder::IntoUpdateTarget for table {
                type WhereClause = <<Self as diesel::query_builder::AsQuery>::Query as diesel::query_builder::IntoUpdateTarget>::WhereClause;

                fn into_update_target(self) -> diesel::query_builder::UpdateTarget<Self::Table, Self::WhereClause> {
                    use diesel::query_builder::AsQuery;
                    let q: diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<table>> = self.as_query();
                    q.into_update_target()
                }
            }

            impl diesel::query_source::AppearsInFromClause<table> for table {
                type Count = diesel::query_source::Once;
            }

            // impl<S: AliasSource<Table=table>> AppearsInFromClause<table> for Alias<S>
            impl<S> diesel::internal::table_macro::AliasAppearsInFromClause<S, table> for table
            where S: diesel::query_source::AliasSource<Target=table>,
            {
                type Count = diesel::query_source::Never;
            }

            // impl<S1: AliasSource<Table=table>, S2: AliasSource<Table=table>> AppearsInFromClause<Alias<S1>> for Alias<S2>
            // Those are specified by the `alias!` macro, but this impl will allow it to implement this trait even in downstream
            // crates from the schema
            impl<S1, S2> diesel::internal::table_macro::AliasAliasAppearsInFromClause<table, S2, S1> for table
            where S1: diesel::query_source::AliasSource<Target=table>,
                  S2: diesel::query_source::AliasSource<Target=table>,
                  S1: diesel::internal::table_macro::AliasAliasAppearsInFromClauseSameTable<S2, table>,
            {
                type Count = <S1 as diesel::internal::table_macro::AliasAliasAppearsInFromClauseSameTable<S2, table>>::Count;
            }

            impl<S> diesel::query_source::AppearsInFromClause<diesel::query_source::Alias<S>> for table
            where S: diesel::query_source::AliasSource,
            {
                type Count = diesel::query_source::Never;
            }

            impl<S, C> diesel::internal::table_macro::FieldAliasMapperAssociatedTypesDisjointnessTrick<table, S, C> for table
            where
                S: diesel::query_source::AliasSource<Target = table> + ::std::clone::Clone,
                C: diesel::query_source::Column<Table = table>,
            {
                type Out = diesel::query_source::AliasedField<S, C>;

                fn map(__diesel_internal_column: C, __diesel_internal_alias: &diesel::query_source::Alias<S>) -> Self::Out {
                    __diesel_internal_alias.field(__diesel_internal_column)
                }
            }

            impl diesel::query_source::AppearsInFromClause<table> for diesel::internal::table_macro::NoFromClause {
                type Count = diesel::query_source::Never;
            }

            impl<Left, Right, Kind> diesel::JoinTo<diesel::internal::table_macro::Join<Left, Right, Kind>> for table where
                diesel::internal::table_macro::Join<Left, Right, Kind>: diesel::JoinTo<table>,
                Left: diesel::query_source::QuerySource,
                Right: diesel::query_source::QuerySource,
            {
                type FromClause = diesel::internal::table_macro::Join<Left, Right, Kind>;
                type OnClause = <diesel::internal::table_macro::Join<Left, Right, Kind> as diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::Join<Left, Right, Kind>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::Join::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<Join, On> diesel::JoinTo<diesel::internal::table_macro::JoinOn<Join, On>> for table where
                diesel::internal::table_macro::JoinOn<Join, On>: diesel::JoinTo<table>,
            {
                type FromClause = diesel::internal::table_macro::JoinOn<Join, On>;
                type OnClause = <diesel::internal::table_macro::JoinOn<Join, On> as diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::JoinOn<Join, On>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::JoinOn::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<F, S, D, W, O, L, Of, G> diesel::JoinTo<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>> for table where
                diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>: diesel::JoinTo<table>,
                F: diesel::query_source::QuerySource
            {
                type FromClause = diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>;
                type OnClause = <diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G> as diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::SelectStatement::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<'a, QS, ST, DB> diesel::JoinTo<diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>> for table where
                diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>: diesel::JoinTo<table>,
                QS: diesel::query_source::QuerySource,
            {
                type FromClause = diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>;
                type OnClause = <diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB> as diesel::JoinTo<table>>::OnClause;
                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::BoxedSelectStatement::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<S> diesel::JoinTo<diesel::query_source::Alias<S>> for table
            where
                diesel::query_source::Alias<S>: diesel::JoinTo<table>,
            {
                type FromClause = diesel::query_source::Alias<S>;
                type OnClause = <diesel::query_source::Alias<S> as diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::query_source::Alias<S>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::query_source::Alias::<S>::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            // This impl should be able to live in Diesel,
            // but Rust tries to recurse for no reason
            impl<T> diesel::insertable::Insertable<T> for table
            where
                <table as diesel::query_builder::AsQuery>::Query: diesel::insertable::Insertable<T>,
            {
                type Values = <<table as diesel::query_builder::AsQuery>::Query as diesel::insertable::Insertable<T>>::Values;

                fn values(self) -> Self::Values {
                    use diesel::query_builder::AsQuery;
                    self.as_query().values()
                }
            }

            impl<'a, T> diesel::insertable::Insertable<T> for &'a table
            where
                table: diesel::insertable::Insertable<T>,
            {
                type Values = <table as diesel::insertable::Insertable<T>>::Values;

                fn values(self) -> Self::Values {
                    (*self).values()
                }
            }

            #backend_specific_table_impls

            /// Contains all of the columns of this table
            pub mod columns {
                use ::diesel;
                use super::table;
                #(#imports_for_column_module)*

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId)]
                /// Represents `table_name.*`, which is sometimes needed for
                /// efficient count queries. It cannot be used in place of
                /// `all_columns`, and has a `SqlType` of `()` to prevent it
                /// being used that way
                pub struct star;

                impl<__GB> diesel::expression::ValidGrouping<__GB> for star
                where
                    (#(#column_names,)*): diesel::expression::ValidGrouping<__GB>,
                {
                    type IsAggregate = <(#(#column_names,)*) as diesel::expression::ValidGrouping<__GB>>::IsAggregate;
                }

                impl diesel::Expression for star {
                    type SqlType = diesel::expression::expression_types::NotSelectable;
                }

                impl<DB: diesel::backend::Backend> diesel::query_builder::QueryFragment<DB> for star where
                    <table as diesel::QuerySource>::FromClause: diesel::query_builder::QueryFragment<DB>,
                {
                    #[allow(non_snake_case)]
                    fn walk_ast<'b>(&'b self, mut __diesel_internal_out: diesel::query_builder::AstPass<'_, 'b, DB>) -> diesel::result::QueryResult<()>
                    {
                        use diesel::QuerySource;

                        if !__diesel_internal_out.should_skip_from() {
                            const FROM_CLAUSE: diesel::internal::table_macro::StaticQueryFragmentInstance<table> = diesel::internal::table_macro::StaticQueryFragmentInstance::new();

                            FROM_CLAUSE.walk_ast(__diesel_internal_out.reborrow())?;
                            __diesel_internal_out.push_sql(".");
                        }
                        __diesel_internal_out.push_sql("*");
                        Ok(())
                    }
                }

                impl diesel::SelectableExpression<table> for star {
                }

                impl diesel::AppearsOnTable<table> for star {
                }

                #(#column_defs)*

                #(#valid_grouping_for_table_columns)*
            }
        }
    }
}

fn generate_valid_grouping_for_table_columns(table: &TableDecl) -> Vec<TokenStream> {
    let mut ret = Vec::with_capacity(table.column_defs.len() * table.column_defs.len());

    let primary_key = if let Some(ref pk) = table.primary_keys {
        if pk.keys.len() == 1 {
            pk.keys.first().map(ToString::to_string)
        } else {
            None
        }
    } else {
        Some(DEFAULT_PRIMARY_KEY_NAME.into())
    };

    for (id, right_col) in table.column_defs.iter().enumerate() {
        for left_col in table.column_defs.iter().skip(id) {
            let right_to_left = if Some(left_col.column_name.to_string()) == primary_key {
                Ident::new("Yes", proc_macro2::Span::call_site())
            } else {
                Ident::new("No", proc_macro2::Span::call_site())
            };

            let left_to_right = if Some(right_col.column_name.to_string()) == primary_key {
                Ident::new("Yes", proc_macro2::Span::call_site())
            } else {
                Ident::new("No", proc_macro2::Span::call_site())
            };

            let left_col = &left_col.column_name;
            let right_col = &right_col.column_name;

            if left_col != right_col {
                ret.push(quote::quote! {
                    impl diesel::expression::IsContainedInGroupBy<#right_col> for #left_col {
                        type Output = diesel::expression::is_contained_in_group_by::#right_to_left;
                    }

                    impl diesel::expression::IsContainedInGroupBy<#left_col> for #right_col {
                        type Output = diesel::expression::is_contained_in_group_by::#left_to_right;
                    }
                });
            }
        }
    }
    ret
}

fn fix_import_for_submodule(import: &syn::ItemUse) -> syn::ItemUse {
    let mut ret = import.clone();

    if let syn::UseTree::Path(ref mut path) = ret.tree {
        // prepend another `super` to the any import
        // that starts with `super` so that it now refers to the correct
        // module
        if path.ident == "super" {
            let inner = path.clone();
            path.tree = Box::new(syn::UseTree::Path(inner));
        }
    }

    ret
}

fn is_numeric(ty: &syn::TypePath) -> bool {
    const NUMERIC_TYPES: &[&str] = &[
        "SmallInt",
        "Int2",
        "Smallint",
        "SmallSerial",
        "Integer",
        "Int4",
        "Serial",
        "BigInt",
        "Int8",
        "Bigint",
        "BigSerial",
        "Float",
        "Float4",
        "Float8",
        "Double",
        "Numeric",
    ];

    if let Some(last) = ty.path.segments.last() {
        match &last.arguments {
            syn::PathArguments::AngleBracketed(t)
                if (last.ident == "Nullable" || last.ident == "Unsigned") && t.args.len() == 1 =>
            {
                if let Some(syn::GenericArgument::Type(syn::Type::Path(t))) = t.args.first() {
                    NUMERIC_TYPES.iter().any(|i| {
                        t.path.segments.last().map(|s| s.ident.to_string())
                            == Some(String::from(*i))
                    })
                } else {
                    false
                }
            }
            _ => NUMERIC_TYPES.iter().any(|i| last.ident == *i),
        }
    } else {
        false
    }
}

fn is_date_time(ty: &syn::TypePath) -> bool {
    const DATE_TYPES: &[&str] = &["Time", "Date", "Timestamp", "Timestamptz"];
    if let Some(last) = ty.path.segments.last() {
        match &last.arguments {
            syn::PathArguments::AngleBracketed(t)
                if last.ident == "Nullable" && t.args.len() == 1 =>
            {
                if let Some(syn::GenericArgument::Type(syn::Type::Path(t))) = t.args.first() {
                    DATE_TYPES.iter().any(|i| {
                        t.path.segments.last().map(|s| s.ident.to_string())
                            == Some(String::from(*i))
                    })
                } else {
                    false
                }
            }
            _ => DATE_TYPES.iter().any(|i| last.ident == *i),
        }
    } else {
        false
    }
}

fn is_network(ty: &syn::TypePath) -> bool {
    const NETWORK_TYPES: &[&str] = &["Cidr", "Inet"];

    if let Some(last) = ty.path.segments.last() {
        match &last.arguments {
            syn::PathArguments::AngleBracketed(t)
                if last.ident == "Nullable" && t.args.len() == 1 =>
            {
                if let Some(syn::GenericArgument::Type(syn::Type::Path(t))) = t.args.first() {
                    NETWORK_TYPES.iter().any(|i| {
                        t.path.segments.last().map(|s| s.ident.to_string())
                            == Some(String::from(*i))
                    })
                } else {
                    false
                }
            }
            _ => NETWORK_TYPES.iter().any(|i| last.ident == *i),
        }
    } else {
        false
    }
}

fn generate_op_impl(op: &str, tpe: &syn::Ident) -> TokenStream {
    let fn_name = syn::Ident::new(&op.to_lowercase(), tpe.span());
    let op = syn::Ident::new(op, tpe.span());
    quote::quote! {
        impl<Rhs> ::std::ops::#op<Rhs> for #tpe
        where
            Rhs: diesel::expression::AsExpression<
                <<#tpe as diesel::Expression>::SqlType as diesel::sql_types::ops::#op>::Rhs,
            >,
        {
            type Output = diesel::internal::table_macro::ops::#op<Self, Rhs::Expression>;

            fn #fn_name(self, __diesel_internal_rhs: Rhs) -> Self::Output {
                diesel::internal::table_macro::ops::#op::new(self, __diesel_internal_rhs.as_expression())
            }
        }
    }
}

fn expand_column_def(column_def: &ColumnDef) -> TokenStream {
    // TODO get a better span here as soon as that's
    // possible using stable rust
    let span = column_def.column_name.span();
    let meta = &column_def.meta;
    let column_name = &column_def.column_name;
    let sql_name = &column_def.sql_name;
    let sql_type = &column_def.tpe;

    let backend_specific_column_impl = if cfg!(feature = "postgres") {
        Some(quote::quote! {
            impl diesel::query_source::AppearsInFromClause<diesel::query_builder::Only<super::table>>
                for #column_name
            {
                type Count = diesel::query_source::Once;
            }
            impl diesel::SelectableExpression<diesel::query_builder::Only<super::table>> for #column_name {}

            impl<TSM> diesel::query_source::AppearsInFromClause<diesel::query_builder::Tablesample<super::table, TSM>>
                for #column_name
                    where
                TSM: diesel::internal::table_macro::TablesampleMethod
            {
                type Count = diesel::query_source::Once;
            }
            impl<TSM> diesel::SelectableExpression<diesel::query_builder::Tablesample<super::table, TSM>>
                for #column_name where TSM: diesel::internal::table_macro::TablesampleMethod {}
        })
    } else {
        None
    };

    let ops_impls = if is_numeric(&column_def.tpe) {
        let add = generate_op_impl("Add", column_name);
        let sub = generate_op_impl("Sub", column_name);
        let div = generate_op_impl("Div", column_name);
        let mul = generate_op_impl("Mul", column_name);
        Some(quote::quote! {
            #add
            #sub
            #div
            #mul
        })
    } else if is_date_time(&column_def.tpe) || is_network(&column_def.tpe) {
        let add = generate_op_impl("Add", column_name);
        let sub = generate_op_impl("Sub", column_name);
        Some(quote::quote! {
            #add
            #sub
        })
    } else {
        None
    };

    let max_length = column_def.max_length.as_ref().map(|column_max_length| {
        quote::quote! {
            impl self::diesel::query_source::SizeRestrictedColumn for #column_name {
                const MAX_LENGTH: usize = #column_max_length;
            }
        }
    });

    quote::quote_spanned! {span=>
        #(#meta)*
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId, Default)]
        pub struct #column_name;

        impl diesel::expression::Expression for #column_name {
            type SqlType = #sql_type;
        }

        impl<DB> diesel::query_builder::QueryFragment<DB> for #column_name where
            DB: diesel::backend::Backend,
            diesel::internal::table_macro::StaticQueryFragmentInstance<table>: diesel::query_builder::QueryFragment<DB>,
        {
            #[allow(non_snake_case)]
            fn walk_ast<'b>(&'b self, mut __diesel_internal_out: diesel::query_builder::AstPass<'_, 'b, DB>) -> diesel::result::QueryResult<()>
            {
                if !__diesel_internal_out.should_skip_from() {
                    const FROM_CLAUSE: diesel::internal::table_macro::StaticQueryFragmentInstance<table> = diesel::internal::table_macro::StaticQueryFragmentInstance::new();

                    FROM_CLAUSE.walk_ast(__diesel_internal_out.reborrow())?;
                    __diesel_internal_out.push_sql(".");
                }
                __diesel_internal_out.push_identifier(#sql_name)
            }
        }

        impl diesel::SelectableExpression<super::table> for #column_name {
        }

        impl<QS> diesel::AppearsOnTable<QS> for #column_name where
            QS: diesel::query_source::AppearsInFromClause<super::table, Count=diesel::query_source::Once>,
        {
        }

        impl<Left, Right> diesel::SelectableExpression<
                diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::LeftOuter>,
            > for #column_name where
            #column_name: diesel::AppearsOnTable<diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::LeftOuter>>,
            Self: diesel::SelectableExpression<Left>,
            // If our table is on the right side of this join, only
            // `Nullable<Self>` can be selected
            Right: diesel::query_source::AppearsInFromClause<super::table, Count=diesel::query_source::Never> + diesel::query_source::QuerySource,
            Left: diesel::query_source::QuerySource
        {
        }

        impl<Left, Right> diesel::SelectableExpression<
                diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::Inner>,
            > for #column_name where
            #column_name: diesel::AppearsOnTable<diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::Inner>>,
            Left: diesel::query_source::AppearsInFromClause<super::table> + diesel::query_source::QuerySource,
            Right: diesel::query_source::AppearsInFromClause<super::table> + diesel::query_source::QuerySource,
        (Left::Count, Right::Count): diesel::internal::table_macro::Pick<Left, Right>,
            Self: diesel::SelectableExpression<
                <(Left::Count, Right::Count) as diesel::internal::table_macro::Pick<Left, Right>>::Selection,
            >,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        impl<Join, On> diesel::SelectableExpression<diesel::internal::table_macro::JoinOn<Join, On>> for #column_name where
            #column_name: diesel::SelectableExpression<Join> + diesel::AppearsOnTable<diesel::internal::table_macro::JoinOn<Join, On>>,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        impl<From> diesel::SelectableExpression<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<From>>> for #column_name where
            From: diesel::query_source::QuerySource,
            #column_name: diesel::SelectableExpression<From> + diesel::AppearsOnTable<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<From>>>,
        {
        }

        impl<__GB> diesel::expression::ValidGrouping<__GB> for #column_name
        where __GB: diesel::expression::IsContainedInGroupBy<#column_name, Output = diesel::expression::is_contained_in_group_by::Yes>,
        {
            type IsAggregate = diesel::expression::is_aggregate::Yes;
        }

        impl diesel::expression::ValidGrouping<()> for #column_name {
            type IsAggregate = diesel::expression::is_aggregate::No;
        }

        impl diesel::expression::IsContainedInGroupBy<#column_name> for #column_name {
            type Output = diesel::expression::is_contained_in_group_by::Yes;
        }

        impl diesel::query_source::Column for #column_name {
            type Table = super::table;

            const NAME: &'static str = #sql_name;
        }

        impl<T> diesel::EqAll<T> for #column_name where
            T: diesel::expression::AsExpression<#sql_type>,
            diesel::dsl::Eq<#column_name, T::Expression>: diesel::Expression<SqlType=diesel::sql_types::Bool>,
        {
            type Output = diesel::dsl::Eq<Self, T::Expression>;

            fn eq_all(self, __diesel_internal_rhs: T) -> Self::Output {
                use diesel::expression_methods::ExpressionMethods;
                self.eq(__diesel_internal_rhs)
            }
        }

        #max_length

        #ops_impls
        #backend_specific_column_impl
    }
}
