/// Errors produced by `diesel-dynamic-schema`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DynamicSchemaError {
    /// Named dynamic loading received an unnamed field.
    #[error("dynamic output field has no name")]
    UnnamedField,
    /// Query metadata and row metadata reported different field counts.
    #[error("dynamic output metadata has {metadata} fields but the row has {row}")]
    OutputFieldCountMismatch {
        /// The number of fields reported by query metadata.
        metadata: usize,
        /// The number of fields reported by the row.
        row: usize,
    },
}
