use quote::quote;
use syn::Token;
use syn::{punctuated::Punctuated, DeriveInput};

pub(crate) fn expand(cfg: CfgInput, item: EntryWithVisibility) -> proc_macro2::TokenStream {
    item.hide_for_cfg(cfg.cfg, cfg.field_list)
}

pub struct CfgInput {
    cfg: syn::Meta,
    field_list: Vec<syn::Ident>,
}

impl syn::parse::Parse for CfgInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut cfg = Punctuated::<syn::Meta, Token![,]>::parse_terminated(input)?;
        if cfg.len() == 1 {
            Ok(Self {
                cfg: cfg
                    .pop()
                    .expect("There is exactly one element")
                    .into_value(),
                field_list: Vec::new(),
            })
        } else if cfg.len() == 2 {
            let value_1 = cfg
                .pop()
                .expect("There is exactly one element")
                .into_value();
            let value_2 = cfg
                .pop()
                .expect("There is exactly one element")
                .into_value();
            let (cfg, fields) = if matches!(&value_1, syn::Meta::List(v) if v.path.is_ident("public_fields"))
            {
                (value_2, value_1)
            } else if matches!(&value_2, syn::Meta::List(v) if v.path.is_ident("public_fields")) {
                (value_1, value_2)
            } else {
                panic!(
                    "Incompatible argument list detected. `__diesel_public_if` \
                     expects a cfg argument and a optional public_fields"
                )
            };
            let field_list = if let syn::Meta::List(v) = fields {
                use syn::parse::Parser;
                let parser =
                    syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated;
                let idents = parser.parse2(v.tokens)?;
                idents.into_iter().collect()
            } else {
                unreachable!()
            };
            Ok(Self { cfg, field_list })
        } else {
            panic!(
                "Incompatible argument list detected. `__diesel_public_if` \
                 expects a cfg argument and an optional public_fields"
            )
        }
    }
}

#[derive(Clone)]
pub enum EntryWithVisibility {
    TraitFunction {
        meta: Vec<syn::Attribute>,
        tail: proc_macro2::TokenStream,
    },
    Item {
        meta: Vec<syn::Attribute>,
        vis: syn::Visibility,
        tail: proc_macro2::TokenStream,
    },
    Struct {
        meta: Vec<syn::Attribute>,
        vis: syn::Visibility,
        def: syn::DataStruct,
        ident: syn::Ident,
        generics: syn::Generics,
    },
}

impl syn::parse::Parse for EntryWithVisibility {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let meta = syn::Attribute::parse_outer(input)?;
        if input.peek(Token![fn]) || input.peek(Token![type]) {
            let tail = input.parse()?;
            Ok(Self::TraitFunction { meta, tail })
        } else {
            let vis = input.parse()?;
            if input.peek(Token![struct]) {
                let s = DeriveInput::parse(input)?;
                if let syn::Data::Struct(def) = s.data {
                    Ok(Self::Struct {
                        meta,
                        vis,
                        def,
                        generics: s.generics,
                        ident: s.ident,
                    })
                } else {
                    unreachable!()
                }
            } else {
                let tail = input.parse()?;
                Ok(Self::Item { meta, vis, tail })
            }
        }
    }
}

impl EntryWithVisibility {
    fn hide_for_cfg(
        &self,
        cfg: syn::Meta,
        field_list: Vec<syn::Ident>,
    ) -> proc_macro2::TokenStream {
        match self {
            EntryWithVisibility::TraitFunction { meta, tail } if field_list.is_empty() => quote! {
                #(#meta)*
                #[cfg_attr(not(#cfg), doc(hidden))]
                #[cfg_attr(docsrs, doc(cfg(#cfg)))]
                #tail
            },
            EntryWithVisibility::Item { meta, vis, tail } if field_list.is_empty() => {
                quote! {
                    #(#meta)*
                    #[cfg(not(#cfg))]
                    #vis #tail

                    #(#meta)*
                    #[cfg(#cfg)]
                    pub #tail
                }
            }
            EntryWithVisibility::Struct {
                meta,
                vis,
                def,
                ident,
                generics,
            } => {
                let fields1 = def.fields.iter();
                let fields2 = def.fields.iter().map(|f| {
                    let mut ret = f.clone();
                    if ret
                        .ident
                        .as_ref()
                        .map(|i| field_list.contains(i))
                        .unwrap_or(false)
                    {
                        ret.vis = syn::Visibility::Public(Default::default());
                    }
                    ret
                });

                quote! {
                    #(#meta)*
                    #[cfg(not(#cfg))]
                    #vis struct #ident #generics {
                        #(#fields1,)*
                    }

                    #(#meta)*
                    #[cfg(#cfg)]
                    #[non_exhaustive]
                    pub struct #ident #generics {
                        #(#fields2,)*
                    }
                }
            }
            EntryWithVisibility::TraitFunction { .. } | EntryWithVisibility::Item { .. } => {
                panic!("Public field list is only supported for structs")
            }
        }
    }
}
