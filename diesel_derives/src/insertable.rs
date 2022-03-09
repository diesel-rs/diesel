use proc_macro2::TokenStream;
use syn::{DeriveInput, Expr, Path, Type};

use attrs::AttributeSpanWrapper;
use field::Field;
use model::Model;
use util::{inner_of_option_ty, is_option_ty, wrap_in_dummy_mod};

pub fn derive(item: DeriveInput) -> TokenStream {
    let model = Model::from_item(&item, false);

    let treat_none_as_default_value = model.treat_none_as_default_value();
    let table_name = &model.table_name();
    let struct_name = &item.ident;

    let (_, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut impl_generics = item.generics.clone();
    impl_generics.params.push(parse_quote!('insert));
    let (impl_generics, ..) = impl_generics.split_for_impl();

    let mut generate_borrowed_insert = true;

    let mut direct_field_ty = Vec::with_capacity(model.fields().len());
    let mut direct_field_assign = Vec::with_capacity(model.fields().len());
    let mut ref_field_ty = Vec::with_capacity(model.fields().len());
    let mut ref_field_assign = Vec::with_capacity(model.fields().len());

    for field in model.fields() {
        match (field.serialize_as.as_ref(), field.embed()) {
            (None, true) => {
                direct_field_ty.push(field_ty_embed(field, None));
                direct_field_assign.push(field_expr_embed(field, None));
                ref_field_ty.push(field_ty_embed(field, Some(quote!(&'insert))));
                ref_field_assign.push(field_expr_embed(field, Some(quote!(&))));
            }
            (None, false) => {
                direct_field_ty.push(field_ty(
                    field,
                    table_name,
                    None,
                    treat_none_as_default_value,
                ));
                direct_field_assign.push(field_expr(
                    field,
                    table_name,
                    None,
                    treat_none_as_default_value,
                ));
                ref_field_ty.push(field_ty(
                    field,
                    table_name,
                    Some(quote!(&'insert)),
                    treat_none_as_default_value,
                ));
                ref_field_assign.push(field_expr(
                    field,
                    table_name,
                    Some(quote!(&)),
                    treat_none_as_default_value,
                ));
            }
            (Some(AttributeSpanWrapper { item: ty, .. }), false) => {
                direct_field_ty.push(field_ty_serialize_as(
                    field,
                    table_name,
                    ty,
                    treat_none_as_default_value,
                ));
                direct_field_assign.push(field_expr_serialize_as(
                    field,
                    table_name,
                    ty,
                    treat_none_as_default_value,
                ));

                generate_borrowed_insert = false; // as soon as we hit one field with #[diesel(serialize_as)] there is no point in generating the impl of Insertable for borrowed structs
            }
            (Some(AttributeSpanWrapper { attribute_span, .. }), true) => {
                abort!(
                    attribute_span,
                    "`#[diesel(embed)]` cannot be combined with `#[diesel(serialize_as)]`"
                )
            }
        }
    }

    let insert_owned = quote! {
        impl #impl_generics Insertable<#table_name::table> for #struct_name #ty_generics
            #where_clause
        {
            type Values = <(#(#direct_field_ty,)*) as Insertable<#table_name::table>>::Values;

            fn values(self) -> Self::Values {
                (#(#direct_field_assign,)*).values()
            }
        }
    };

    let insert_borrowed = if generate_borrowed_insert {
        quote! {
            impl #impl_generics Insertable<#table_name::table>
                for &'insert #struct_name #ty_generics
            #where_clause
            {
                type Values = <(#(#ref_field_ty,)*) as Insertable<#table_name::table>>::Values;

                fn values(self) -> Self::Values {
                    (#(#ref_field_assign,)*).values()
                }
            }
        }
    } else {
        quote! {}
    };

    wrap_in_dummy_mod(quote! {
        use diesel::insertable::Insertable;
        use diesel::internal::derives::insertable::UndecoratedInsertRecord;
        use diesel::prelude::*;

        #insert_owned

        #insert_borrowed

        impl #impl_generics UndecoratedInsertRecord<#table_name::table>
                for #struct_name #ty_generics
            #where_clause
        {
        }
    })
}

fn field_ty_embed(field: &Field, lifetime: Option<TokenStream>) -> TokenStream {
    let field_ty = &field.ty;

    quote!(#lifetime #field_ty)
}

fn field_expr_embed(field: &Field, lifetime: Option<TokenStream>) -> TokenStream {
    let field_name = &field.name;

    quote!(#lifetime self.#field_name)
}

fn field_ty_serialize_as(
    field: &Field,
    table_name: &Path,
    ty: &Type,
    treat_none_as_default_value: bool,
) -> TokenStream {
    let column_name = field.column_name();

    if treat_none_as_default_value {
        let inner_ty = inner_of_option_ty(ty);

        quote!(
            std::option::Option<diesel::dsl::Eq<
                #table_name::#column_name,
                #inner_ty,
            >>
        )
    } else {
        quote!(
            diesel::dsl::Eq<
                #table_name::#column_name,
                #ty,
            >
        )
    }
}

fn field_expr_serialize_as(
    field: &Field,
    table_name: &Path,
    ty: &Type,
    treat_none_as_default_value: bool,
) -> TokenStream {
    let field_name = &field.name;
    let column_name = field.column_name();
    let column = quote!(#table_name::#column_name);

    if treat_none_as_default_value {
        if is_option_ty(ty) {
            quote!(self.#field_name.map(|x| #column.eq(::std::convert::Into::<#ty>::into(x))))
        } else {
            quote!(std::option::Option::Some(#column.eq(::std::convert::Into::<#ty>::into(self.#field_name))))
        }
    } else {
        quote!(#column.eq(::std::convert::Into::<#ty>::into(self.#field_name)))
    }
}

fn field_ty(
    field: &Field,
    table_name: &Path,
    lifetime: Option<TokenStream>,
    treat_none_as_default_value: bool,
) -> TokenStream {
    let column_name = field.column_name();

    if treat_none_as_default_value {
        let inner_ty = inner_of_option_ty(&field.ty);

        quote!(
            std::option::Option<diesel::dsl::Eq<
                #table_name::#column_name,
                #lifetime #inner_ty,
            >>
        )
    } else {
        let inner_ty = &field.ty;

        quote!(
            diesel::dsl::Eq<
                #table_name::#column_name,
                #lifetime #inner_ty,
            >
        )
    }
}

fn field_expr(
    field: &Field,
    table_name: &Path,
    lifetime: Option<TokenStream>,
    treat_none_as_default_value: bool,
) -> TokenStream {
    let field_name = &field.name;
    let column_name = field.column_name();
    let column: Expr = parse_quote!(#table_name::#column_name);

    if treat_none_as_default_value {
        if is_option_ty(&field.ty) {
            if lifetime.is_some() {
                quote!(self.#field_name.as_ref().map(|x| #column.eq(x)))
            } else {
                quote!(self.#field_name.map(|x| #column.eq(x)))
            }
        } else {
            quote!(std::option::Option::Some(#column.eq(#lifetime self.#field_name)))
        }
    } else {
        quote!(#column.eq(#lifetime self.#field_name))
    }
}
