use proc_macro2::Span;

pub trait ResolvedAtExt {
    fn resolved_at(self, span: Span) -> Span;
}

#[cfg(feature = "nightly")]
impl ResolvedAtExt for Span {
    fn resolved_at(self, span: Span) -> Span {
        self.unstable().resolved_at(span.unstable()).into()
    }
}

#[cfg(not(feature = "nightly"))]
impl ResolvedAtExt for Span {
    fn resolved_at(self, _: Span) -> Span {
        self
    }
}
