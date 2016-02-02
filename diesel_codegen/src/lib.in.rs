mod associations;
mod attr;
mod insertable;
mod model;
mod queryable;
#[cfg(feature = "postgres")]
mod schema_inference;
#[cfg(not(feature = "postgres"))]
#[path="dummy_schema_inference.rs"]
mod schema_inference;

mod update;
