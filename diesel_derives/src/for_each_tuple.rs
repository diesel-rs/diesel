use std::ops::Range;

use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{Attribute, Ident, Type, TypeParam};

#[cfg(feature = "128-column-tables")]
pub const DEFAULT_SIZE: i32 = 128;

#[cfg(all(feature = "64-column-tables", not(feature = "128-column-tables")))]
pub const DEFAULT_SIZE: i32 = 64;

#[cfg(all(
    feature = "32-column-tables",
    not(feature = "128-column-tables"),
    not(feature = "64-column-tables")
))]
pub const DEFAULT_SIZE: i32 = 32;

#[cfg(all(
    not(feature = "64-column-tables"),
    not(feature = "128-column-tables"),
    not(feature = "32-column-tables")
))]
pub const DEFAULT_SIZE: i32 = 16;

macro_rules! timed_block {
    ($start: expr, $text: expr, $debug: expr) => {
        if $debug {
            let elapsed = $start.elapsed();
            eprintln!(
                "    {}: {}.{:06} s",
                $text,
                elapsed.as_secs(),
                elapsed.subsec_micros()
            );
        }
    };
}

macro_rules! start_timed_block {
    ($name: expr, $debug: expr) => {{
        if $debug {
            eprintln!("    {}", $name);
        }
        std::time::Instant::now()
    }};
}

pub struct ArgWrapper {
    pub start_index: i32,
    pub debug: bool,
    pub timing: bool,
}

impl Parse for ArgWrapper {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let time = start_timed_block!("Parse arguments", false);
        let mut index_start = 0;
        let mut debug = false;
        let mut timing = false;
        let metas =
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(input)?;

        for meta in metas {
            if let syn::Meta::NameValue(value) = meta {
                if value.path == parse_quote!(index_start) {
                    if let syn::Lit::Int(ref lit) = value.lit {
                        index_start = lit.base10_parse()?;
                    }
                }
                if value.path == parse_quote!(debug) {
                    if let syn::Lit::Bool(ref lit) = value.lit {
                        debug = lit.value;
                    }
                }
                if value.path == parse_quote!(timing) {
                    if let syn::Lit::Bool(ref lit) = value.lit {
                        timing = lit.value;
                    }
                }
            }
        }
        timed_block!(time, "Parsing args", timing);

        Ok(Self {
            start_index: index_start,
            debug,
            timing,
        })
    }
}

pub struct Repetition {
    item_impl: syn::Item,
    repeated_types: Vec<TypeParam>,
}

fn get_repeated_types(generics: &mut syn::Generics) -> Vec<TypeParam> {
    let repeat_attr: Attribute = parse_quote!(#[repeat]);
    let repeated_generics = generics
        .type_params()
        .filter(|p| p.attrs.iter().any(|a| a == &repeat_attr))
        .cloned()
        .map(|mut p| {
            p.attrs = Vec::new();
            p
        })
        .collect::<Vec<_>>();

    let generic_params = std::mem::replace(&mut generics.params, Default::default());

    generics.params = generic_params
        .into_iter()
        .filter(|a| match a {
            syn::GenericParam::Type(t) => t.attrs.iter().all(|a| a != &repeat_attr),
            syn::GenericParam::Lifetime(_) => true,
            syn::GenericParam::Const(_) => true,
        })
        .collect();
    repeated_generics
}

impl Parse for Repetition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let time = start_timed_block!("Parse repetition", false);
        let mut item: syn::Item = input.parse()?;

        let repeated_generics = match item {
            syn::Item::Impl(ref mut i) => get_repeated_types(&mut i.generics),
            syn::Item::Struct(ref mut s) => get_repeated_types(&mut s.generics),
            _ => Vec::new(),
        };
        timed_block!(time, "Parsing repetition", false);

        Ok(Self {
            item_impl: item,
            repeated_types: repeated_generics,
        })
    }
}

