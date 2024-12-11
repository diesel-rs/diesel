//! SQLite specific functions
use crate::expression::functions::define_sql_function;
use crate::sql_types::*;
use crate::sqlite::expression::expression_methods::JsonOrNullableJsonOrJsonbOrNullableJsonb;
use crate::sqlite::expression::expression_methods::MaybeNullableValue;

#[cfg(feature = "sqlite")]
define_sql_function! {
    /// Verifies that its argument is a valid JSON string or JSONB blob and returns a minified
    /// version of that JSON string with all unnecessary whitespace removed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     #[cfg(feature = "serde_json")]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::dsl::json;
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    /// 
    /// let result = diesel::select(json::<Json, _>(json!({"a": "b", "c": 1})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{"a":"b","c":1}"#, result);
    ///
    /// let result = diesel::select(json::<Json, _>(json!({ "this" : "is", "a": [ "test" ] })))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{"a":["test"],"this":"is"}"#, result);
    ///
    /// let result = diesel::select(json::<Nullable<Json>, _>(None::<Value>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// let result = diesel::select(json::<Jsonb, _>(json!({"a": "b", "c": 1})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{"a":"b","c":1}"#, result);
    ///
    /// let result = diesel::select(json::<Jsonb, _>(json!({ "this" : "is", "a": [ "test" ] })))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{"a":["test"],"this":"is"}"#, result);
    ///
    /// let result = diesel::select(json::<Nullable<Jsonb>, _>(None::<Value>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn json<E: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue + MaybeNullableValue<Text>>(e: E) -> E::Out;
}

#[cfg(feature = "sqlite")]
define_sql_function! {
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     #[cfg(feature = "serde_json")]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::dsl::jsonb;
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    ///
    /// let result = diesel::select(jsonb::<Json, _>(json!({"a": "b", "c": 1})))
    ///     .get_result::<Value>(connection)?;
    ///
    /// assert_eq!(json!({"a": "b", "c": 1}), result);
    /// println!("json abc1");
    ///
    /// let result = diesel::select(jsonb::<Jsonb, _>(json!({"a": "b", "c": 1})))
    ///     .get_result::<Value>(connection)?;
    ///
    /// assert_eq!(json!({"a": "b", "c": 1}), result);
    /// println!("jsonb abc1");
    ///
    /// let result = diesel::select(jsonb::<Nullable<Json>, _>(None::<Value>))
    ///     .get_result::<Option<Value>>(connection)?;
    ///
    /// assert!(result.is_none());
    /// println!("json null");
    /// 
    /// let result = diesel::select(jsonb::<Nullable<Jsonb>, _>(None::<Value>))
    ///     .get_result::<Option<Value>>(connection)?;
    ///
    /// assert!(result.is_none());
    /// println!("jsonb null");
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn jsonb<E: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue + MaybeNullableValue<Jsonb>>(e: E) -> E::Out;
}
