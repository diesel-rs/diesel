use proc_macro2;
use proc_macro2::*;
use syn;

use field::FieldName;
use model::Model;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = &item.ident;

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let row = model
        .fields()
        .iter()
        .map(|f| {
            let ty = f.ty_for_deserialize()?;
            if let FieldName::Named(ref name) = f.name {
                let chars = name
                    .to_string()
                    .chars()
                    .map(|c| match c {
                        c @ 'a'..='z' | c @ 'A'..='Z' => c.to_string(),
                        c @ '0'..='9' | c @ '_' => format!("_{}", c),
                        _ => panic!("Unsupported name"),
                    })
                    .collect::<Vec<_>>();
                let chars = chars
                    .iter()
                    .map(|c| Ident::new(c, Span::call_site()))
                    .map(|c| quote!(diesel::frunk::labelled::chars::#c));
                Ok(quote!(diesel::deserialize::Field<(#(#chars,)*), #ty>))
            } else {
                panic!("Only named fields are supported")
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let field_assign = model
        .fields()
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let i = syn::Index::from(i);
            f.name.assign(parse_quote!(row.#i.value))
        })
        .collect::<Vec<_>>();

    let dummy_name = format!("_impl_named_queryable_for_{}", item.ident);
    Ok(wrap_in_dummy_mod(
        Ident::new(&dummy_name.to_lowercase(), Span::call_site()),
        quote! {
            impl#impl_generics diesel::deserialize::NamedQueryable for #struct_name #ty_generics
                #where_clause
            {
                type Row = (#(#row,)*);

                fn build(row: Self::Row) -> Self {
                    Self {
                        #(#field_assign,)*
                    }
                }
            }
        },
    ))
}
