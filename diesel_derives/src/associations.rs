use proc_macro2::{Span, TokenStream};
use syn::fold::Fold;
use syn::{DeriveInput, Ident, Lifetime};

use model::Model;
use parsers::BelongsTo;
use util::{camel_to_snake, wrap_in_dummy_mod};

pub fn derive(item: DeriveInput) -> TokenStream {
    let model = Model::from_item(&item, false);

    if model.belongs_to.is_empty() {
        abort_call_site!(
            "At least one `belongs_to` is needed for deriving `Associations` on a structure."
        );
    }

    let tokens = model
        .belongs_to
        .iter()
        .map(|assoc| derive_belongs_to(&item, &model, assoc));

    wrap_in_dummy_mod(quote!(#(#tokens)*))
}

fn derive_belongs_to(item: &DeriveInput, model: &Model, assoc: &BelongsTo) -> TokenStream {
    let (_, ty_generics, _) = item.generics.split_for_impl();

    let struct_name = &item.ident;
    let table_name = model.table_name();

    let foreign_key = &foreign_key(assoc);

    let foreign_key_field = model.find_column(foreign_key);
    let foreign_key_name = &foreign_key_field.name;
    let foreign_key_ty = &foreign_key_field.ty;

    let mut generics = item.generics.clone();

    let parent_struct = ReplacePathLifetimes::new(|i, span| {
        let letter = char::from(b'b' + i as u8);
        let lifetime = Lifetime::new(&format!("'__{}", letter), span);
        generics.params.push(parse_quote!(#lifetime));
        lifetime
    })
    .fold_type_path(assoc.parent.clone());

    generics.params.push(parse_quote!(__FK));
    {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!(__FK: std::hash::Hash + std::cmp::Eq));
        where_clause.predicates.push(
                parse_quote!(for<'__a> &'__a #foreign_key_ty: std::convert::Into<::std::option::Option<&'__a __FK>>),
            );
        where_clause.predicates.push(
                parse_quote!(for<'__a> &'__a #parent_struct: diesel::associations::Identifiable<Id = &'__a __FK>),
            );
    }

    let foreign_key_expr = quote!(std::convert::Into::into(&self.#foreign_key_name));
    let foreign_key_ty = quote!(__FK);

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics diesel::associations::BelongsTo<#parent_struct>
            for #struct_name #ty_generics
        #where_clause
        {
            type ForeignKey = #foreign_key_ty;
            type ForeignKeyColumn = #table_name::#foreign_key;

            fn foreign_key(&self) -> std::option::Option<&Self::ForeignKey> {
                #foreign_key_expr
            }

            fn foreign_key_column() -> Self::ForeignKeyColumn {
                #table_name::#foreign_key
            }
        }
    }
}

fn foreign_key(assoc: &BelongsTo) -> Ident {
    let ident = &assoc
        .parent
        .path
        .segments
        .last()
        .expect("paths always have at least one segment")
        .ident;

    assoc
        .foreign_key
        .clone()
        .unwrap_or_else(|| infer_foreign_key(ident))
}

fn infer_foreign_key(name: &Ident) -> Ident {
    let snake_case = camel_to_snake(&name.to_string());
    Ident::new(&format!("{}_id", snake_case), name.span())
}

struct ReplacePathLifetimes<F> {
    count: usize,
    f: F,
}

impl<F> ReplacePathLifetimes<F> {
    fn new(f: F) -> Self {
        Self { count: 0, f }
    }
}

impl<F> Fold for ReplacePathLifetimes<F>
where
    F: FnMut(usize, Span) -> Lifetime,
{
    fn fold_lifetime(&mut self, mut lt: Lifetime) -> Lifetime {
        if lt.ident == "_" {
            lt = (self.f)(self.count, lt.span());
            self.count += 1;
        }
        lt
    }
}
