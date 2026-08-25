use alloc::string::String;

/// Errors produced by `diesel-dynamic-schema`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DynamicSchemaError {
    /// A runtime type has no [`DynamicValue`](crate::dynamic_value::DynamicValue) representation.
    #[error("no `DynamicValue` representation for the {backend} runtime type {sql_type}")]
    UnsupportedType {
        /// The backend that reported the value.
        backend: &'static str,
        /// The runtime type as the backend describes it.
        sql_type: String,
    },
    /// The required rich-type feature is disabled.
    #[error(
        "decoding a {sql_type} value into a `DynamicValue` requires the `{feature}` feature of `diesel-dynamic-schema`"
    )]
    FeatureDisabled {
        /// The crate feature that must be enabled.
        feature: &'static str,
        /// The runtime type that needs the feature.
        sql_type: &'static str,
    },
}