impl Repetition {
    pub fn repeat(
        &self,
        index_start: i32,
        tuple_size: i32,
        timing: bool,
        debug: bool,
    ) -> syn::Item {
        let time = start_timed_block!(format!("Repetition {}", tuple_size), debug);
        use proc_macro2::Span;
        use quote::ToTokens;
        use syn::visit_mut::VisitMut;

        let mut tokens = proc_macro2::TokenStream::new();

        let meta = syn::NestedMeta::Meta(syn::Meta::Path(path_from_ident(Ident::new(
            "non_camel_case_types",
            Span::call_site(),
        ))));

        syn::token::Paren::default().surround(&mut tokens, |tokens| meta.to_tokens(tokens));

        let attr = syn::Attribute {
            pound_token: Default::default(),
            style: syn::AttrStyle::Outer,
            bracket_token: Default::default(),
            path: path_from_ident(Ident::new("allow", Span::call_site())),
            tokens,
        };

        let mut ret = self.item_impl.clone();
        ConcreteRepetition {
            repeated_types: self.repeated_types.clone(),
            index_range: index_start..tuple_size + index_start,
            timing,
            debug,
        }
        .visit_item_mut(&mut ret);

        match ret {
            syn::Item::Struct(ref mut s) => {
                s.ident = Ident::new(&format!("{}_{}", s.ident, tuple_size), s.span());
                s.attrs.push(attr);
            }
            syn::Item::Impl(ref mut s) => {
                s.attrs.push(attr);
            }
            _ => {}
        }

        timed_block!(time, format!("Repetition {}", tuple_size), timing);

        ret
    }
}

#[allow(dead_code)]
struct ConcreteRepetition {
    repeated_types: Vec<TypeParam>,
    index_range: Range<i32>,
    timing: bool,
    debug: bool,
}

