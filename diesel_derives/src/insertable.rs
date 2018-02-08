use syn;
use quote;
use proc_macro2::Span;

use field::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<quote::Tokens, Diagnostic> {
    let model = Model::from_item(&item)?;

    if model.fields().is_empty() {
        return Err(Span::call_site()
            .error("Cannot derive Insertable for unit structs")
            .help(format!(
                "Use `insert_into({}::table).default_values()` if you want `DEFAULT VALUES`",
                model.table_name()
            )));
    }

    let table_name = model.table_name();
    let struct_name = item.ident;

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut ref_generics = item.generics.clone();
    ref_generics.params.push(parse_quote!('insert));
    let (ref_generics, ..) = ref_generics.split_for_impl();

    let (field_ty, field_assign): (Vec<_>, Vec<_>) = model
        .fields()
        .iter()
        .map(|f| (field_ty(f, table_name), field_expr(f, table_name)))
        .unzip();

    Ok(wrap_in_dummy_mod(
        model.dummy_mod_name("insertable"),
        quote! {
            use self::diesel::insertable::Insertable;
            use self::diesel::query_builder::UndecoratedInsertRecord;
            use self::diesel::prelude::*;

            impl #ref_generics Insertable<#table_name::table>
                for &'insert #struct_name #ty_generics
            #where_clause
            {
                type Values = <(#(#field_ty,)*) as Insertable<#table_name::table>>::Values;

                fn values(self) -> Self::Values {
                    (#(#field_assign,)*).values()
                }
            }

            impl #impl_generics UndecoratedInsertRecord<#table_name::table>
                for #struct_name #ty_generics
            #where_clause
            {
            }
        },
    ))
}

fn field_ty(field: &Field, table_name: syn::Ident) -> syn::Type {
    if field.has_flag("embed") {
        let field_ty = &field.ty;
        parse_quote!(&'insert #field_ty)
    } else {
        let inner_ty = inner_of_option_ty(&field.ty);
        let column_name = field.column_name();
        parse_quote!(
            std::option::Option<diesel::dsl::Eq<
                #table_name::#column_name,
                &'insert #inner_ty,
            >>
        )
    }
}

fn field_expr(field: &Field, table_name: syn::Ident) -> syn::Expr {
    let field_access = field.name.access();
    if field.has_flag("embed") {
        parse_quote!(&self#field_access)
    } else {
        let column_name = field.column_name();
        let column: syn::Expr = parse_quote!(#table_name::#column_name);
        if is_option_ty(&field.ty) {
            parse_quote!(self#field_access.as_ref().map(|x| #column.eq(x)))
        } else {
            parse_quote!(std::option::Option::Some(#column.eq(&self#field_access)))
        }
    }
}
