use diesel_table_macro_syntax::{ColumnDef, TableDecl};
use proc_macro2::{Span, TokenStream};
use syn::{Ident, parse_quote};

const DEFAULT_PRIMARY_KEY_NAME: &str = "id";

fn cfg_attributes(attrs: &[syn::Attribute]) -> Vec<&syn::Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .collect()
}

fn has_cfg_attributes(column: &ColumnDef) -> bool {
    column.meta.iter().any(|attr| attr.path().is_ident("cfg"))
}

struct CfgGroup<'a> {
    cfg_attrs: Vec<&'a syn::Attribute>,
    columns: Vec<&'a ColumnDef>,
}

/// Yields `2^n` flag vectors of length `n`. `n == 0` yields one empty vector,
/// so callers can handle the no-cfg-groups case without a special branch.
fn cfg_combinations(n: usize) -> impl Iterator<Item = Vec<bool>> {
    (0..1usize << n).map(move |mask| (0..n).map(|i| (mask >> i) & 1 == 1).collect())
}

fn generate_combined_cfg_condition(groups: &[CfgGroup<'_>], enabled_flags: &[bool]) -> TokenStream {
    debug_assert_eq!(groups.len(), enabled_flags.len());

    if groups.is_empty() {
        return TokenStream::new();
    }

    let mut conditions: Vec<TokenStream> = Vec::new();

    for (group, &is_enabled) in groups.iter().zip(enabled_flags) {
        let group_conditions: Vec<&TokenStream> = group
            .cfg_attrs
            .iter()
            .filter_map(|attr| match &attr.meta {
                syn::Meta::List(list) => Some(&list.tokens),
                _ => None,
            })
            .collect();

        if group_conditions.is_empty() {
            continue;
        }

        if is_enabled {
            conditions.extend(
                group_conditions
                    .iter()
                    .map(|tokens| quote::quote! { #tokens }),
            );
        } else if let [tokens] = group_conditions[..] {
            conditions.push(quote::quote! { not(#tokens) });
        } else {
            // Negate the conjunction (`not(all(...))`), not each attribute, so
            // that mixed combinations such as `x && !y` still match.
            conditions.push(quote::quote! { not(all(#(#group_conditions),*)) });
        }
    }

    if conditions.is_empty() {
        TokenStream::new()
    } else {
        quote::quote! {
            #[cfg(all(#(#conditions),*))]
        }
    }
}

struct AggregateTokens {
    all_columns_const: TokenStream,
    all_columns_type_variants: TokenStream,
}

fn all_columns_tuple_expr<'a>(
    non_gated_columns: &[&'a ColumnDef],
    cfg_groups: &[CfgGroup<'a>],
) -> TokenStream {
    let mut fields: Vec<TokenStream> = Vec::new();
    for col in non_gated_columns {
        let name = &col.column_name;
        fields.push(quote::quote! { #name, });
    }
    for group in cfg_groups {
        let attrs = &group.cfg_attrs;
        for col in &group.columns {
            let name = &col.column_name;
            fields.push(quote::quote! { #(#attrs)* #name, });
        }
    }
    quote::quote! { (#(#fields)*) }
}

/// `AllColumns` needs `2^n` cfg-gated variants because Rust does not allow
/// `#[cfg]` on tuple type fields (rust-lang/rfcs#3532). `SqlType` is derived
/// from `AllColumns` and the remaining items reference these aliases, so they
/// stay as single declarations.
fn generate_aggregate_variants<'a>(
    non_gated_columns: &[&'a ColumnDef],
    cfg_groups: &[CfgGroup<'a>],
    kind_name: &str,
) -> AggregateTokens {
    let base_column_names: Vec<_> = non_gated_columns.iter().map(|c| &c.column_name).collect();

    let mut all_columns_type_variants = Vec::new();

    for flags in cfg_combinations(cfg_groups.len()) {
        let cfg_condition = generate_combined_cfg_condition(cfg_groups, &flags);

        let mut column_names = base_column_names.clone();
        for (group, &enabled) in cfg_groups.iter().zip(&flags) {
            if enabled {
                for col in &group.columns {
                    column_names.push(&col.column_name);
                }
            }
        }

        all_columns_type_variants.push(quote::quote! {
            #cfg_condition
            #[allow(non_camel_case_types, dead_code)]
            #[doc = concat!("The tuple of all column structs on this ", #kind_name)]
            pub type AllColumns = (#(#column_names,)*);
        });
    }

    let tuple_expr = all_columns_tuple_expr(non_gated_columns, cfg_groups);

    let all_columns_const = quote::quote! {
        #[allow(non_upper_case_globals, dead_code)]
        #[doc = concat!("A tuple of all of the columns on this", #kind_name)]
        pub const all_columns: AllColumns = #tuple_expr;
    };

    AggregateTokens {
        all_columns_const,
        all_columns_type_variants: quote::quote! { #(#all_columns_type_variants)* },
    }
}

fn generate_kind_specific_impls(
    primary_key: &Option<TokenStream>,
    kind: QuerySourceMacroKind,
) -> TokenStream {
    match kind {
        QuerySourceMacroKind::Table => quote::quote! {
            impl diesel::Table for table {
                type PrimaryKey = #primary_key;
                type AllColumns = AllColumns;

                fn primary_key(&self) -> Self::PrimaryKey {
                    #primary_key
                }

                fn all_columns() -> Self::AllColumns {
                    all_columns
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
        },
        QuerySourceMacroKind::View => quote::quote! {
            #[doc(hidden)]
            pub use self::view as table;

            impl diesel::query_source::QueryRelation for view {
                type AllColumns = AllColumns;

                fn all_columns() -> Self::AllColumns {
                    all_columns
                }
            }

            impl diesel::internal::table_macro::Sealed for view {}
            impl diesel::query_source::View for view {}
        },
    }
}

fn collect_cfg_groups<'a>(
    columns: impl IntoIterator<Item = &'a ColumnDef> + Clone,
) -> Vec<CfgGroup<'a>> {
    use std::collections::HashMap;

    // syn::Attribute does not implement Hash, so we key by its token-stream
    // string representation.
    let mut groups_map: HashMap<String, CfgGroup<'a>> = HashMap::new();

    for col in columns.clone() {
        let cfg_attrs = cfg_attributes(&col.meta);
        if cfg_attrs.is_empty() {
            continue;
        }

        let key = cfg_attrs
            .iter()
            .map(|a| quote::quote!(#a).to_string())
            .collect::<Vec<_>>()
            .join(" ");

        groups_map
            .entry(key)
            .or_insert_with(|| CfgGroup {
                cfg_attrs: cfg_attrs.clone(),
                columns: Vec::new(),
            })
            .columns
            .push(col);
    }

    // Order groups by first occurrence in the original column list so that
    // generated cfg-combination output is deterministic.
    let mut groups: Vec<CfgGroup<'a>> = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();

    for col in columns {
        let cfg_attrs = cfg_attributes(&col.meta);
        if cfg_attrs.is_empty() {
            continue;
        }

        let key = cfg_attrs
            .iter()
            .map(|a| quote::quote!(#a).to_string())
            .collect::<Vec<_>>()
            .join(" ");

        if !seen_keys.contains(&key) {
            seen_keys.insert(key.clone());
            if let Some(group) = groups_map.remove(&key) {
                groups.push(group);
            }
        }
    }

    groups
}

#[derive(Clone, Copy)]
pub enum QuerySourceMacroKind {
    Table,
    View,
}

impl QuerySourceMacroKind {
    fn macro_name(&self) -> &'static str {
        match self {
            QuerySourceMacroKind::Table => "table",
            QuerySourceMacroKind::View => "view",
        }
    }
}

pub fn query_source_macro(
    tokenstream2: proc_macro2::TokenStream,
    kind: QuerySourceMacroKind,
) -> proc_macro2::TokenStream {
    // include the input in the error output so that rust-analyzer is happy
    match syn::parse2::<TableDecl>(tokenstream2.clone()) {
        Ok(input) => expand(input, kind),
        Err(_) => {
            let kind = kind.macro_name();
            quote::quote! {
                compile_error!(
                    concat!("invalid `", #kind, "!` syntax \nhelp: please see the `", #kind, "!` macro docs for more info\n\
                             help: docs available at: `https://docs.diesel.rs/", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"), ".x/diesel/macro.", #kind, ".html`\n"
                    ));
                #tokenstream2
            }
        }
    }
}

fn expand(input: TableDecl, kind: QuerySourceMacroKind) -> TokenStream {
    let kind_name = kind.macro_name();
    let column_count = input.view.column_defs.len() as u16;
    let too_many_columns_error_message = format!(
        "`{}` contains {column_count} columns, which is more than the supported maximum number of columns\n\
        Try enabling a crate level feature to support more columns",
        input.view.table_name
    );
    let meta = &input.view.meta;
    let table_name = &input.view.table_name;
    let imports = if input.view.use_statements.is_empty() {
        vec![parse_quote!(
            use diesel::sql_types::*;
        )]
    } else {
        input.view.use_statements.clone()
    };

    let non_gated_columns: Vec<_> = input
        .view
        .column_defs
        .iter()
        .filter(|c| !has_cfg_attributes(c))
        .collect();
    let cfg_groups = collect_cfg_groups(&input.view.column_defs);

    let column_names = input
        .view
        .column_defs
        .iter()
        .map(|c| &c.column_name)
        .collect::<Vec<_>>();
    let column_names = &column_names;
    let primary_key: Option<TokenStream> = if matches!(kind, QuerySourceMacroKind::Table) {
        let primary_key = match input.primary_keys.as_ref() {
            None if column_names.contains(&&syn::Ident::new(
                DEFAULT_PRIMARY_KEY_NAME,
                proc_macro2::Span::mixed_site(),
            )) =>
            {
                let id = syn::Ident::new(DEFAULT_PRIMARY_KEY_NAME, proc_macro2::Span::mixed_site());
                parse_quote! {
                    #id
                }
            }
            None => {
                let mut message = format!(
                    "neither an explicit primary key found nor does an `id` column exist.\n\
                 consider explicitly defining a primary key. \n\
                 for example for specifying `{key}` as primary key:\n\n\
                 {kind_name}! {{\n
                     {table}({key}){{\n",
                    key = column_names[0],
                    table = input.view.table_name,
                );
                message += &format!("\t{table_name} ({}) {{\n", &column_names[0]);
                for c in &input.view.column_defs {
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

                let span = Span::mixed_site().located_at(input.view.table_name.span());
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
        Some(primary_key)
    } else {
        None
    };

    let query_source_ident = match kind {
        QuerySourceMacroKind::Table => syn::Ident::new("table", input.view.table_name.span()),
        QuerySourceMacroKind::View => syn::Ident::new("view", input.view.table_name.span()),
    };

    let column_defs = input
        .view
        .column_defs
        .iter()
        .map(|c| expand_column_def(c, &query_source_ident, kind));
    let valid_grouping_for_table_columns = generate_valid_grouping_for_table_columns(&input);

    let sql_name = &input.view.sql_name;
    let static_query_fragment_impl_for_table = if let Some(schema) = input.view.schema {
        let schema_name = schema.to_string();
        quote::quote! {
            impl diesel::internal::table_macro::StaticQueryFragment for #query_source_ident {
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
            impl diesel::internal::table_macro::StaticQueryFragment for #query_source_ident {
                type Component = diesel::internal::table_macro::Identifier<'static>;
                const STATIC_COMPONENT: &'static Self::Component = &diesel::internal::table_macro::Identifier(#sql_name);
            }
        }
    };

    let reexport_column_from_dsl = input.view.column_defs.iter().map(|c| {
        let column_name = &c.column_name;
        let cfg_attrs = cfg_attributes(&c.meta);
        if c.column_name == *table_name {
            let span = Span::mixed_site().located_at(c.column_name.span());
            let message = format!(
                "column `{column_name}` cannot be named the same as it's {kind_name}.\n\
                 you may use `#[sql_name = \"{column_name}\"]` to reference the {kind_name}'s \
                 `{column_name}` column \n\
                 docs available at: `https://docs.diesel.rs/{}.x/diesel/macro.{kind_name}.html`\n",
                env!("CARGO_PKG_VERSION")
                    .rsplit_once('.')
                    .expect("This is a valid version")
                    .0
            );
            quote::quote_spanned! { span =>
                compile_error!(#message);
            }
        } else {
            quote::quote! {
                #(#cfg_attrs)*
                pub use super::columns::#column_name;
            }
        }
    });

    let AggregateTokens {
        all_columns_const,
        all_columns_type_variants,
    } = generate_aggregate_variants(&non_gated_columns, &cfg_groups, kind_name);

    let kind_specific_impls = generate_kind_specific_impls(&primary_key, kind);

    let backend_specific_table_impls = if matches!(kind, QuerySourceMacroKind::Table) {
        Some(quote::quote! {
            diesel::internal::table_macro::expand_pg! {
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
            }
            diesel::internal::table_macro::expand_pg! {
                impl diesel::query_source::AppearsInFromClause<diesel::query_builder::Only<table>>
                    for table
                {
                    type Count = diesel::query_source::Once;
                }
            }
            diesel::internal::table_macro::expand_pg! {
                impl diesel::query_source::AppearsInFromClause<table>
                    for diesel::query_builder::Only<table>
                {
                    type Count = diesel::query_source::Once;
                }
            }
            diesel::internal::table_macro::expand_pg! {
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
            }
            diesel::internal::table_macro::expand_pg! {
                impl<TSM> diesel::query_source::AppearsInFromClause<diesel::query_builder::Tablesample<table, TSM>>
                    for table
                where
                    TSM: diesel::internal::table_macro::TablesampleMethod
                {
                    type Count = diesel::query_source::Once;
                }
            }
            diesel::internal::table_macro::expand_pg! {
                impl<TSM> diesel::query_source::AppearsInFromClause<table>
                    for diesel::query_builder::Tablesample<table, TSM>
                where
                    TSM: diesel::internal::table_macro::TablesampleMethod
                {
                    type Count = diesel::query_source::Once;
                }
            }
        })
    } else {
        None
    };

    let imports_for_column_module = imports.iter().map(fix_import_for_submodule);

    quote::quote! {
        #(#meta)*
        #[allow(unused_imports, dead_code, unreachable_pub, unused_qualifications)]
        pub mod #table_name {
            const _: () = {
                assert!(
                    #column_count <= diesel::internal::table_macro::MAX_COLUMN_COUNT,
                    #too_many_columns_error_message
                );
            };

            use ::diesel;
            pub use self::columns::*;
            #(#imports)*

            #[doc = concat!("Re-exports all of the columns of this ", #kind_name, ", as well as the")]
            #[doc = concat!(#kind_name, " struct renamed to the module name. This is meant to be")]
            #[doc = concat!("glob imported for functions which only deal with one ", #kind_name, ".")]
            pub mod dsl {
                #(#reexport_column_from_dsl)*
                pub use super::#query_source_ident as #table_name;
            }

            #all_columns_const

            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[doc = concat!("The actual ", #kind_name, " struct")]
            ///
            /// This is the type which provides the base methods of the query
            /// builder, such as `.select` and `.filter`.
            pub struct #query_source_ident;

            impl #query_source_ident {
                #[allow(dead_code)]
                #[doc = concat!("Represents `", #kind_name, "_name.*`, which is sometimes necessary")]
                /// for efficient count queries. It cannot be used in place of
                /// `all_columns`
                pub fn star(&self) -> star {
                    star
                }
            }

            #all_columns_type_variants

            #[doc = concat!("The SQL type of all of the columns on this ", #kind_name)]
            pub type SqlType = <AllColumns as diesel::Expression>::SqlType;

            #[doc = concat!("Helper type for representing a boxed query from this ", #kind_name)]
            pub type BoxedQuery<'a, DB, ST = SqlType> = diesel::internal::table_macro::BoxedSelectStatement<'a, ST, diesel::internal::table_macro::FromClause<#query_source_ident>, DB>;

            impl diesel::QuerySource for #query_source_ident {
                type FromClause = diesel::internal::table_macro::StaticQueryFragmentInstance<#query_source_ident>;
                type DefaultSelection = <Self as diesel::query_source::QueryRelation>::AllColumns;

                fn from_clause(&self) -> Self::FromClause {
                    diesel::internal::table_macro::StaticQueryFragmentInstance::new()
                }

                fn default_selection(&self) -> Self::DefaultSelection {
                    <Self as diesel::query_source::QueryRelation>::all_columns()
                }
            }

            impl diesel::internal::table_macro::PlainQuerySource for #query_source_ident {}

            impl<DB> diesel::query_builder::QueryFragment<DB> for #query_source_ident where
                DB: diesel::backend::Backend,
                <Self as diesel::internal::table_macro::StaticQueryFragment>::Component: diesel::query_builder::QueryFragment<DB>
            {
                fn walk_ast<'b>(&'b self, __diesel_internal_pass: diesel::query_builder::AstPass<'_, 'b, DB>) -> diesel::result::QueryResult<()> {
                    <Self as diesel::internal::table_macro::StaticQueryFragment>::STATIC_COMPONENT.walk_ast(__diesel_internal_pass)
                }
            }

            #static_query_fragment_impl_for_table

            impl diesel::query_builder::AsQuery for #query_source_ident {
                type SqlType = SqlType;
                type Query = diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<Self>>;

                fn as_query(self) -> Self::Query {
                    diesel::internal::table_macro::SelectStatement::simple(self)
                }
            }

            #kind_specific_impls

            impl diesel::query_source::AppearsInFromClause<Self> for #query_source_ident {
                type Count = diesel::query_source::Once;
            }

            // impl<S: AliasSource<Table=table>> AppearsInFromClause<table> for Alias<S>
            impl<S> diesel::internal::table_macro::AliasAppearsInFromClause<S, Self> for #query_source_ident
            where S: diesel::query_source::AliasSource<Target = Self>,
            {
                type Count = diesel::query_source::Never;
            }

            // impl<S1: AliasSource<Table=table>, S2: AliasSource<Table=table>> AppearsInFromClause<Alias<S1>> for Alias<S2>
            // Those are specified by the `alias!` macro, but this impl will allow it to implement this trait even in downstream
            // crates from the schema
            impl<S1, S2> diesel::internal::table_macro::AliasAliasAppearsInFromClause<Self, S2, S1> for #query_source_ident
            where S1: diesel::query_source::AliasSource<Target = Self>,
                  S2: diesel::query_source::AliasSource<Target = Self>,
                  S1: diesel::internal::table_macro::AliasAliasAppearsInFromClauseSameTable<S2, Self>,
            {
                type Count = <S1 as diesel::internal::table_macro::AliasAliasAppearsInFromClauseSameTable<S2, Self>>::Count;
            }

            impl<S> diesel::query_source::AppearsInFromClause<diesel::query_source::Alias<S>> for #query_source_ident
            where S: diesel::query_source::AliasSource,
            {
                type Count = diesel::query_source::Never;
            }

            impl<S, C> diesel::internal::table_macro::FieldAliasMapperAssociatedTypesDisjointnessTrick<Self, S, C> for #query_source_ident
            where
                S: diesel::query_source::AliasSource<Target = Self> + ::core::clone::Clone,
                C: diesel::query_source::QueryRelationField<QueryRelation = Self>,
            {
                type Out = diesel::query_source::AliasedField<S, C>;

                fn map(__diesel_internal_column: C, __diesel_internal_alias: &diesel::query_source::Alias<S>) -> Self::Out {
                    __diesel_internal_alias.field(__diesel_internal_column)
                }
            }

            impl<StmtKind> diesel::query_source::AppearsInFromClause<#query_source_ident> for diesel::internal::table_macro::returning::ReturningQuerySource<StmtKind, #query_source_ident>
            {
                type Count = diesel::query_source::Once;
            }

            impl<StmtKind, T> diesel::query_source::AppearsInFromClause<diesel::internal::table_macro::returning::ReturningQuerySource<StmtKind, T>> for #query_source_ident {
                type Count = diesel::query_source::Never;
            }

            impl diesel::query_source::AppearsInFromClause<#query_source_ident> for diesel::internal::table_macro::NoFromClause {
                type Count = diesel::query_source::Never;
            }

            impl<Left, Right, Kind> diesel::JoinTo<diesel::internal::table_macro::Join<Left, Right, Kind>> for #query_source_ident where
                diesel::internal::table_macro::Join<Left, Right, Kind>: diesel::JoinTo<Self>,
                Left: diesel::query_source::QuerySource,
                Right: diesel::query_source::QuerySource,
            {
                type FromClause = diesel::internal::table_macro::Join<Left, Right, Kind>;
                type OnClause = <diesel::internal::table_macro::Join<Left, Right, Kind> as diesel::JoinTo<Self>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::Join<Left, Right, Kind>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::Join::join_target(Self);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<Join, On> diesel::JoinTo<diesel::internal::table_macro::JoinOn<Join, On>> for #query_source_ident where
                diesel::internal::table_macro::JoinOn<Join, On>: diesel::JoinTo<Self>,
            {
                type FromClause = diesel::internal::table_macro::JoinOn<Join, On>;
                type OnClause = <diesel::internal::table_macro::JoinOn<Join, On> as diesel::JoinTo<Self>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::JoinOn<Join, On>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::JoinOn::join_target(Self);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<F, S, D, W, O, L, Of, G> diesel::JoinTo<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>> for #query_source_ident where
                diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>: diesel::JoinTo<Self>,
                F: diesel::query_source::QuerySource
            {
                type FromClause = diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>;
                type OnClause = <diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G> as diesel::JoinTo<Self>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<F>, S, D, W, O, L, Of, G>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::SelectStatement::join_target(Self);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<'a, QS, ST, DB> diesel::JoinTo<diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>> for #query_source_ident where
                diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>: diesel::JoinTo<Self>,
                QS: diesel::query_source::QuerySource,
            {
                type FromClause = diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>;
                type OnClause = <diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB> as diesel::JoinTo<Self>>::OnClause;
                fn join_target(__diesel_internal_rhs: diesel::internal::table_macro::BoxedSelectStatement<'a, diesel::internal::table_macro::FromClause<QS>, ST, DB>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::internal::table_macro::BoxedSelectStatement::join_target(Self);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            impl<S> diesel::JoinTo<diesel::query_source::Alias<S>> for #query_source_ident
            where
                diesel::query_source::Alias<S>: diesel::JoinTo<Self>,
            {
                type FromClause = diesel::query_source::Alias<S>;
                type OnClause = <diesel::query_source::Alias<S> as diesel::JoinTo<Self>>::OnClause;

                fn join_target(__diesel_internal_rhs: diesel::query_source::Alias<S>) -> (Self::FromClause, Self::OnClause) {
                    let (_, __diesel_internal_on_clause) = diesel::query_source::Alias::<S>::join_target(Self);
                    (__diesel_internal_rhs, __diesel_internal_on_clause)
                }
            }

            #backend_specific_table_impls

            #[doc = concat!("Contains all of the columns of this ", #kind_name)]
            pub mod columns {
                use ::diesel;
                use super::#query_source_ident;
                #(#imports_for_column_module)*

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId, PartialEq, Eq, PartialOrd, Ord, Hash)]
                #[doc = concat!("Represents `", #kind_name, "_name.*`, which is sometimes needed for")]
                /// efficient count queries. It cannot be used in place of
                /// `all_columns`, and has a `SqlType` of `()` to prevent it
                /// being used that way
                pub struct star;

                impl<__GB> diesel::expression::ValidGrouping<__GB> for star
                where
                    super::AllColumns: diesel::expression::ValidGrouping<__GB>,
                {
                    type IsAggregate =
                        <super::AllColumns as diesel::expression::ValidGrouping<__GB>>::IsAggregate;
                }

                impl diesel::Expression for star {
                    type SqlType = diesel::expression::expression_types::NotSelectable;
                }

                impl<DB: diesel::backend::Backend> diesel::query_builder::QueryFragment<DB> for star where
                    <#query_source_ident as diesel::QuerySource>::FromClause: diesel::query_builder::QueryFragment<DB>,
                {
                    #[allow(non_snake_case)]
                    fn walk_ast<'b>(&'b self, mut __diesel_internal_out: diesel::query_builder::AstPass<'_, 'b, DB>) -> diesel::result::QueryResult<()>
                    {
                        use diesel::QuerySource;

                        if !__diesel_internal_out.should_skip_from() {
                            const FROM_CLAUSE: diesel::internal::table_macro::StaticQueryFragmentInstance<#query_source_ident> = diesel::internal::table_macro::StaticQueryFragmentInstance::new();

                            FROM_CLAUSE.walk_ast(__diesel_internal_out.reborrow())?;
                            __diesel_internal_out.push_sql(".");
                        }
                        __diesel_internal_out.push_sql("*");
                        Ok(())
                    }
                }

                impl diesel::SelectableExpression<#query_source_ident> for star {}

                impl diesel::AppearsOnTable<#query_source_ident> for star {}

                #(#column_defs)*

                #(#valid_grouping_for_table_columns)*
            }
        }
    }
}

fn generate_valid_grouping_for_table_columns(table: &TableDecl) -> Vec<TokenStream> {
    let mut ret = Vec::with_capacity(table.view.column_defs.len() * table.view.column_defs.len());

    let primary_key = if let Some(ref pk) = table.primary_keys {
        if pk.keys.len() == 1 {
            pk.keys.first().map(ToString::to_string)
        } else {
            None
        }
    } else {
        Some(DEFAULT_PRIMARY_KEY_NAME.into())
    };

    for (id, right_col_def) in table.view.column_defs.iter().enumerate() {
        for left_col_def in table.view.column_defs.iter().skip(id) {
            let right_to_left = if Some(left_col_def.column_name.to_string()) == primary_key {
                Ident::new("Yes", proc_macro2::Span::mixed_site())
            } else {
                Ident::new("No", proc_macro2::Span::mixed_site())
            };

            let left_to_right = if Some(right_col_def.column_name.to_string()) == primary_key {
                Ident::new("Yes", proc_macro2::Span::mixed_site())
            } else {
                Ident::new("No", proc_macro2::Span::mixed_site())
            };

            let left_col = &left_col_def.column_name;
            let right_col = &right_col_def.column_name;

            let left_cfg_attrs = cfg_attributes(&left_col_def.meta);
            let right_cfg_attrs = cfg_attributes(&right_col_def.meta);

            if left_col != right_col {
                ret.push(quote::quote! {
                    #(#left_cfg_attrs)*
                    #(#right_cfg_attrs)*
                    impl diesel::expression::IsContainedInGroupBy<#right_col> for #left_col {
                        type Output = diesel::expression::is_contained_in_group_by::#right_to_left;
                    }

                    #(#left_cfg_attrs)*
                    #(#right_cfg_attrs)*
                    impl diesel::expression::IsContainedInGroupBy<#left_col> for #right_col {
                        type Output = diesel::expression::is_contained_in_group_by::#left_to_right;
                    }
                });
            }
        }
    }
    ret
}

/// Imports inside the nested `columns` submodule see `super::` as the table
/// module rather than its parent, so any `use super::...` needs another
/// `super::` prepended to resolve correctly.
fn fix_import_for_submodule(import: &syn::ItemUse) -> syn::ItemUse {
    let mut ret = import.clone();

    if let syn::UseTree::Path(ref mut path) = ret.tree
        && path.ident == "super"
    {
        let inner = path.clone();
        *path.tree = syn::UseTree::Path(inner);
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
        "Decimal",
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

fn generate_op_impl(op: &str, tpe: &syn::Ident, cfg_attrs: &[&syn::Attribute]) -> TokenStream {
    let fn_name = syn::Ident::new(&op.to_lowercase(), tpe.span());
    let op = syn::Ident::new(op, tpe.span());
    quote::quote! {
        #(#cfg_attrs)*
        impl<Rhs> ::core::ops::#op<Rhs> for #tpe
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

fn expand_column_def(
    column_def: &ColumnDef,
    query_source_ident: &Ident,
    kind: QuerySourceMacroKind,
) -> TokenStream {
    // TODO get a better span here as soon as that's
    // possible using stable rust
    let span = Span::mixed_site().located_at(column_def.column_name.span());
    let meta = &column_def.meta;
    let cfg_attrs = cfg_attributes(&column_def.meta);
    let column_name = &column_def.column_name;
    let sql_name = &column_def.sql_name;
    let sql_type = &column_def.tpe;

    let backend_specific_column_impl = if matches!(kind, QuerySourceMacroKind::Table) {
        Some(quote::quote! {
            #(#cfg_attrs)*
            diesel::internal::table_macro::expand_pg! {
                impl diesel::query_source::AppearsInFromClause<diesel::query_builder::Only<super::table>>
                    for #column_name
                {
                    type Count = diesel::query_source::Once;
                }
            }
            #(#cfg_attrs)*
            diesel::internal::table_macro::expand_pg! {
                impl diesel::SelectableExpression<diesel::query_builder::Only<super::table>> for #column_name {}
            }
            #(#cfg_attrs)*
            diesel::internal::table_macro::expand_pg! {
                impl<TSM> diesel::query_source::AppearsInFromClause<diesel::query_builder::Tablesample<super::table, TSM>>
                    for #column_name
                where
                    TSM: diesel::internal::table_macro::TablesampleMethod
                {
                    type Count = diesel::query_source::Once;
                }
            }
            #(#cfg_attrs)*
            diesel::internal::table_macro::expand_pg! {
                impl<TSM> diesel::SelectableExpression<diesel::query_builder::Tablesample<super::table, TSM>>
                    for #column_name where TSM: diesel::internal::table_macro::TablesampleMethod {}
            }
        })
    } else {
        None
    };

    let ops_impls = if is_numeric(&column_def.tpe) {
        let add = generate_op_impl("Add", column_name, &cfg_attrs);
        let sub = generate_op_impl("Sub", column_name, &cfg_attrs);
        let div = generate_op_impl("Div", column_name, &cfg_attrs);
        let mul = generate_op_impl("Mul", column_name, &cfg_attrs);
        Some(quote::quote! {
            #add
            #sub
            #div
            #mul
        })
    } else if is_date_time(&column_def.tpe) || is_network(&column_def.tpe) {
        let add = generate_op_impl("Add", column_name, &cfg_attrs);
        let sub = generate_op_impl("Sub", column_name, &cfg_attrs);
        Some(quote::quote! {
            #add
            #sub
        })
    } else {
        None
    };

    let max_length = column_def.max_length.as_ref().map(|column_max_length| {
        quote::quote! {
            #(#cfg_attrs)*
            impl self::diesel::query_source::SizeRestrictedColumn for #column_name {
                const MAX_LENGTH: usize = #column_max_length;
            }
        }
    });

    let table_specific_impls = if matches!(kind, QuerySourceMacroKind::Table) {
        quote::quote! {
            #(#cfg_attrs)*
            impl diesel::query_source::Column for #column_name {
                type Table = super::table;

                const NAME: &'static str = #sql_name;
            }
        }
    } else {
        quote::quote! {
            #(#cfg_attrs)*
            impl diesel::query_source::QueryRelationField for #column_name {
                type QueryRelation = super::view;

                const NAME: &'static str = #sql_name;
            }
        }
    };

    quote::quote_spanned! {span=>
        #(#meta)*
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct #column_name;

        #(#cfg_attrs)*
        impl diesel::expression::Expression for #column_name {
            type SqlType = #sql_type;
        }

        #(#cfg_attrs)*
        impl<DB> diesel::query_builder::QueryFragment<DB> for #column_name where
            DB: diesel::backend::Backend,
            diesel::internal::table_macro::StaticQueryFragmentInstance<#query_source_ident>: diesel::query_builder::QueryFragment<DB>,
        {
            #[allow(non_snake_case)]
            fn walk_ast<'b>(&'b self, mut __diesel_internal_out: diesel::query_builder::AstPass<'_, 'b, DB>) -> diesel::result::QueryResult<()>
            {
                if !__diesel_internal_out.should_skip_from() {
                    const FROM_CLAUSE: diesel::internal::table_macro::StaticQueryFragmentInstance<#query_source_ident> = diesel::internal::table_macro::StaticQueryFragmentInstance::new();

                    FROM_CLAUSE.walk_ast(__diesel_internal_out.reborrow())?;
                    __diesel_internal_out.push_sql(".");
                }
                __diesel_internal_out.push_identifier(#sql_name)
            }
        }

        #(#cfg_attrs)*
        impl diesel::SelectableExpression<super::#query_source_ident> for #column_name {
        }

        #(#cfg_attrs)*
        impl<__StmtKind>
            diesel::SelectableExpression<
                diesel::internal::table_macro::returning::ReturningQuerySource<
                    __StmtKind,
                    super::#query_source_ident,
                >,
            > for #column_name
        {
        }

        #(#cfg_attrs)*
        impl<QS> diesel::AppearsOnTable<QS> for #column_name where
            QS: diesel::query_source::AppearsInFromClause<super::#query_source_ident, Count=diesel::query_source::Once>,
        {
        }

        #(#cfg_attrs)*
        impl<Left, Right> diesel::SelectableExpression<
                diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::LeftOuter>,
            > for #column_name where
            #column_name: diesel::AppearsOnTable<diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::LeftOuter>>,
            Self: diesel::SelectableExpression<Left>,
            // If our table is on the right side of this join, only
            // `Nullable<Self>` can be selected
            Right: diesel::query_source::AppearsInFromClause<super::#query_source_ident, Count=diesel::query_source::Never> + diesel::query_source::QuerySource,
            Left: diesel::query_source::QuerySource
        {
        }

        #(#cfg_attrs)*
        impl<Left, Right> diesel::SelectableExpression<
                diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::Inner>,
            > for #column_name where
            #column_name: diesel::AppearsOnTable<diesel::internal::table_macro::Join<Left, Right, diesel::internal::table_macro::Inner>>,
            Left: diesel::query_source::AppearsInFromClause<super::#query_source_ident> + diesel::query_source::QuerySource,
            Right: diesel::query_source::AppearsInFromClause<super::#query_source_ident> + diesel::query_source::QuerySource,
        (Left::Count, Right::Count): diesel::internal::table_macro::Pick<Left, Right>,
            Self: diesel::SelectableExpression<
                <(Left::Count, Right::Count) as diesel::internal::table_macro::Pick<Left, Right>>::Selection,
            >,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        #(#cfg_attrs)*
        impl<Join, On> diesel::SelectableExpression<diesel::internal::table_macro::JoinOn<Join, On>> for #column_name where
            #column_name: diesel::SelectableExpression<Join> + diesel::AppearsOnTable<diesel::internal::table_macro::JoinOn<Join, On>>,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        #(#cfg_attrs)*
        impl<From> diesel::SelectableExpression<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<From>>> for #column_name where
            From: diesel::query_source::QuerySource,
            #column_name: diesel::SelectableExpression<From> + diesel::AppearsOnTable<diesel::internal::table_macro::SelectStatement<diesel::internal::table_macro::FromClause<From>>>,
        {
        }

        #(#cfg_attrs)*
        impl<__GB> diesel::expression::ValidGrouping<__GB> for #column_name
        where __GB: diesel::expression::IsContainedInGroupBy<#column_name, Output = diesel::expression::is_contained_in_group_by::Yes>,
        {
            type IsAggregate = diesel::expression::is_aggregate::Yes;
        }

        #(#cfg_attrs)*
        impl diesel::expression::ValidGrouping<()> for #column_name {
            type IsAggregate = diesel::expression::is_aggregate::No;
        }

        #(#cfg_attrs)*
        impl diesel::expression::IsContainedInGroupBy<#column_name> for #column_name {
            type Output = diesel::expression::is_contained_in_group_by::Yes;
        }



        #(#cfg_attrs)*
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
        #table_specific_impls
    }
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn cfg_combinations_zero_groups_yields_single_empty_vector() {
        let combos: Vec<Vec<bool>> = cfg_combinations(0).collect();
        assert_eq!(combos, vec![Vec::<bool>::new()]);
    }

    #[test]
    fn cfg_combinations_enumerates_every_flag_vector() {
        let combos: Vec<Vec<bool>> = cfg_combinations(2).collect();
        assert_eq!(combos.len(), 4);
        assert!(combos.contains(&vec![false, false]));
        assert!(combos.contains(&vec![true, false]));
        assert!(combos.contains(&vec![false, true]));
        assert!(combos.contains(&vec![true, true]));
    }

    #[test]
    fn combined_cfg_condition_is_empty_without_groups() {
        let condition = generate_combined_cfg_condition(&[], &[]);
        assert!(condition.is_empty());
    }

    #[test]
    fn combined_cfg_condition_negates_disabled_groups() {
        let attr: syn::Attribute = parse_quote!(#[cfg(feature = "chrono")]);
        let group = CfgGroup {
            cfg_attrs: vec![&attr],
            columns: Vec::new(),
        };

        let enabled = generate_combined_cfg_condition(std::slice::from_ref(&group), &[true]);
        assert_eq!(
            enabled.to_string(),
            quote::quote! { #[cfg(all(feature = "chrono"))] }.to_string()
        );

        let disabled = generate_combined_cfg_condition(std::slice::from_ref(&group), &[false]);
        assert_eq!(
            disabled.to_string(),
            quote::quote! { #[cfg(all(not(feature = "chrono")))] }.to_string()
        );
    }

    #[test]
    fn combined_cfg_condition_negates_the_conjunction_of_multiple_attrs() {
        let first: syn::Attribute = parse_quote!(#[cfg(feature = "x")]);
        let second: syn::Attribute = parse_quote!(#[cfg(feature = "y")]);
        let group = CfgGroup {
            cfg_attrs: vec![&first, &second],
            columns: Vec::new(),
        };

        let enabled = generate_combined_cfg_condition(std::slice::from_ref(&group), &[true]);
        assert_eq!(
            enabled.to_string(),
            quote::quote! { #[cfg(all(feature = "x", feature = "y"))] }.to_string()
        );

        let disabled = generate_combined_cfg_condition(std::slice::from_ref(&group), &[false]);
        assert_eq!(
            disabled.to_string(),
            quote::quote! { #[cfg(all(not(all(feature = "x", feature = "y"))))] }.to_string()
        );
    }
}
