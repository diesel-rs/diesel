use proc_macro2;
use proc_macro2::Span;
use syn;
use syn::fold::Fold;
use syn::spanned::Spanned;

use diagnostic_shim::*;
use meta::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;
    let tokens = MetaItem::all_with_name(&item.attrs, "belongs_to")
        .into_iter()
        .filter_map(
            |attr| match derive_belongs_to(&model, &item.generics, attr) {
                Ok(t) => Some(t),
                Err(e) => {
                    e.emit();
                    None
                }
            },
        );

    Ok(wrap_in_dummy_mod(quote!(#(#tokens)*)))
}

fn derive_belongs_to(
    model: &Model,
    generics: &syn::Generics,
    meta: MetaItem,
) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let AssociationOptions {
        parent_struct,
        foreign_key,
    } = AssociationOptions::from_meta(meta)?;
    let (_, ty_generics, _) = generics.split_for_impl();

    let foreign_key_field = model.find_column(&foreign_key)?;
    let struct_name = &model.name;
    let foreign_key_access = foreign_key_field.name.access();
    let foreign_key_ty = &foreign_key_field.ty;
    let table_name = model.table_name();

    let mut generics = generics.clone();

    let parent_struct = ReplacePathLifetimes::new(|i, span| {
        let letter = char::from(b'b' + i as u8);
        let lifetime = syn::Lifetime::new(&format!("'__{}", letter), span);
        generics.params.push(parse_quote!(#lifetime));
        lifetime
    })
    .fold_type_path(parent_struct);

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

    let foreign_key_expr = quote!(std::convert::Into::into(&self#foreign_key_access));
    let foreign_key_ty = quote!(__FK);

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(quote! {
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
    })
}

struct AssociationOptions {
    parent_struct: syn::TypePath,
    foreign_key: syn::Ident,
}

impl AssociationOptions {
    fn from_meta(meta: MetaItem) -> Result<Self, Diagnostic> {
        let parent_struct = meta
            .nested()?
            .find(|m| m.path().is_ok() || m.name().is_ident("parent"))
            .ok_or_else(|| meta.span())
            .and_then(|m| {
                m.path()
                    .map(|i| parse_quote!(#i))
                    .or_else(|_| m.ty_value())
                    .map_err(|_| m.span())
            })
            .and_then(|ty| match ty {
                syn::Type::Path(ty_path) => Ok(ty_path),
                _ => Err(ty.span()),
            })
            .map_err(|span| {
                span.error("Expected a struct name")
                    .help("e.g. `#[belongs_to(User)]` or `#[belongs_to(parent = \"User<'_>\")]")
            })?;
        let foreign_key = {
            let parent_struct_name = parent_struct
                .path
                .segments
                .last()
                .expect("paths always have at least one segment");
            meta.nested_item("foreign_key")?
                .map(|i| i.ident_value())
                .unwrap_or_else(|| Ok(infer_foreign_key(&parent_struct_name.ident)))?
        };

        let (unrecognized_paths, unrecognized_options): (Vec<_>, Vec<_>) = meta
            .nested()?
            .skip(1)
            .filter(|n| !n.name().is_ident("foreign_key"))
            .partition(|item| item.path().is_ok());

        if !unrecognized_paths.is_empty() {
            let parent_path_string = path_to_string(&parent_struct.path);
            let unrecognized_path_strings: Vec<_> = unrecognized_paths
                .iter()
                .filter_map(|item| item.path().as_ref().map(path_to_string).ok())
                .collect();

            meta.span()
                .warning(format!(
                    "belongs_to takes a single parent. Change\n\
                     \tbelongs_to({}, {})\n\
                     to\n\
                     \tbelongs_to({})\n\
                     {}",
                    parent_path_string,
                    unrecognized_path_strings.join(","),
                    parent_path_string,
                    unrecognized_path_strings
                        .iter()
                        .map(|path| format!("\tbelongs_to({})", path))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
                .emit();
        }

        for ignored in unrecognized_options {
            ignored
                .span()
                .warning(format!(
                    "Unrecognized option {}",
                    path_to_string(&ignored.name())
                ))
                .emit();
        }

        Ok(Self {
            parent_struct,
            foreign_key,
        })
    }
}

fn infer_foreign_key(name: &syn::Ident) -> syn::Ident {
    let snake_case = camel_to_snake(&name.to_string());
    syn::Ident::new(&format!("{}_id", snake_case), name.span())
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
    F: FnMut(usize, Span) -> syn::Lifetime,
{
    fn fold_lifetime(&mut self, mut lt: syn::Lifetime) -> syn::Lifetime {
        if lt.ident == "_" {
            lt = (self.f)(self.count, lt.span());
            self.count += 1;
        }
        lt
    }
}
