use super::Mysql;
use backend::BinaryRawValue;

/// Raw mysql value as received from the database
#[derive(Copy, Clone, Debug)]
pub struct MysqlValue<'a> {
    raw: &'a [u8],
}

impl<'a> MysqlValue<'a> {
    pub(crate) fn new(raw: &'a [u8]) -> Self {
        Self { raw }
    }

    /// Get the underlying raw byte representation
    pub fn as_bytes(&self) -> &[u8] {
        self.raw
    }
}

impl<'a> BinaryRawValue<'a> for Mysql {
    fn as_bytes(value: Self::RawValue) -> &'a [u8] {
        value.raw
    }
}
