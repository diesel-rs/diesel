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
             `32-column-tables` feature to enable diesels support for \
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
        None => {
            let id = syn::Ident::new(DEFAULT_PRIMARY_KEY_NAME, proc_macro2::Span::call_site());
            parse_quote! {
                #id
            }
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
            #[derive(Debug, Clone, Copy, self::diesel::query_builder::QueryId, Default)]
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
            pub type BoxedQuery<'a, DB, ST = SqlType> = self::diesel::internal::table_macro::BoxedSelectStatement<'a, ST, self::diesel::internal::table_macro::FromClause<table>, DB>;

            impl self::diesel::QuerySource for table {
                type FromClause = self::diesel::internal::table_macro::StaticQueryFragmentInstance<table>;
                type DefaultSelection = <Self as self::diesel::Table>::AllColumns;

                fn from_clause(&self) -> Self::FromClause {
                    self::diesel::internal::table_macro::StaticQueryFragmentInstance::new()
                }

                fn default_selection(&self) -> Self::DefaultSelection {
                    use self::diesel::Table;
                    Self::all_columns()
                }
            }

            impl<DB> self::diesel::query_builder::QueryFragment<DB> for table where
                DB: self::diesel::backend::Backend,
                <table as self::diesel::internal::table_macro::StaticQueryFragment>::Component: self::diesel::query_builder::QueryFragment<DB>
            {
                fn walk_ast<'b>(&'b self, __diesel_internal_pass: self::diesel::query_builder::AstPass<'_, 'b, DB>) -> self::diesel::result::QueryResult<()> {
                    <table as self::diesel::internal::table_macro::StaticQueryFragment>::STATIC_COMPONENT.walk_ast(__diesel_internal_pass)
                }
            }

            #static_query_fragment_impl_for_table

            impl self::diesel::query_builder::AsQuery for table {
                type SqlType = SqlType;
                type Query = self::diesel::internal::table_macro::SelectStatement<self::diesel::internal::table_macro::FromClause<Self>>;

                fn as_query(self) -> Self::Query {
                    self::diesel::internal::table_macro::SelectStatement::simple(self)
                }
            }

            impl self::diesel::Table for table {
                type PrimaryKey = #primary_key;
                type AllColumns = (#(#column_names,)*);

                fn primary_key(&self) -> Self::PrimaryKey {
                    #primary_key
                }

                fn all_columns() -> Self::AllColumns {
                    (#(#column_names,)*)
                }
            }

            impl self::diesel::associations::HasTable for table {
                type Table = Self;

                fn table() -> Self::Table {
                    table
                }
            }

            impl self::diesel::query_builder::IntoUpdateTarget for table {
                type WhereClause = <<Self as self::diesel::query_builder::AsQuery>::Query as self::diesel::query_builder::IntoUpdateTarget>::WhereClause;

                fn into_update_target(self) -> self::diesel::query_builder::UpdateTarget<Self::Table, Self::WhereClause> {
                    use self::diesel::query_builder::AsQuery;
                    let q: self::diesel::internal::table_macro::SelectStatement<self::diesel::internal::table_macro::FromClause<table>> = self.as_query();
                    q.into_update_target()
                }
            }

            impl self::diesel::query_source::AppearsInFromClause<table> for table {
                type Count = self::diesel::query_source::Once;
            }

            // impl<S: AliasSource<Table=table>> AppearsInFromClause<table> for Alias<S>
            impl<S> self::diesel::internal::table_macro::AliasAppearsInFromClause<S, table> for table
            where S: self::diesel::query_source::AliasSource<Target=table>,
            {
                type Count = self::diesel::query_source::Never;
            }

            // impl<S1: AliasSource<Table=table>, S2: AliasSource<Table=table>> AppearsInFromClause<Alias<S1>> for Alias<S2>
            // Those are specified by the `alias!` macro, but this impl will allow it to implement this trait even in downstream
            // crates from the schema
            impl<S1, S2> self::diesel::internal::table_macro::AliasAliasAppearsInFromClause<table, S2, S1> for table
            where S1: self::diesel::query_source::AliasSource<Target=table>,
                  S2: self::diesel::query_source::AliasSource<Target=table>,
                  S1: self::diesel::internal::table_macro::AliasAliasAppearsInFromClauseSameTable<S2, table>,
            {
                type Count = <S1 as self::diesel::internal::table_macro::AliasAliasAppearsInFromClauseSameTable<S2, table>>::Count;
            }

            impl<S> self::diesel::query_source::AppearsInFromClause<self::diesel::query_source::Alias<S>> for table
            where S: self::diesel::query_source::AliasSource,
            {
                type Count = self::diesel::query_source::Never;
            }

            impl<S, C> self::diesel::internal::table_macro::FieldAliasMapperAssociatedTypesDisjointnessTrick<table, S, C> for table
            where
                S: self::diesel::query_source::AliasSource<Target = table> + ::std::clone::Clone,
                C: self::diesel::query_source::Column<Table = table>,
            {
                type Out = self::diesel::query_source::AliasedField<S, C>;

                fn map(__diesel_internal_column: C, __diesel_internal_alias: &self::diesel::query_source::Alias<S>) -> Self::Out {
                    __diesel_internal_alias.field(__diesel_internal_column)
                }
            }

            impl self::diesel::query_source::AppearsInFromClause<table> for self::diesel::internal::table_macro::NoFromClause {
                type Count = self::diesel::query_source::Never;
            }

            impl<Left, Right, Kind> self::diesel::JoinTo<self::diesel::internal::table_macro::Join<Left, Right, Kind>> for table where
                self::diesel::internal::table_macro::Join<Left, Right, Kind>: self::diesel::JoinTo<table>,
                Left: self::diesel::query_source::QuerySource,
                Right: diesel::query_source::QuerySource,
            {
                type FromClause = self::diesel::internal::table_macro::Join<Left, Right, Kind>;
                type OnClause = <diesel::internal::table_macro::Join<Left, Right, Kind> as self::diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: self::diesel::internal::table_macro::Join<Left, Right, Kind>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = self::diesel::internal::table_macro::Join::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<Join, On> self::diesel::JoinTo<diesel::internal::table_macro::JoinOn<Join, On>> for table where
                self::diesel::internal::table_macro::JoinOn<Join, On>: self::diesel::JoinTo<table>,
            {
                type FromClause = self::diesel::internal::table_macro::JoinOn<Join, On>;
                type OnClause = <diesel::internal::table_macro::JoinOn<Join, On> as self::diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: self::diesel::internal::table_macro::JoinOn<Join, On>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = self::diesel::internal::table_macro::JoinOn::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<F, S, D, W, O, L, Of, G> self::diesel::JoinTo<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>> for table where
                self::diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>: self::diesel::JoinTo<table>,
                F: self::diesel::query_source::QuerySource
            {
                type FromClause = self::diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>;
                type OnClause = <diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G> as self::diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: self::diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = self::diesel::internal::table_macro::SelectStatement::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<'a, QS, ST, DB> self::diesel::JoinTo<diesel::internal::table_macro::BoxedSelectStatement<'a, self::diesel::internal::table_macro::FromClause<QS>, ST, DB>> for table where
                self::diesel::internal::table_macro::BoxedSelectStatement<'a, self::diesel::internal::table_macro::FromClause<QS>, ST, DB>: self::diesel::JoinTo<table>,
                QS: self::diesel::query_source::QuerySource,
            {
                type FromClause = self::diesel::internal::table_macro::BoxedSelectStatement<'a, self::diesel::internal::table_macro::FromClause<QS>, ST, DB>;
                type OnClause = <diesel::internal::table_macro::BoxedSelectStatement<'a, self::diesel::internal::table_macro::FromClause<QS>, ST, DB> as self::diesel::JoinTo<table>>::OnClause;
                fn join_target(__diesel_internal_rhs: self::diesel::internal::table_macro::BoxedSelectStatement<'a, self::diesel::internal::table_macro::FromClause<QS>, ST, DB>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = self::diesel::internal::table_macro::BoxedSelectStatement::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<S> self::diesel::JoinTo<diesel::query_source::Alias<S>> for table
            where
                self::diesel::query_source::Alias<S>: self::diesel::JoinTo<table>,
            {
                type FromClause = self::diesel::query_source::Alias<S>;
                type OnClause = <diesel::query_source::Alias<S> as self::diesel::JoinTo<table>>::OnClause;

                fn join_target(__diesel_internal_rhs: self::diesel::query_source::Alias<S>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = self::diesel::query_source::Alias::<S>::join_target(table);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            // This impl should be able to live in Diesel,
            // but Rust tries to recurse for no reason
            impl<T> self::diesel::insertable::Insertable<T> for table
            where
                <table as self::diesel::query_builder::AsQuery>::Query: self::diesel::insertable::Insertable<T>,
            {
                type Values = <<table as self::diesel::query_builder::AsQuery>::Query as self::diesel::insertable::Insertable<T>>::Values;

                fn values(self) -> Self::Values {
                    use self::diesel::query_builder::AsQuery;
                    self.as_query().values()
                }
            }

            impl<'a, T> self::diesel::insertable::Insertable<T> for &'a table
            where
                table: self::diesel::insertable::Insertable<T>,
            {
                type Values = <table as self::diesel::insertable::Insertable<T>>::Values;

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
                #[derive(Debug, Clone, Copy, self::diesel::query_builder::QueryId)]
                /// Represents `table_name.*`, which is sometimes needed for
                /// efficient count queries. It cannot be used in place of
                /// `all_columns`, and has a `SqlType` of `()` to prevent it
                /// being used that way
                pub struct star;

                impl<__GB> self::diesel::expression::ValidGrouping<__GB> for star
                where
                    (#(#column_names,)*): self::diesel::expression::ValidGrouping<__GB>,
                {
                    type IsAggregate = <(#(#column_names,)*) as self::diesel::expression::ValidGrouping<__GB>>::IsAggregate;
                }

                impl self::diesel::Expression for star {
                    type SqlType = self::diesel::expression::expression_types::NotSelectable;
                }

                impl<DB: self::diesel::backend::Backend> self::diesel::query_builder::QueryFragment<DB> for star where
                    <table as self::diesel::QuerySource>::FromClause: self::diesel::query_builder::QueryFragment<DB>,
                {
                    #[allow(non_snake_case)]
                    fn walk_ast<'b>(&'b self, mut __diesel_internal_out: self::diesel::query_builder::AstPass<'_, 'b, DB>) -> self::diesel::result::QueryResult<()>
                    {
                        use self::diesel::QuerySource;

                        if !__diesel_internal_out.should_skip_from() {
                            const FROM_CLAUSE: self::diesel::internal::table_macro::StaticQueryFragmentInstance<table> = self::diesel::internal::table_macro::StaticQueryFragmentInstance::new();

                            FROM_CLAUSE.walk_ast(__diesel_internal_out.reborrow())?;
                            __diesel_internal_out.push_sql(".");
                        }
                        __diesel_internal_out.push_sql("*");
                        Ok(())
                    }
                }

                impl self::diesel::SelectableExpression<table> for star {
                }

                impl self::diesel::AppearsOnTable<table> for star {
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
                    impl self::diesel::expression::IsContainedInGroupBy<#right_col> for #left_col {
                        type Output = self::diesel::expression::is_contained_in_group_by::#right_to_left;
                    }

                    impl self::diesel::expression::IsContainedInGroupBy<#left_col> for #right_col {
                        type Output = self::diesel::expression::is_contained_in_group_by::#left_to_right;
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
            Rhs: self::diesel::expression::AsExpression<
                <<#tpe as self::diesel::Expression>::SqlType as self::diesel::sql_types::ops::#op>::Rhs,
            >,
        {
            type Output = self::diesel::internal::table_macro::ops::#op<Self, Rhs::Expression>;

            fn #fn_name(self, __diesel_internal_rhs: Rhs) -> Self::Output {
                self::diesel::internal::table_macro::ops::#op::new(self, __diesel_internal_rhs.as_expression())
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
            impl self::diesel::query_source::AppearsInFromClause<diesel::query_builder::Only<super::table>>
                for #column_name
            {
                type Count = self::diesel::query_source::Once;
            }
            impl self::diesel::SelectableExpression<diesel::query_builder::Only<super::table>> for #column_name {}
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

    quote::quote_spanned! {span=>
        #(#meta)*
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy, self::diesel::query_builder::QueryId, Default)]
        pub struct #column_name;

        impl self::diesel::expression::Expression for #column_name {
            type SqlType = #sql_type;
        }

        impl<DB> self::diesel::query_builder::QueryFragment<DB> for #column_name where
            DB: self::diesel::backend::Backend,
            self::diesel::internal::table_macro::StaticQueryFragmentInstance<table>: self::diesel::query_builder::QueryFragment<DB>,
        {
            #[allow(non_snake_case)]
            fn walk_ast<'b>(&'b self, mut __diesel_internal_out: self::diesel::query_builder::AstPass<'_, 'b, DB>) -> self::diesel::result::QueryResult<()>
            {
                if !__diesel_internal_out.should_skip_from() {
                    const FROM_CLAUSE: self::diesel::internal::table_macro::StaticQueryFragmentInstance<table> = self::diesel::internal::table_macro::StaticQueryFragmentInstance::new();

                    FROM_CLAUSE.walk_ast(__diesel_internal_out.reborrow())?;
                    __diesel_internal_out.push_sql(".");
                }
                __diesel_internal_out.push_identifier(#sql_name)
            }
        }

        impl self::diesel::SelectableExpression<super::table> for #column_name {
        }

        impl<QS> self::diesel::AppearsOnTable<QS> for #column_name where
            QS: self::diesel::query_source::AppearsInFromClause<super::table, Count=diesel::query_source::Once>,
        {
        }

        impl<Left, Right> self::diesel::SelectableExpression<
                self::diesel::internal::table_macro::Join<Left, Right, self::diesel::internal::table_macro::LeftOuter>,
            > for #column_name where
            #column_name: self::diesel::AppearsOnTable<diesel::internal::table_macro::Join<Left, Right, self::diesel::internal::table_macro::LeftOuter>>,
            Self: self::diesel::SelectableExpression<Left>,
            // If our table is on the right side of this join, only
            // `Nullable<Self>` can be selected
            Right: self::diesel::query_source::AppearsInFromClause<super::table, Count=diesel::query_source::Never> + self::diesel::query_source::QuerySource,
            Left: self::diesel::query_source::QuerySource
        {
        }

        impl<Left, Right> self::diesel::SelectableExpression<
                self::diesel::internal::table_macro::Join<Left, Right, self::diesel::internal::table_macro::Inner>,
            > for #column_name where
            #column_name: self::diesel::AppearsOnTable<diesel::internal::table_macro::Join<Left, Right, self::diesel::internal::table_macro::Inner>>,
            Left: self::diesel::query_source::AppearsInFromClause<super::table> + self::diesel::query_source::QuerySource,
            Right: self::diesel::query_source::AppearsInFromClause<super::table> + self::diesel::query_source::QuerySource,
        (Left::Count, Right::Count): self::diesel::internal::table_macro::Pick<Left, Right>,
            Self: self::diesel::SelectableExpression<
                <(Left::Count, Right::Count) as self::diesel::internal::table_macro::Pick<Left, Right>>::Selection,
            >,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        impl<Join, On> self::diesel::SelectableExpression<diesel::internal::table_macro::JoinOn<Join, On>> for #column_name where
            #column_name: self::diesel::SelectableExpression<Join> + self::diesel::AppearsOnTable<diesel::internal::table_macro::JoinOn<Join, On>>,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        impl<From> self::diesel::SelectableExpression<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<From>>> for #column_name where
            From: self::diesel::query_source::QuerySource,
            #column_name: self::diesel::SelectableExpression<From> + self::diesel::AppearsOnTable<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<From>>>,
        {
        }

        impl<__GB> self::diesel::expression::ValidGrouping<__GB> for #column_name
        where __GB: self::diesel::expression::IsContainedInGroupBy<#column_name, Output = self::diesel::expression::is_contained_in_group_by::Yes>,
        {
            type IsAggregate = self::diesel::expression::is_aggregate::Yes;
        }

        impl self::diesel::expression::ValidGrouping<()> for #column_name {
            type IsAggregate = self::diesel::expression::is_aggregate::No;
        }

        impl self::diesel::expression::IsContainedInGroupBy<#column_name> for #column_name {
            type Output = self::diesel::expression::is_contained_in_group_by::Yes;
        }

        impl self::diesel::query_source::Column for #column_name {
            type Table = super::table;

            const NAME: &'static str = #sql_name;
        }

        impl<T> self::diesel::EqAll<T> for #column_name where
            T: self::diesel::expression::AsExpression<#sql_type>,
            self::diesel::dsl::Eq<#column_name, T::Expression>: self::diesel::Expression<SqlType=diesel::sql_types::Bool>,
        {
            type Output = self::diesel::dsl::Eq<Self, T::Expression>;

            fn eq_all(self, __diesel_internal_rhs: T) -> Self::Output {
                use self::diesel::expression_methods::ExpressionMethods;
                self.eq(__diesel_internal_rhs)
            }
        }

        #ops_impls
        #backend_specific_column_impl
    }
}
