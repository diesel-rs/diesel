use syn;
use quote;

use model::Model;
use attr::Attr;
use util::wrap_item_in_const;

pub fn derive_insertable(item: syn::MacroInput) -> quote::Tokens {
    let model = t!(Model::from_item(&item, "Insertable"));

    if !model.has_table_name_annotation() {
        panic!(r#"`#[derive(Insertable)]` requires the struct to be annotated \
            with `#[table_name="something"]`"#);
    }

    if !model.generics.ty_params.is_empty() {
        panic!("`#[derive(Insertable)]` does not support generic types");
    }

    let impl_insertable = generate_impl_insertable(&model);
    let impl_into_insert_statement = generate_impl_into_insert_statement(&model);

    let model_name_uppercase = model.name.as_ref().to_uppercase();
    let dummy_const = format!("_IMPL_INSERTABLE_FOR_{}", model_name_uppercase).into();

    wrap_item_in_const(dummy_const, quote!(
        #impl_insertable
        #impl_into_insert_statement
    ))
}

fn generate_impl_insertable(model: &Model) -> quote::Tokens {
    let insert = syn::LifetimeDef::new("'insert");
    let generics = syn::aster::from_generics(model.generics.clone())
        .ty_param_id("DB")
        .with_lifetime(insert.clone())
        .build();

    let struct_ty = &model.ty;
    let struct_name = format!("{}", model.name);
    let table_name = &model.table_name();

    let build_expr = model.build_expr(quote!(ref));

    let value_type = model.attrs.as_slice().iter().map(|a|
        make_column_insert_value_definition(a, table_name, &struct_name));
    let value_type = quote!((#(#value_type,)*));

    let value_tuple = model.attrs.as_slice().iter().map(|a|
        make_column_insert_value(a, table_name, &struct_name)
    );
    let value_tuple = quote!((#(#value_tuple,)*));

    let value_fn = quote!(
        #[allow(non_shorthand_field_patterns)]
        fn values(self) -> Self::Values {
            use diesel::expression::{AsExpression, Expression};
            use diesel::insertable::ColumnInsertValue;
            use diesel::types::IntoNullable;
            let #build_expr = *self;
            #value_tuple
        });

    quote!(
        impl#generics diesel::insertable::Insertable<#table_name::table, DB>
            for &#insert #struct_ty
            where DB: diesel::backend::Backend,
                  #value_type: diesel::insertable::InsertValues<DB>,
        {
            type Values = #value_type;

            #value_fn
        }
    )
}

fn make_column_insert_value_definition(
    a: &Attr,
    table_name: &syn::Ident,
    struct_name: &str
) -> quote::Tokens {
    let column_name = a.column_name
        .as_ref()
        .expect(&format!("Unknown column name while implementing Insertable for {}"
                         , struct_name));
    let field_ty = &a.ty;
    quote!(
        diesel::insertable::ColumnInsertValue<
            #table_name::#column_name,
            diesel::expression::helper_types::AsNullableExpr<
                &'insert #field_ty,
                #table_name::#column_name,
            >
        >)
}

fn make_column_insert_value(
    a: &Attr,
    table_name: &syn::Ident,
    struct_name: &str
) -> quote::Tokens {
    let column_name = a.column_name
        .as_ref()
        .expect(&format!("Unknown column name while implementing Insertable for {}"
                         , struct_name));
    let column = quote!(#table_name::#column_name);
    let field_access = a.name_for_pattern();

    match a.field_kind() {
        "option" => {
            quote!(
                match #field_access {
                    value @ &Some(_) => {
                        ColumnInsertValue::Expression(
                            #column,
                            AsExpression::<<<#column as Expression>::SqlType
                                as IntoNullable>::Nullable>::as_expression(value)
                        )
                    },
                    &None => ColumnInsertValue::Default(#column),
                }
            )
        }
        "regular" => {
            quote!(
                ColumnInsertValue::Expression(
                    #column,
                    AsExpression::<<<#column as Expression>::SqlType
                        as IntoNullable>::Nullable>::as_expression(#field_access)
                )
            )
        },
        _ => panic!("Unknown field kind while implementing Insertable for {}", struct_name),
    }
}

fn generate_impl_into_insert_statement(model: &Model) -> quote::Tokens {
    let insert = syn::LifetimeDef::new("'insert");
    let generics = syn::aster::from_generics(model.generics.clone())
        .with_lifetime(insert.clone())
        .ty_param_id("Op")
        .ty_param_id("Ret")
        .with_predicates(model.generics.lifetimes
                         .iter()
                         .map(|l|{
                             syn::WherePredicate::RegionPredicate(
                                 syn::WhereRegionPredicate{
                                     lifetime: l.lifetime.clone(),
                                     bounds: vec![insert.lifetime.clone()]
                                 }
                             )
                         }))
        .build();

    let struct_ty = &model.ty;
    let table_name = &model.table_name();

    let into_insert_statement = quote!(
        fn into_insert_statement(
            self,
            target: #table_name::table,
            operator: Op,
            returning: Ret
        ) -> Self::InsertStatement {
            diesel::query_builder::insert_statement::InsertStatement::new(
                target,
                self,
                operator,
                returning,
            )
        }
    );

    quote!(
        impl#generics diesel::query_builder::insert_statement::IntoInsertStatement<
            #table_name::table,
            Op,
             Ret
        > for &'insert #struct_ty
        {
            type InsertStatement =
                diesel::query_builder::insert_statement::InsertStatement<
                    #table_name::table,
                    Self,
                    Op,
                    Ret
                >;

            #into_insert_statement
        }
    )
}
