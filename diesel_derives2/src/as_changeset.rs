use quote;
use syn;

use field::*;
use meta::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> quote::Tokens {
    let treat_none_as_null = MetaItem::with_name(&item.attrs, "changeset_options")
        .map(|meta| {
            meta.nested_item("treat_none_as_null")
                .required()
                .expect_bool_value()
        })
        .unwrap_or(false);
    let model = Model::from_item(&item);
    let struct_name = &model.name;
    let table_name = model.table_name();

    let (_, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut impl_generics = item.generics.clone();
    impl_generics.params.push(parse_quote!('update));
    let (impl_generics, _, _) = impl_generics.split_for_impl();

    let fields_for_update = model
        .fields()
        .iter()
        .filter(|f| !model.primary_key_names.contains(&f.column_name()))
        .collect::<Vec<_>>();
    let changeset_ty = fields_for_update
        .iter()
        .map(|field| field_changeset_ty(field, table_name, treat_none_as_null));
    let changeset_expr = fields_for_update
        .iter()
        .map(|field| field_changeset_expr(field, table_name, treat_none_as_null));

    if fields_for_update.is_empty() {
        panic!(
            "Deriving `AsChangeset` on a structure that only contains the primary key isn't \
             supported. If you want to change the primary key of a row, you should do so with \
             `.set(table::id.eq(new_id))`. `AsChangeset` never changes the primary key of a row."
        );
    }

    wrap_in_dummy_mod(
        model.dummy_mod_name("as_changeset"),
        quote!(
            use self::diesel::query_builder::AsChangeset;

            impl #impl_generics AsChangeset for &'update #struct_name #ty_generics
            #where_clause
            {
                type Target = #table_name::table;
                type Changeset = <(#(#changeset_ty,)*) as AsChangeset>::Changeset;

                fn as_changeset(self) -> Self::Changeset {
                    (#(#changeset_expr,)*).as_changeset()
                }
            }
        ),
    )
}

fn field_changeset_ty(
    field: &Field,
    table_name: syn::Ident,
    treat_none_as_null: bool,
) -> syn::Type {
    let column_name = field.column_name();
    if !treat_none_as_null && is_option_ty(&field.ty) {
        let field_ty = inner_of_option_ty(&field.ty);
        parse_quote!(std::option::Option<diesel::dsl::Eq<#table_name::#column_name, &'update #field_ty>>)
    } else {
        let field_ty = &field.ty;
        parse_quote!(diesel::dsl::Eq<#table_name::#column_name, &'update #field_ty>)
    }
}

fn field_changeset_expr(
    field: &Field,
    table_name: syn::Ident,
    treat_none_as_null: bool,
) -> syn::Expr {
    let field_access = &field.name;
    let column_name = field.column_name();
    if !treat_none_as_null && is_option_ty(&field.ty) {
        parse_quote!(self#field_access.as_ref().map(|x| #table_name::#column_name.eq(x)))
    } else {
        parse_quote!(#table_name::#column_name.eq(&self#field_access))
    }
}
