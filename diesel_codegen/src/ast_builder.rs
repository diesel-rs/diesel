use syn::{Ident, Ty, Path, PathSegment};

pub fn ty_ident(ident: Ident) -> Ty {
    ty_path(path_ident(ident))
}

pub fn ty_path(path: Path) -> Ty {
    Ty::Path(None, path)
}

pub fn path_ident(ident: Ident) -> Path {
    Path {
        global: false,
        segments: vec![PathSegment::from(ident)],
    }
}
