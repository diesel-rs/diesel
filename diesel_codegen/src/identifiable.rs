use quote::Tokens;
use syn;

use model::Model;
use util::wrap_item_in_const;

pub fn derive_identifiable(item: syn::MacroInput) -> Tokens {
    let model = t!(Model::from_item(&item, "Identifiable"));

    let has_table = impl_has_table(&model);
    let identifiable = impl_identifiable(&model);

    let model_name_uppercase = model.name.as_ref().to_uppercase();
    let dummy_const = format!("_IMPL_IDENTIFIABLE_FOR_{}", model_name_uppercase).into();

    wrap_item_in_const(dummy_const, quote!(
        #has_table
        #identifiable
    ))
}

fn impl_has_table(model: &Model) -> Tokens {
    let generics = &model.generics;
    let struct_ty = &model.ty;
    let table_name = model.table_name();

    quote!(
        impl#generics diesel::associations::HasTable for #struct_ty {
            type Table = #table_name::table;

            fn table() -> Self::Table {
                #table_name::table
            }
        }
    )
}

fn impl_identifiable(model: &Model) -> Tokens {
    let ident = syn::LifetimeDef::new("'ident");
    let generics = syn::aster::from_generics(model.generics.clone())
        .with_lifetime(ident.clone())
        .build();

    let struct_ty = &model.ty;
    let primary_keys = &model.primary_key_names;

    let primary_key_fields = model.attrs.as_slice().iter().cloned().filter(|f| {
        if let Some(ref name) = f.column_name {
            if primary_keys.contains(name) {
                return true;
            }
        }
        if let Some(ref name) = f.field_name {
            if primary_keys.contains(name) {
                return true;
            }
        }
        false
    }).collect::<Vec<_>>();

    let mut id_ty = primary_key_fields.iter().map(|a| {
        let ty = &a.ty;
        quote!(&#ident #ty)
    });
    let id_ty = if primary_key_fields.len() == 1 {
        id_ty.next().expect("It's there because there is one primary key")
    } else {
        quote!((#(#id_ty,)*))
    };

    let mut id_ret = primary_key_fields.iter().enumerate().map(|(i, a)|{
        let name = a.field_name.clone().unwrap_or_else(|| syn::Ident::from(i));
        quote!(&self.#name)
    });
    let id_ret = if primary_key_fields.len() == 1 {
        id_ret.next().expect("It's there because there is one primary key")
    } else {
        quote!((#(#id_ret,)*))
    };

    quote!(
        impl#generics diesel::associations::Identifiable for &#ident #struct_ty {
            type Id = #id_ty;

            fn id(self) -> Self::Id {
                #id_ret
            }
        }
    )
}
