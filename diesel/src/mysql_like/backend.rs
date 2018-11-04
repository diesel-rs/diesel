//! The MySQL backend
use backend::Backend;
use sql_types::TypeMetadata;

pub trait MysqlLikeBackend: Backend + TypeMetadata {}
