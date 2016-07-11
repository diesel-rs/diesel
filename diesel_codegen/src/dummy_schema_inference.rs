use syntax::codemap::Span;
use syntax::ext::base::*;
use syntax::tokenstream;

pub fn expand_load_table<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    _tts: &[tokenstream::TokenTree]
) -> Box<MacResult+'cx> {
    cx.span_warn(sp, "load_table_from_schema! is only supported on PostgreSQL");
    DummyResult::any(sp)
}

pub fn expand_infer_schema<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    _tts: &[tokenstream::TokenTree]
) -> Box<MacResult+'cx> {
    cx.span_warn(sp, "infer_schema! is only supported on PostgreSQL");
    DummyResult::any(sp)
}
