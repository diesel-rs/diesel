/// Raw sqlite value as received from the database
///
/// Use existing `FromSql` implementations to convert this into
/// rust values
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue<'row, 'stmt, 'query> {
    _v: std::marker::PhantomData<(&'row (), &'stmt (), &'query ())>,
}