impl ConcreteRepetition {
    fn type_contains_repeated_tuple(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Tuple(t) => {
                t.elems.len() == 1
                    && t.elems
                        .iter()
                        .next()
                        .map(|t| {
                            if let syn::Type::Path(syn::TypePath { qself: None, path }) = t {
                                path.segments.len() == 1
                                    && path
                                        .segments
                                        .last()
                                        .map(|seg| {
                                            self.repeated_types.iter().any(|r| r.ident == seg.ident)
                                        })
                                        .unwrap_or(false)
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false)
            }
            syn::Type::Path(syn::TypePath { qself, path }) => {
                if let Some(qself) = qself {
                    if self.type_contains_repeated_tuple(&*qself.ty) {
                        return true;
                    }
                }
                path.segments
                    .iter()
                    .any(|seg| self.path_contains_repeated_tuple(seg))
            }
            _ => false,
        }
    }

    fn path_contains_repeated_tuple(&self, seg: &syn::PathSegment) -> bool {
        if let syn::PathArguments::AngleBracketed(ref args) = seg.arguments {
            args.args.iter().any(|arg| match arg {
                syn::GenericArgument::Type(t) => self.type_contains_repeated_tuple(t),
                _ => false,
            })
        } else {
            false
        }
    }

    fn bound_contains_repeated_tuple(&self, bound: &syn::TypeParamBound) -> bool {
        match bound {
            syn::TypeParamBound::Trait(bound) => bound
                .path
                .segments
                .iter()
                .any(|seg| self.path_contains_repeated_tuple(seg)),
            syn::TypeParamBound::Lifetime(_) => false,
        }
    }

    fn expand_predicate(&self, p: syn::WherePredicate) -> Vec<syn::WherePredicate> {
        let time = start_timed_block!("expand_predicate", self.timing);
        // This check is a workaround for
        // https://github.com/rust-lang/rust/issues/75542
        // It can likely be removed as soon as someone
        // fixes the issue in upstream rustc
        let ret = if let syn::WherePredicate::Type(ref t) = p {
            if let syn::Type::Path(syn::TypePath {
                qself: None,
                ref path,
            }) = t.bounded_ty
            {
                if path
                    .segments
                    .first()
                    .map(|p| {
                        let ty = p.ident.to_string();
                        self.repeated_types.iter().any(|t| {
                            t.ident.to_string() != ty && ty.starts_with(&t.ident.to_string())
                        })
                    })
                    .unwrap_or(false)
                {
                    if t.bounds
                        .iter()
                        .any(|b| self.bound_contains_repeated_tuple(b))
                    {
                        timed_block!(
                            time,
                            "expand_predicate (postive recursive check",
                            self.timing
                        );
                        let ret = self.expand_recursive_predicates(p);
                        timed_block!(time, "expand_recursive_predicates", self.timing);
                        return ret;
                    }
                }
            }
            timed_block!(time, "expand_predicate (recursive check)", self.timing);
            self.repeated_predicate(p)
        } else {
            timed_block!(time, "expand_predicate (recursive check)", self.timing);
            self.repeated_predicate(p)
        };
        timed_block!(time, "expand_predicate", self.timing);
        ret
    }

    fn replace_tuple_in_type_with_ty(&self, ty: &mut syn::Type, idx: i32) {
        let span = ty.span();
        let mut ret_ty = None;
        match ty {
            syn::Type::Tuple(t) if t.elems.len() == 1 => {
                let first = t.elems.first_mut().unwrap();
                if let syn::Type::Path(syn::TypePath {
                    qself: None,
                    ref mut path,
                }) = first
                {
                    if path.segments.len() == 1 {
                        let last = path.segments.last_mut().unwrap();
                        if let Some(ty_param) =
                            self.repeated_types.iter().find(|t| t.ident == last.ident)
                        {
                            let ident = Ident::new(&format!("{}_{}", ty_param.ident, idx), span);
                            ret_ty = Some(ty_path_from_ident(ident));
                        }
                    }
                }
            }
            syn::Type::Path(syn::TypePath {
                ref mut qself,
                ref mut path,
            }) => {
                if let Some(qself) = qself {
                    self.replace_tuple_in_type_with_ty(&mut *qself.ty, idx);
                }
                self.replace_tuple_in_path_with_ty(path, idx);
                return;
            }
            _ => {
                return;
            }
        }
        if let Some(ret_ty) = ret_ty {
            *ty = syn::Type::Path(ret_ty);
        }
    }

    fn replace_tuple_in_path_with_ty(&self, path: &mut syn::Path, idx: i32) {
        for seg in &mut path.segments {
            if let syn::PathArguments::AngleBracketed(ref mut args) = seg.arguments {
                for arg in &mut args.args {
                    if let syn::GenericArgument::Type(ref mut t) = arg {
                        self.replace_tuple_in_type_with_ty(t, idx);
                    }
                }
            }
        }
    }

    fn expand_recursive_predicate(
        &self,
        t: &syn::PredicateType,
        idx: Option<i32>,
    ) -> syn::PredicateType {
        let mut ret = t.clone();
        if let Some(idx) = idx {
            if self.index_range.contains(&idx) {
                if let syn::Type::Path(ref mut ty) = ret.bounded_ty {
                    let first_seg = ty.path.segments.first_mut().unwrap();
                    if let Some(ty) = self.repeated_types.iter().find(|t| {
                        first_seg
                            .ident
                            .to_string()
                            .starts_with(&t.ident.to_string())
                    }) {
                        first_seg.ident =
                            Ident::new(&format!("{}_{}", ty.ident, idx), first_seg.ident.span());
                    }
                }
            }
        }
        ret
    }

    fn expand_recursive_predicates(&self, p: syn::WherePredicate) -> Vec<syn::WherePredicate> {
        let time = start_timed_block!("expand_recursive_predicates", self.timing);
        let mut ret = Vec::with_capacity(self.index_range.len());
        if let syn::WherePredicate::Type(ref t) = p {
            {
                let mut pred = self.expand_recursive_predicate(t, Some(1));
                for bound in &mut pred.bounds {
                    if let syn::TypeParamBound::Trait(ref mut bound) = bound {
                        self.replace_tuple_in_path_with_ty(&mut bound.path, 0);
                    }
                }
                ret.push(syn::WherePredicate::Type(pred));
            }
            if self.index_range.clone().skip(1).len() > 0 {
                timed_block!(
                    time,
                    "expand_recursive_predicate (before loop)",
                    self.timing
                );
                for idx in self.index_range.clone().skip(2).map(Some).chain(vec![None]) {
                    if let Some(syn::WherePredicate::Type(last)) = ret.last() {
                        let mut pred = self.expand_recursive_predicate(t, idx);
                        timed_block!(
                            time,
                            format!(
                                "    expand_recursive_predicate (loop {:?}, after expand)",
                                idx
                            ),
                            self.timing
                        );
                        assert!(last.bounds.len() == 1);
                        assert!(pred.bounds.len() == 1);
                        let ty = &last.bounded_ty;
                        if let Some(syn::TypeParamBound::Trait(bound)) = last.bounds.first() {
                            timed_block!(
                                time,
                                "    expand_recursive_predicate (before clone)",
                                self.timing
                            );
                            let mut path = bound.path.clone();
                            timed_block!(
                                time,
                                "    expand_recursive_predicate (after clone)",
                                self.timing
                            );
                            path.segments.extend(vec![syn::PathSegment {
                                ident: Ident::new("Out", proc_macro2::Span::call_site()),
                                arguments: syn::PathArguments::None,
                            }]);
                            timed_block!(
                                time,
                                "    expand_recursive_predicate (before inner)",
                                self.timing
                            );
                            let inner = syn::TypePath {
                                qself: Some(syn::QSelf {
                                    lt_token: Default::default(),
                                    gt_token: Default::default(),
                                    ty: Box::new(ty.clone()),
                                    position: bound.path.segments.len(),
                                    as_token: Some(Default::default()),
                                }),
                                path,
                            };
                            let inner = syn::GenericArgument::Type(syn::Type::Path(inner));
                            timed_block!(
                                time,
                                "    expand_recursive_predicate (after inner)",
                                self.timing
                            );
                            if let Some(syn::TypeParamBound::Trait(ref mut bound)) =
                                pred.bounds.first_mut()
                            {
                                let last_seg = bound.path.segments.last_mut().unwrap();
                                last_seg.arguments = syn::PathArguments::AngleBracketed(
                                    syn::AngleBracketedGenericArguments {
                                        colon2_token: None,
                                        lt_token: Default::default(),
                                        args: vec![inner].into_iter().collect(),
                                        gt_token: Default::default(),
                                    },
                                );
                                timed_block!(
                                    time,
                                    "    expand_recursive_predicate (before push)",
                                    self.timing
                                );
                                ret.push(syn::WherePredicate::Type(pred));
                                timed_block!(
                                    time,
                                    "    expand_recursive_predicate (after inner)",
                                    self.timing
                                );
                            }
                        }
                    }
                    timed_block!(
                        time,
                        format!("    expand_recursive_predicate (loop {:?})", idx),
                        self.timing
                    );
                }
                timed_block!(time, "expand_recursive_predicate (after loop", self.timing);
            }
        }
        timed_block!(time, "expand_recursive_predicate", self.timing);
        ret
    }

    fn repeated_predicate(&self, p: syn::WherePredicate) -> Vec<syn::WherePredicate> {
        let mut span = None;
        let mut ret = Vec::new();
        let mut should_be_repeated = false;
        for idx in self.index_range.clone() {
            let mut p = p.clone();
            if let syn::WherePredicate::Type(ref mut t) = p {
                if let syn::Type::Path(syn::TypePath {
                    qself: None,
                    ref mut path,
                }) = t.bounded_ty
                {
                    if path.segments.len() == 1 {
                        let last = path.segments.last_mut().unwrap();
                        if let Some(param) =
                            self.repeated_types.iter().find(|t| last.ident == t.ident)
                        {
                            let span = if let Some(span) = span.clone() {
                                span
                            } else {
                                let s = param.span();
                                span = Some(s.clone());
                                s
                            };
                            let ident = Ident::new(&format!("{}_{}", param.ident, idx), span);

                            last.ident = ident;
                            should_be_repeated = true;
                        }
                    }
                }
                for bound in t.bounds.iter_mut() {
                    if let syn::TypeParamBound::Trait(bound) = bound {
                        if let Some(path) = bound.path.segments.last_mut() {
                            self.replace_path_arguments(path, idx, &mut should_be_repeated);
                        }
                    }
                }
            }
            if should_be_repeated {
                ret.push(p);
            } else {
                break;
            }
        }
        if should_be_repeated {
            ret
        } else {
            vec![p]
        }
    }

    fn replace_path_arguments(
        &self,
        path: &mut syn::PathSegment,
        idx: i32,
        should_be_repeated: &mut bool,
    ) {
        if let syn::PathArguments::AngleBracketed(ref mut args) = path.arguments {
            for arg in args.args.iter_mut() {
                match arg {
                    syn::GenericArgument::Type(Type::Path(syn::TypePath {
                        ref mut qself,
                        ref mut path,
                    })) => {
                        for seg in &mut path.segments {
                            self.replace_path_arguments(seg, idx, should_be_repeated);
                        }
                        if path.segments.len() == 1 {
                            let last = path.segments.last_mut().unwrap();
                            if let Some(param) =
                                self.repeated_types.iter().find(|t| last.ident == t.ident)
                            {
                                let ident =
                                    Ident::new(&format!("{}_{}", param.ident, idx), param.span());

                                last.ident = ident;
                                *should_be_repeated = true;
                            }
                        }
                        if let Some(ref mut qself) = qself {
                            if let syn::Type::Path(ref mut tpe) = *qself.ty {
                                if tpe.path.segments.len() == 1 {
                                    let last = tpe.path.segments.last_mut().unwrap();
                                    if let Some(param) =
                                        self.repeated_types.iter().find(|t| last.ident == t.ident)
                                    {
                                        let ident = Ident::new(
                                            &format!("{}_{}", param.ident, idx),
                                            param.span(),
                                        );
                                        last.ident = ident;
                                        *should_be_repeated = true;
                                    }
                                }
                            }
                        }
                    }
                    syn::GenericArgument::Binding(b) => {
                        if let Type::Path(syn::TypePath {
                            qself: None,
                            ref mut path,
                        }) = b.ty
                        {
                            if let Some(path) = path.segments.last_mut() {
                                self.replace_path_arguments(path, idx, should_be_repeated);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn repeat_type<'a>(&'a self, tpe: Type) -> Box<dyn Iterator<Item = Type> + 'a> {
        match tpe {
            Type::Reference(ref_tpe) => {
                Box::new(self.repeat_type((*ref_tpe.elem).clone()).map(move |t| {
                    Type::Reference(syn::TypeReference {
                        elem: Box::new(t),
                        ..ref_tpe.clone()
                    })
                }))
            }
            Type::Path(syn::TypePath { qself, path }) => {
                if let Some(qself) = qself.clone() {
                    if let Type::Path(p) = &(*qself.ty) {
                        if p.path.segments.len() == 1 {
                            let last = p.path.segments.last().unwrap();
                            if let Some(param) = self
                                .repeated_types
                                .iter()
                                .find(|param| param.ident == last.ident)
                            {
                                let span = param.span();
                                return Box::new(self.index_range.clone().map(move |idx| {
                                    let mut qself = qself.clone();
                                    let ident =
                                        Ident::new(&format!("{}_{}", param.ident, idx), span);
                                    qself.ty = Box::new(syn::Type::Path(ty_path_from_ident(ident)));

                                    Type::Path(syn::TypePath {
                                        qself: Some(qself),
                                        path: path.clone(),
                                    })
                                }));
                            }
                        }
                    }
                }
                let mut segment_iter = path.segments.clone().into_iter();

                if let Some(segment) = segment_iter.next() {
                    let other_segments = segment_iter.collect::<Vec<_>>();
                    if let Some(param) = self
                        .repeated_types
                        .iter()
                        .find(|t| t.ident == segment.ident)
                    {
                        let span = param.span();
                        return Box::new(self.index_range.clone().map(move |idx| {
                            let ident = Ident::new(&format!("{}_{}", param.ident, idx), span);
                            let segments = std::iter::once(syn::PathSegment {
                                ident: ident,
                                arguments: syn::PathArguments::None,
                            })
                            .chain(other_segments.clone())
                            .collect();
                            let path = syn::Path {
                                leading_colon: None,
                                segments,
                            };
                            syn::Type::Path(syn::TypePath { qself: None, path })
                        }));
                    }
                }
                Box::new(std::iter::once(Type::Path(syn::TypePath {
                    qself,
                    path: path.clone(),
                })))
            }
            _ => Box::new(std::iter::once(tpe)),
        }
    }
}

impl syn::visit_mut::VisitMut for ConcreteRepetition {
    fn visit_item_struct_mut(&mut self, s: &mut syn::ItemStruct) {
        let time = start_timed_block!("visit_item_struct_mut", self.timing);
        let tuple_args = self.repeated_types.iter().flat_map(|t| {
            let ident = &t.ident;
            let span = t.span();
            self.index_range.clone().map(move |idx| {
                let ident = Ident::new(&format!("{}_{}", ident, idx), span);
                let type_param = TypeParam { ident, ..t.clone() };
                syn::GenericParam::Type(type_param)
            })
        });

        let generic_params = std::mem::replace(&mut s.generics.params, Default::default());

        s.generics.params = generic_params.into_iter().chain(tuple_args).collect();

        syn::visit_mut::visit_item_struct_mut(self, s);
        timed_block!(time, "visit_item_struct_mut", self.timing);
    }

    fn visit_item_impl_mut(&mut self, item_impl: &mut syn::ItemImpl) {
        let time = start_timed_block!("visit_item_impl_mut", self.timing);
        let tuple_args = self.repeated_types.iter().flat_map(|t| {
            let ident = &t.ident;
            let span = t.span();
            self.index_range.clone().map(move |idx| {
                let ident = Ident::new(&format!("{}_{}", ident, idx), span);
                let type_param = TypeParam { ident, ..t.clone() };
                syn::GenericParam::Type(type_param)
            })
        });

        let generic_params = std::mem::replace(&mut item_impl.generics.params, Default::default());

        item_impl.generics.params = generic_params.into_iter().chain(tuple_args).collect();

        syn::visit_mut::visit_item_impl_mut(self, item_impl);
        timed_block!(time, "visit_item_impl_mut", self.timing);
    }

    fn visit_type_tuple_mut(&mut self, t: &mut syn::TypeTuple) {
        let span = t.span();
        let time = start_timed_block!("visit_type_tuple_mut", self.timing);
        let elems = std::mem::replace(&mut t.elems, Default::default());
        t.elems = elems
            .into_iter()
            .flat_map(|elem| self.repeat_type(elem))
            .map(|t| syn::punctuated::Pair::Punctuated(t, syn::Token![,](span)))
            .collect();

        syn::visit_mut::visit_type_tuple_mut(self, t);
        timed_block!(time, "visit_type_tuple_mut", self.timing);
    }

    fn visit_where_clause_mut(&mut self, where_clause: &mut syn::WhereClause) {
        let time = start_timed_block!("visit_where_clause_mut", self.timing);
        where_clause.predicates = where_clause
            .predicates
            .clone()
            .into_iter()
            .flat_map(|p| self.expand_predicate(p))
            .collect();
        timed_block!(
            time,
            "visit_where_clause_mut (before calling into syn",
            self.timing
        );
        syn::visit_mut::visit_where_clause_mut(self, where_clause);
        timed_block!(time, "visit_where_clause_mut", self.timing);
    }

    fn visit_expr_tuple_mut(&mut self, expr: &mut syn::ExprTuple) {
        let time = start_timed_block!("visit_expr_tuple_mut", self.timing);
        let span = expr.span();
        let is_repeat = |a: &syn::Attribute| {
            a.path.segments.len() == 1 && a.path.segments.last().unwrap().ident == "repeat"
        };
        if let Some(_attr) = expr.attrs.iter().find(|a| is_repeat(a)).cloned() {
            let attrs = std::mem::replace(&mut expr.attrs, Vec::new());
            expr.attrs = attrs.into_iter().filter(|a| !is_repeat(a)).collect();
            let elems = std::mem::replace(&mut expr.elems, Default::default());
            expr.elems = elems
                .into_iter()
                .flat_map(|e| {
                    let mut copy = e.clone();
                    ExprReplacer {
                        idx: 0,
                        repeated_types: &self.repeated_types,
                    }
                    .visit_expr_mut(&mut copy);

                    if copy != e {
                        self.index_range
                            .clone()
                            .map(|idx| {
                                let mut elem = e.clone();
                                ExprReplacer {
                                    idx: idx as u32,
                                    repeated_types: &self.repeated_types,
                                }
                                .visit_expr_mut(&mut elem);
                                elem
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![e]
                    }
                })
                .map(|t| syn::punctuated::Pair::Punctuated(t, Token![,](span)))
                .collect();
        }

        syn::visit_mut::visit_expr_tuple_mut(self, expr);
        timed_block!(time, "visit_expr_tuple_mut", self.timing);
    }

    fn visit_expr_block_mut(&mut self, expr: &mut syn::ExprBlock) {
        let time = start_timed_block!("visit_expr_block_mut", self.timing);
        let is_repeat = |a: &syn::Attribute| {
            a.path.segments.len() == 1 && a.path.segments.last().unwrap().ident == "repeat"
        };
        if let Some(_attr) = expr.attrs.iter().find(|a| is_repeat(a)).cloned() {
            let attrs = std::mem::replace(&mut expr.attrs, Vec::new());
            expr.attrs = attrs.into_iter().filter(|a| !is_repeat(a)).collect();

            let repeated_block = self.index_range.clone().map(|idx| {
                let mut block = expr.clone();
                ExprReplacer {
                    idx: idx as u32,
                    repeated_types: &self.repeated_types,
                }
                .visit_block_mut(&mut block.block);

                syn::Stmt::Expr(syn::Expr::Block(block))
            });

            expr.block = syn::Block {
                brace_token: Default::default(),
                stmts: repeated_block.collect(),
            };
        }

        syn::visit_mut::visit_expr_block_mut(self, expr);
        timed_block!(time, "visit_expr_block_mut", self.timing);
    }

    fn visit_impl_item_const_mut(&mut self, const_impl: &mut syn::ImplItemConst) {
        let time = start_timed_block!("visit_impl_item_const_mut", self.timing);
        if let syn::Expr::Binary(syn::ExprBinary {
            left, right, op, ..
        }) = const_impl.expr.clone()
        {
            if let syn::Expr::Path(syn::ExprPath {
                qself: None, path, ..
            }) = *left
            {
                let mut path_iter = path.segments.into_iter();
                if let Some(first) = path_iter.next() {
                    let remaining_segments = path_iter.collect::<Vec<_>>();
                    if let Some(param) = self.repeated_types.iter().find(|t| t.ident == first.ident)
                    {
                        let expr = self.index_range.clone().map(move |idx| {
                            let ident =
                                Ident::new(&format!("{}_{}", param.ident, idx), first.span());
                            let segments = std::iter::once(syn::PathSegment {
                                ident: ident,
                                arguments: syn::PathArguments::None,
                            })
                            .chain(remaining_segments.clone())
                            .collect();
                            syn::Path {
                                leading_colon: None,
                                segments,
                            }
                        });
                        const_impl.expr = parse_quote!(#(#expr #op)* #right);
                    }
                }
            }
        }

        syn::visit_mut::visit_impl_item_const_mut(self, const_impl);
        timed_block!(time, "visit_impl_item_const_mut", self.timing);
    }

    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        let time = start_timed_block!("visit_expr_mut", self.timing);
        if let syn::Expr::Macro(macro_expr) = expr {
            if macro_expr.mac.path.segments.last().unwrap().ident == "tuple_size" {
                let span = expr.span();
                let tuple_size = self.index_range.end;
                *expr = syn::Expr::Lit(syn::ExprLit {
                    attrs: Vec::new(),
                    lit: syn::Lit::Int(syn::LitInt::new(&tuple_size.to_string(), span)),
                });
            }
        }

        syn::visit_mut::visit_expr_mut(self, expr);
        timed_block!(time, "visit_expr_mut", self.timing);
    }
}

struct ExprReplacer<'a> {
    idx: u32,
    repeated_types: &'a [TypeParam],
}

impl<'a> syn::visit_mut::VisitMut for ExprReplacer<'a> {
    fn visit_expr_field_mut(&mut self, i: &mut syn::ExprField) {
        if let syn::Member::Named(n) = i.member.clone() {
            if n == "idx" {
                i.member = syn::Member::Unnamed(syn::Index {
                    index: self.idx,
                    span: i.member.span(),
                });
            }
        }
        syn::visit_mut::visit_expr_field_mut(self, i);
    }

    fn visit_expr_path_mut(&mut self, expr: &mut syn::ExprPath) {
        if let Some(ref mut qself) = expr.qself {
            if let syn::Type::Path(ref mut path) = &mut (*qself.ty) {
                if path.path.segments.len() == 1 {
                    let last = path.path.segments.last_mut().unwrap();
                    if let Some(param) = self.repeated_types.iter().find(|t| last.ident == t.ident)
                    {
                        last.ident =
                            Ident::new(&format!("{}_{}", param.ident, self.idx), last.span());
                    }
                }
            }
        }
        for seg in expr.path.segments.iter_mut() {
            if let syn::PathArguments::AngleBracketed(ref mut args) = seg.arguments {
                for arg in args.args.iter_mut() {
                    match arg {
                        syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                            qself: None,
                            ref mut path,
                        })) => {
                            if path.segments.len() == 1 {
                                let last = path.segments.last_mut().unwrap();
                                if let Some(param) =
                                    self.repeated_types.iter().find(|t| last.ident == t.ident)
                                {
                                    last.ident = Ident::new(
                                        &format!("{}_{}", param.ident, self.idx),
                                        last.span(),
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(first) = expr.path.segments.first_mut() {
            if let Some(param) = self.repeated_types.iter().find(|t| first.ident == t.ident) {
                first.ident = Ident::new(&format!("{}_{}", param.ident, self.idx), first.span());
            }
        }

        syn::visit_mut::visit_expr_path_mut(self, expr);
    }
}

fn ty_path_from_ident(ident: Ident) -> syn::TypePath {
    syn::TypePath {
        qself: None,
        path: path_from_ident(ident),
    }
}

fn path_from_ident(ident: Ident) -> syn::Path {
    syn::Path {
        leading_colon: None,
        segments: vec![syn::PathSegment {
            ident,
            arguments: syn::PathArguments::None,
        }]
        .into_iter()
        .collect(),
    }
}
