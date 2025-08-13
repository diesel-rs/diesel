//! SQLite specific functions
#[cfg(doc)]
use crate::expression::functions::aggregate_expressions::AggregateExpressionMethods;
use crate::expression::functions::declare_sql_function;
use crate::sql_types::*;
use crate::sqlite::expression::expression_methods::BinaryOrNullableBinary;
use crate::sqlite::expression::expression_methods::JsonOrNullableJson;
use crate::sqlite::expression::expression_methods::JsonOrNullableJsonOrJsonbOrNullableJsonb;
use crate::sqlite::expression::expression_methods::MaybeNullableValue;
use crate::sqlite::expression::expression_methods::NotBlob;
use crate::sqlite::expression::expression_methods::TextOrNullableText;
use crate::sqlite::expression::expression_methods::TextOrNullableTextOrBinaryOrNullableBinary;
use crate::sqlite::expression::functions::helper::CombinedNullableValue;

#[cfg(feature = "sqlite")]
#[declare_sql_function(generate_return_type_helpers = true)]
#[backends(crate::sqlite::Sqlite)]
extern "SQL" {
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
    /// #     use diesel::sql_types::{Text, Nullable};
    /// #     let connection = &mut establish_connection();
    ///
    /// let result = diesel::select(json::<Text, _>(r#"{"a": "b", "c": 1}"#))
    ///     .get_result::<Value>(connection)?;
    ///
    /// assert_eq!(json!({"a":"b","c":1}), result);
    ///
    /// let result = diesel::select(json::<Text, _>(r#"{ "this" : "is", "a": [ "test" ] }"#))
    ///     .get_result::<Value>(connection)?;
    ///
    /// assert_eq!(json!({"a":["test"],"this":"is"}), result);
    ///
    /// let result = diesel::select(json::<Nullable<Text>, _>(None::<&str>))
    ///     .get_result::<Option<Value>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn json<E: TextOrNullableText + MaybeNullableValue<Json>>(e: E) -> E::Out;

    /// The jsonb(X) function returns the binary JSONB representation of the JSON provided as argument X.
    ///
    /// This function requires at least SQLite 3.45 or newer
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
    /// #     use diesel::dsl::{sql, jsonb};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Binary, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 45, 0);
    ///
    /// let result = diesel::select(jsonb::<Binary, _>(br#"{"a": "b", "c": 1}"#))
    ///     .get_result::<Value>(connection)?;
    ///
    /// assert_eq!(json!({"a": "b", "c": 1}), result);
    ///
    /// let result = diesel::select(jsonb::<Binary, _>(br#"{"this":"is","a":["test"]}"#))
    ///     .get_result::<Value>(connection)?;
    ///
    /// assert_eq!(json!({"this":"is","a":["test"]}), result);
    ///
    /// let result = diesel::select(jsonb::<Nullable<Binary>, _>(None::<Vec<u8>>))
    ///     .get_result::<Option<Value>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn jsonb<E: BinaryOrNullableBinary + MaybeNullableValue<Jsonb>>(e: E) -> E::Out;

    /// The json_array_length(X) function returns the number of elements in the JSON array X,
    /// or 0 if X is some kind of JSON value other than an array.
    /// Errors are thrown if either X is not well-formed JSON or if P is not a well-formed path.
    ///
    /// This function requires at least SQLite 3.46 or newer
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
    /// #     use diesel::dsl::{sql, json_array_length};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Json, Jsonb, Text, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 46, 0);
    ///
    /// let result = diesel::select(json_array_length::<Json, _>(json!([1,2,3,4])))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(4, result);
    ///
    /// let result = diesel::select(json_array_length::<Json, _>(json!({"one":[1,2,3]})))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(0, result);
    ///
    /// let result = diesel::select(json_array_length::<Nullable<Json>, _>(None::<Value>))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// let result = diesel::select(json_array_length::<Jsonb, _>(json!([1,2,3,4])))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(4, result);
    ///
    /// let result = diesel::select(json_array_length::<Jsonb, _>(json!({"one":[1,2,3]})))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(0, result);
    ///
    /// let result = diesel::select(json_array_length::<Nullable<Jsonb>, _>(None::<Value>))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn json_array_length<
        J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Integer>,
    >(
        j: J,
    ) -> J::Out;

    /// The json_array_length(X) function returns the number of elements in the JSON array X,
    /// or 0 if X is some kind of JSON value other than an array.
    /// The json_array_length(X,P) locates the array at path P within X and returns the length of that array,
    /// or 0 if path P locates an element in X that is not a JSON array,
    /// and NULL if path P does not locate any element of X.
    /// Errors are thrown if either X is not well-formed JSON or if P is not a well-formed path.
    ///
    /// This function requires at least SQLite 3.46 or newer
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
    /// #     use diesel::dsl::{sql, json_array_length_with_path};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Json, Jsonb, Text, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 46, 0);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Json, _, _>(json!([1,2,3,4]), "$"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(Some(4), result);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Json, _, _>(json!([1,2,3,4]), "$[2]"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(Some(0), result);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Json, _, _>(json!({"one":[1,2,3]}), "$.one"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(Some(3), result);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Nullable<Json>, _, _>(json!({"one":[1,2,3]}), "$.two"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Jsonb, _, _>(json!([1,2,3,4]), "$"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(Some(4), result);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Jsonb, _, _>(json!([1,2,3,4]), "$[2]"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(Some(0), result);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Jsonb, _, _>(json!({"one":[1,2,3]}), "$.one"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(Some(3), result);
    ///
    /// let result = diesel::select(json_array_length_with_path::<Nullable<Jsonb>, _, _>(json!({"one":[1,2,3]}), "$.two"))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[sql_name = "json_array_length"]
    #[cfg(feature = "sqlite")]
    fn json_array_length_with_path<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue>(
        j: J,
        path: Text,
    ) -> Nullable<Integer>;

    /// The json_error_position(X) function returns 0 if the input X is a well-formed JSON or JSON5 string.
    /// If the input X contains one or more syntax errors, then this function returns the character position of the first syntax error.
    /// The left-most character is position 1.
    ///
    /// If the input X is a BLOB, then this routine returns 0 if X is a well-formed JSONB blob. If the return value is positive,
    /// then it represents the approximate 1-based position in the BLOB of the first detected error.
    ///
    /// This function requires at least SQLite 3.46 or newer
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
    /// #     use diesel::dsl::{sql, json_error_position};
    /// #     use diesel::sql_types::{Binary, Text, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 46, 0);
    ///
    /// let result = diesel::select(json_error_position::<Text, _>(r#"{"a": "b", "c": 1}"#))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(0, result);
    ///
    /// let result = diesel::select(json_error_position::<Text, _>(r#"{"a": b", "c": 1}"#))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(7, result);
    ///
    /// let json5 = r#"
    ///     {
    ///         // A traditional message.
    ///         message: 'hello world',
    ///
    ///         // A number for some reason.
    ///         n: 42,
    ///     }
    /// "#;
    /// let result =
    ///     diesel::select(json_error_position::<Text, _>(json5)).get_result::<i32>(connection)?;
    ///
    /// assert_eq!(0, result);
    ///
    /// let json5_with_error = r#"
    ///     {
    ///         // A traditional message.
    ///         message: hello world',
    ///
    ///         // A number for some reason.
    ///         n: 42,
    ///     }
    /// "#;
    /// let result = diesel::select(json_error_position::<Text, _>(json5_with_error))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(59, result);
    ///
    /// let result = diesel::select(json_error_position::<Nullable<Text>, _>(None::<&str>))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// let result = diesel::select(json_error_position::<Binary, _>(br#"{"a": "b", "c": 1}"#))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(0, result);
    ///
    /// let result = diesel::select(json_error_position::<Binary, _>(br#"{"a": b", "c": 1}"#))
    ///     .get_result::<i32>(connection)?;
    ///
    /// assert_eq!(7, result);
    ///
    /// let result = diesel::select(json_error_position::<Nullable<Binary>, _>(None::<Vec<u8>>))
    ///     .get_result::<Option<i32>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn json_error_position<
        X: TextOrNullableTextOrBinaryOrNullableBinary + MaybeNullableValue<Integer>,
    >(
        x: X,
    ) -> X::Out;

    /// Converts the given json value to pretty-printed, indented text
    ///
    /// This function requires at least SQLite 3.46 or newer
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
    /// #     use diesel::dsl::{sql, json_pretty};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 46, 0);
    ///
    /// let result = diesel::select(json_pretty::<Json, _>(json!([{"f1":1,"f2":null},2,null,3])))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"[
    ///     {
    ///         "f1": 1,
    ///         "f2": null
    ///     },
    ///     2,
    ///     null,
    ///     3
    /// ]"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Json, _>(json!({"a": 1, "b": "cd"})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{
    ///     "a": 1,
    ///     "b": "cd"
    /// }"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Json, _>(json!("abc")))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#""abc""#, result);
    ///
    /// let result = diesel::select(json_pretty::<Json, _>(json!(22)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"22"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Json, _>(json!(false)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"false"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Json, _>(json!(null)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"null"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Json, _>(json!({})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{}"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Nullable<Json>, _>(None::<Value>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// let result = diesel::select(json_pretty::<Jsonb, _>(json!([{"f1":1,"f2":null},2,null,3])))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"[
    ///     {
    ///         "f1": 1,
    ///         "f2": null
    ///     },
    ///     2,
    ///     null,
    ///     3
    /// ]"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Jsonb, _>(json!({"a": 1, "b": "cd"})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{
    ///     "a": 1,
    ///     "b": "cd"
    /// }"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Jsonb, _>(json!("abc")))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#""abc""#, result);
    ///
    /// let result = diesel::select(json_pretty::<Jsonb, _>(json!(22)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"22"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Jsonb, _>(json!(false)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"false"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Jsonb, _>(json!(null)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"null"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Jsonb, _>(json!({})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{}"#, result);
    ///
    /// let result = diesel::select(json_pretty::<Nullable<Jsonb>, _>(None::<Value>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    /// #     Ok(())
    /// # }
    /// ```
    fn json_pretty<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Text>>(
        j: J,
    ) -> J::Out;

    /// Converts the given json value to pretty-printed, indented text
    ///
    /// This function requires at least SQLite 3.46 or newer
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
    /// #     use diesel::dsl::{sql, json_pretty_with_indentation};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 46, 0);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!([{"f1":1,"f2":null},2,null,3]), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"[
    ///   {
    ///     "f1": 1,
    ///     "f2": null
    ///   },
    ///   2,
    ///   null,
    ///   3
    /// ]"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!([{"f1":1,"f2":null},2,null,3]), None::<&str>))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"[
    ///     {
    ///         "f1": 1,
    ///         "f2": null
    ///     },
    ///     2,
    ///     null,
    ///     3
    /// ]"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!({"a": 1, "b": "cd"}), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{
    ///   "a": 1,
    ///   "b": "cd"
    /// }"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!("abc"), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#""abc""#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!(22), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"22"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!(false), None::<&str>))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"false"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!(null), None::<&str>))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"null"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Json, _, _>(json!({}), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{}"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Nullable<Json>, _, _>(None::<Value>, None::<&str>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!([{"f1":1,"f2":null},2,null,3]), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"[
    ///   {
    ///     "f1": 1,
    ///     "f2": null
    ///   },
    ///   2,
    ///   null,
    ///   3
    /// ]"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!([{"f1":1,"f2":null},2,null,3]), None::<&str>))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"[
    ///     {
    ///         "f1": 1,
    ///         "f2": null
    ///     },
    ///     2,
    ///     null,
    ///     3
    /// ]"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!({"a": 1, "b": "cd"}), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{
    ///   "a": 1,
    ///   "b": "cd"
    /// }"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!("abc"), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#""abc""#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!(22), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"22"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!(false), None::<&str>))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"false"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!(null), None::<&str>))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"null"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Jsonb, _, _>(json!({}), "  "))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!(r#"{}"#, result);
    ///
    /// let result = diesel::select(json_pretty_with_indentation::<Nullable<Jsonb>, _, _>(None::<Value>, None::<&str>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[sql_name = "json_pretty"]
    fn json_pretty_with_indentation<
        J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Text>,
    >(
        j: J,
        indentation: Nullable<Text>,
    ) -> J::Out;

    /// Returns  `true`  if the argument is well-formed JSON, or returns  `false`  if is not well-formed.
    ///
    /// This function requires at least SQLite 3.46 or newer
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
    /// #     use diesel::dsl::{sql, json_valid};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 46, 0);
    ///
    /// let result = diesel::select(json_valid::<Json, _>(json!({"x":35})))
    ///     .get_result::<bool>(connection)?;
    ///
    /// assert_eq!(true, result);
    ///
    /// let result = diesel::select(json_valid::<Nullable<Json>, _>(None::<serde_json::Value>))
    ///     .get_result::<Option<bool>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[sql_name = "json_valid"]
    #[cfg(feature = "sqlite")]
    fn json_valid<J: JsonOrNullableJson + MaybeNullableValue<Bool>>(j: J) -> J::Out;

    /// The json_type(X) function returns the "type" of the outermost element of X.
    /// The "type" returned by json_type() is one of the following SQL text values:
    /// 'null', 'true', 'false', 'integer', 'real', 'text', 'array', or 'object'.
    ///
    /// This function requires at least SQLite 3.38 or newer
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
    /// #     use diesel::dsl::{sql, json_type};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!({"a": [2, 3.5, true, false, null, "x"]})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("object", result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!({"a": [2, 3.5, true, false, null, "x"]})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("object", result);
    ///
    /// let result = diesel::select(json_type::<Nullable<Json>, _>(None::<serde_json::Value>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[sql_name = "json_type"]
    #[cfg(feature = "sqlite")]
    fn json_type<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Text>>(
        j: J,
    ) -> J::Out;

    /// The json_type(X,P) function returns the "type" of the element in X that is selected by path P.
    /// If the path P in json_type(X,P) selects an element that does not exist in X, then this function returns NULL.
    ///
    /// This function requires at least SQLite 3.38 or newer
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
    /// #     use diesel::dsl::{sql, json_type_with_path};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    ///
    /// let json_value = json!({"a": [2, 3.5, true, false, null, "x"]});
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json_value.clone(), "$.a"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(Some("array".to_string()), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json_value.clone(), "$.a[0]"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(Some("integer".to_string()), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json_value.clone(), "$.a[1]"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(Some("real".to_string()), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json_value.clone(), "$.a[2]"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(Some("true".to_string()), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json_value.clone(), "$.a[6]"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json_value.clone(), "$.a"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(Some("array".to_string()), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Nullable<Json>, _, _>(None::<serde_json::Value>, "$.a"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert_eq!(None, result);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[sql_name = "json_type"]
    #[cfg(feature = "sqlite")]
    fn json_type_with_path<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue>(
        j: J,
        path: Text,
    ) -> Nullable<Text>;

    /// The json_quote(X) function converts the SQL value X (a number or a string) into its corresponding JSON
    /// representation. If X is a JSON value returned by another JSON function, then this function is a no-op.
    ///
    /// This function requires at least SQLite 3.38 or newer
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
    /// #     use diesel::dsl::{sql, json_quote};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Integer, Float, Double, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    ///
    /// let result = diesel::select(json_quote::<Integer, _>(42))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(json!(42), result);
    ///
    /// let result = diesel::select(json_quote::<Text, _>("verdant"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(json!("verdant"), result);
    ///
    /// let result = diesel::select(json_quote::<Text, _>("[1]"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(json!("[1]"), result);
    ///
    /// let result = diesel::select(json_quote::<Nullable<Text>, _>(None::<&str>))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(json!(null), result);
    ///
    /// let result = diesel::select(json_quote::<Double, _>(3.14159))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(json!(3.14159), result);
    ///
    /// let result = diesel::select(json_quote::<Json, _>(json!([1])))
    ///     .get_result::<Value>(connection)?;
    // assert_eq!(json!([1]), result);
    ///
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[sql_name = "json_quote"]
    #[cfg(feature = "sqlite")]
    fn json_quote<J: SqlType + SingleValue>(j: J) -> Json;

    /// The `json_group_array(X)` function is an aggregate SQL function that returns a JSON array comprised of
    /// all X values in the aggregation.
    ///
    /// ## Aggregate Function Expression
    ///
    /// This function can be used as aggregate expression. See [`AggregateExpressionMethods`] for details.
    ///
    /// # Examples
    ///
    /// ## Normal function usage
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
    /// #     use diesel::dsl::*;
    /// #     use schema::animals::dsl::*;
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #
    /// let result = animals
    ///     .select(json_group_array(species))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!(["dog", "spider"]));
    ///
    /// let result = animals
    ///     .select(json_group_array(legs))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!([4, 8]));
    ///
    /// let result = animals
    ///     .select(json_group_array(name))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!(["Jack", null]));
    ///
    /// # Ok(())
    /// # }
    /// ```
    /// ## Aggregate function expression
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
    /// #     use diesel::dsl::*;
    /// #     use schema::animals::dsl::*;
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #
    /// let result = animals
    ///     .select(json_group_array(species).aggregate_filter(legs.lt(8)))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!(["dog"]));
    ///
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See also
    /// - [`jsonb_group_array`](jsonb_group_array()) will return data in JSONB format instead of JSON.
    /// - [`json_group_object`](json_group_object()) will return JSON object instead of array.
    #[cfg(feature = "sqlite")]
    #[aggregate]
    fn json_group_array<E: SqlType + SingleValue>(elements: E) -> Json;

    /// The `jsonb_group_array(X)` function is an aggregate SQL function that returns a JSONB array comprised of
    /// all X values in the aggregation.
    ///
    /// ## Aggregate Function Expression
    ///
    /// This function can be used as aggregate expression. See [`AggregateExpressionMethods`] for details.
    ///
    /// # Examples
    ///
    /// ## Normal function usage
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
    /// #     use diesel::dsl::*;
    /// #     use schema::animals::dsl::*;
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #
    /// let result = animals
    ///     .select(json_group_array(species))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!(["dog", "spider"]));
    ///
    /// let result = animals
    ///     .select(json_group_array(legs))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!([4, 8]));
    ///
    /// let result = animals
    ///     .select(json_group_array(name))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!(["Jack", null]));
    ///
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Aggregate function expression
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
    /// #     use diesel::dsl::*;
    /// #     use schema::animals::dsl::*;
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #
    /// let result = animals
    ///     .select(json_group_array(species).aggregate_filter(legs.lt(8)))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(result, json!(["dog"]));
    ///
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See also
    /// - [`json_group_array`](json_group_array()) will return data in JSON format instead of JSONB.
    /// - [`jsonb_group_object`](jsonb_group_object()) will return JSONB object instead of array.
    #[cfg(feature = "sqlite")]
    #[aggregate]
    fn jsonb_group_array<E: SqlType + SingleValue>(elements: E) -> Jsonb;

    /// The json_group_object(NAME,VALUE) function returns a JSON object comprised of all NAME/VALUE pairs in
    /// the aggregation.
    ///
    /// A potential edge case in this function arises when `names` contains duplicate elements.
    /// In such case, the result will include all duplicates (e.g., `{"key": 1, "key": 2, "key": 3}`).
    /// Note that any duplicate entries in the resulting JSON will be removed during deserialization.
    ///
    /// This function requires at least SQLite 3.38 or newer
    ///
    /// ## Aggregate Function Expression
    ///
    /// This function can be used as aggregate expression. See [`AggregateExpressionMethods`] for details.
    ///
    /// # Examples
    ///
    /// ## Normal function usage
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::Text;
    /// #     use serde_json::json;
    /// #     use schema::animals::dsl::*;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    /// #
    /// let result = animals.select(json_group_object(species, name)).get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!({"dog":"Jack","spider":null}), result);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Aggregate function expression
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::Text;
    /// #     use serde_json::json;
    /// #     use schema::animals::dsl::*;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #
    /// #     let version = diesel::select(sql::<Text>("sqlite_version();"))
    /// #         .get_result::<String>(connection)?;
    /// #
    /// #     let version_components: Vec<&str> = version.split('.').collect();
    /// #     let major: u32 = version_components[0].parse().unwrap();
    /// #     let minor: u32 = version_components[1].parse().unwrap();
    /// #
    /// #     if major < 3 || minor < 38 {
    /// #         println!("SQLite version is too old, skipping the test.");
    /// #         return Ok(());
    /// #     }
    /// #
    /// let result = animals.select(json_group_object(species, name).aggregate_filter(legs.lt(8))).get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!({"dog":"Jack"}), result);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See also
    /// - [`jsonb_group_object`](jsonb_group_object()) will return data in JSONB format instead of JSON.
    /// - [`json_group_array`](json_group_array()) will return JSON array instead of object.
    #[cfg(feature = "sqlite")]
    #[aggregate]
    fn json_group_object<
        N: SqlType<IsNull = is_nullable::NotNull> + SingleValue,
        V: SqlType + SingleValue,
    >(
        names: N,
        values: V,
    ) -> Json;

    /// The jsonb_group_object(NAME,VALUE) function returns a JSONB object comprised of all NAME/VALUE pairs in
    /// the aggregation.
    ///
    /// A potential edge case in this function arises when `names` contains duplicate elements.
    /// In such case, the result will include all duplicates (e.g., `{"key": 1, "key": 2, "key": 3}`).
    /// Note that any duplicate entries in the resulting JSONB will be removed during deserialization.
    ///
    /// This function requires at least SQLite 3.38 or newer
    ///
    /// ## Aggregate Function Expression
    ///
    /// This function can be used as aggregate expression. See [`AggregateExpressionMethods`] for details.
    ///
    /// # Examples
    ///
    /// ## Normal function usage
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::Text;
    /// #     use serde_json::json;
    /// #     use schema::animals::dsl::*;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    /// #
    /// let result = animals.select(jsonb_group_object(species, name)).get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!({"dog":"Jack","spider":null}), result);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Aggregate function expression
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::Text;
    /// #     use serde_json::json;
    /// #     use schema::animals::dsl::*;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #
    /// #     let version = diesel::select(sql::<Text>("sqlite_version();"))
    /// #         .get_result::<String>(connection)?;
    /// #
    /// #     let version_components: Vec<&str> = version.split('.').collect();
    /// #     let major: u32 = version_components[0].parse().unwrap();
    /// #     let minor: u32 = version_components[1].parse().unwrap();
    /// #
    /// #     if major < 3 || minor < 38 {
    /// #         println!("SQLite version is too old, skipping the test.");
    /// #         return Ok(());
    /// #     }
    /// #
    /// let result = animals.select(jsonb_group_object(species, name).aggregate_filter(legs.lt(8))).get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!({"dog":"Jack"}), result);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See also
    /// - [`json_group_object`](jsonb_group_array()) will return data in JSON format instead of JSONB.
    /// - [`jsonb_group_array`](jsonb_group_array()) will return JSONB array instead of object.
    #[cfg(feature = "sqlite")]
    #[aggregate]
    fn jsonb_group_object<
        N: SqlType<IsNull = is_nullable::NotNull> + SingleValue,
        V: SqlType + SingleValue,
    >(
        names: N,
        values: V,
    ) -> Jsonb;

    /// The `json_array()` SQL function accepts zero or more arguments and returns a well-formed JSON array
    /// that is composed from those arguments. Note that arguments of type BLOB will not be accepted by this
    /// function.
    ///
    /// An argument with SQL type TEXT is normally converted into a quoted JSON string. However, if the
    /// argument is the output from another json function, then it is stored as JSON. This allows calls to
    /// `json_array()` and `json_object()` to be nested. The [`json()`] function can also be used to force
    /// strings to be recognized as JSON.
    ///
    /// This function requires at least SQLite 3.38 or newer
    ///
    /// # Examples
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::{Text, Double};
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    /// #
    /// let result = diesel::select(json_array_0()).get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!([]), result);
    ///
    /// let result = diesel::select(json_array_1::<Text, _>("abc"))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!(["abc"]), result);
    ///
    /// let result = diesel::select(json_array_2::<Text, Double, _, _>("abc", 3.1415))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!(["abc", 3.1415]), result);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    #[variadic(1)]
    fn json_array<V: NotBlob>(value: V) -> Json;

    /// The `jsonb_array()` SQL function accepts zero or more arguments and returns a well-formed JSON array
    /// that is composed from those arguments. Note that arguments of type BLOB will not be accepted by this
    /// function.
    ///
    /// An argument with SQL type TEXT is normally converted into a quoted JSON string. However, if the
    /// argument is the output from another json function, then it is stored as JSON. This allows calls to
    /// `jsonb_array()` and `jsonb_object()` to be nested. The [`json()`] function can also be used to force
    /// strings to be recognized as JSON.
    ///
    /// This function works just like the [`json_array()`](json_array_1()) function except that it returns the
    /// constructed JSON array in the SQLite's private JSONB format rather than in the standard RFC 8259 text
    /// format.
    ///
    /// This function requires at least SQLite 3.38 or newer
    ///
    /// # Examples
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::{Text, Double};
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    /// #
    /// let result = diesel::select(jsonb_array_0()).get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!([]), result);
    ///
    /// let result = diesel::select(jsonb_array_1::<Text, _>("abc"))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!(["abc"]), result);
    ///
    /// let result = diesel::select(jsonb_array_2::<Text, Double, _, _>("abc", 3.1415))
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(json!(["abc", 3.1415]), result);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    #[variadic(1)]
    fn jsonb_array<V: NotBlob>(value: V) -> Jsonb;

    /// The `json_remove(X,P,...)` SQL function takes a single JSON value as its first argument followed by
    /// zero or more path arguments. The `json_remove(X,P,...)` function returns a copy of the X parameter
    /// with all the elements identified by path arguments removed. Paths that select elements not found in X
    /// are silently ignored.
    ///
    /// Removals occurs sequentially from left to right. Changes caused by prior removals can affect the path
    /// search for subsequent arguments.
    ///
    /// If the `json_remove(X)` function is called with no path arguments, then it returns the input X
    /// reformatted, with excess whitespace removed.
    ///
    /// The `json_remove()` function throws an error if any of the path arguments is not a well-formed path.
    ///
    /// This function requires at least SQLite 3.38 or newer
    ///
    /// # Examples
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::{Json, Text};
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    /// #
    /// let json = json!(['a', 'b', 'c', 'd']);
    /// let result = diesel::select(json_remove_0::<Json, _>(json))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!(['a', 'b', 'c', 'd'])), result);
    ///
    /// // Values are removed sequentially from left to right.
    /// let json = json!(['a', 'b', 'c', 'd']);
    /// let result = diesel::select(json_remove_2::<Json, _, _, _>(json, "$[0]", "$[2]"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!(['b', 'c'])), result);
    ///
    /// let json = json!({"a": 10, "b": 20});
    /// let result = diesel::select(json_remove_1::<Json, _, _>(json, "$.a"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!({"b": 20})), result);
    ///
    /// // Paths that select not existing elements are silently ignored.
    /// let json = json!({"a": 10, "b": 20});
    /// let result = diesel::select(json_remove_1::<Json, _, _>(json, "$.c"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!({"a": 10, "b": 20})), result);
    ///
    /// let json = json!({"a": 10, "b": 20});
    /// let result = diesel::select(json_remove_1::<Json, _, _>(json, "$"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(None, result);
    ///
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    #[variadic(1)]
    fn json_remove<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue>(
        json: J,
        path: Text,
    ) -> Nullable<Json>;

    /// The `jsonb_remove(X,P,...)` SQL function takes a single JSON value as its first argument followed by
    /// zero or more path arguments. The `jsonb_remove(X,P,...)` function returns a copy of the X parameter
    /// with all the elements identified by path arguments removed. Paths that select elements not found in X
    /// are silently ignored.
    ///
    /// Removals occurs sequentially from left to right. Changes caused by prior removals can affect the path
    /// search for subsequent arguments.
    ///
    /// If the `jsonb_remove(X)` function is called with no path arguments, then it returns the input X
    /// reformatted, with excess whitespace removed.
    ///
    /// The `jsonb_remove()` function throws an error if any of the path arguments is not a well-formed path.
    ///
    /// This function returns value in a binary JSONB format.
    ///
    /// This function requires at least SQLite 3.38 or newer
    ///
    /// # Examples
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
    /// #     use diesel::dsl::*;
    /// #     use diesel::sql_types::{Jsonb, Text};
    /// #     use serde_json::json;
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 38, 0);
    /// #
    /// let json = json!(['a', 'b', 'c', 'd']);
    /// let result = diesel::select(jsonb_remove_0::<Jsonb, _>(json))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!(['a', 'b', 'c', 'd'])), result);
    ///
    /// // Values are removed sequentially from left to right.
    /// let json = json!(['a', 'b', 'c', 'd']);
    /// let result = diesel::select(jsonb_remove_2::<Jsonb, _, _, _>(json, "$[0]", "$[2]"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!(['b', 'c'])), result);
    ///
    /// let json = json!({"a": 10, "b": 20});
    /// let result = diesel::select(jsonb_remove_1::<Jsonb, _, _>(json, "$.a"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!({"b": 20})), result);
    ///
    /// // Paths that select not existing elements are silently ignored.
    /// let json = json!({"a": 10, "b": 20});
    /// let result = diesel::select(jsonb_remove_1::<Jsonb, _, _>(json, "$.c"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(Some(json!({"a": 10, "b": 20})), result);
    ///
    /// let json = json!({"a": 10, "b": 20});
    /// let result = diesel::select(jsonb_remove_1::<Jsonb, _, _>(json, "$"))
    ///     .get_result::<Option<serde_json::Value>>(connection)?;
    /// assert_eq!(None, result);
    ///
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    #[variadic(1)]
    fn jsonb_remove<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue>(
        json: J,
        path: Text,
    ) -> Nullable<Jsonb>;

    /// Applies an RFC 7396 MergePatch `patch` to the input JSON `target` and
    /// returns the patched JSON value.
    ///
    /// MergePatch can add, modify, or delete elements of a JSON object. Arrays are
    /// treated as atomic values: they can only be inserted, replaced, or deleted as a
    /// whole, not modified element-wise.
    ///
    /// # Examples
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
    /// #     use diesel::dsl::json_patch;
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Json, Nullable};
    /// #     let connection = &mut establish_connection();
    ///
    /// let result = diesel::select(json_patch::<Json, Json, _, _>(
    ///     json!( {"a":1,"b":2} ),
    ///     json!( {"c":3,"d":4} ),
    /// ))
    /// .get_result::<Value>(connection)?;
    /// assert_eq!(json!({"a":1,"b":2,"c":3,"d":4}), result);
    ///
    /// let result = diesel::select(json_patch::<Json, Json, _, _>(
    ///     json!( {"a":[1,2],"b":2} ),
    ///     json!( {"a":9} ),
    /// ))
    /// .get_result::<Value>(connection)?;
    /// assert_eq!(json!({"a":9,"b":2}), result);
    ///
    /// let result = diesel::select(json_patch::<Json, Json, _, _>(
    ///     json!( {"a":[1,2],"b":2} ),
    ///     json!( {"a":null} ),
    /// ))
    /// .get_result::<Value>(connection)?;
    /// assert_eq!(json!({"b":2}), result);
    ///
    /// let result = diesel::select(json_patch::<Json, Json, _, _>(
    ///     json!( {"a":1,"b":2} ),
    ///     json!( {"a":9,"b":null,"c":8} ),
    /// ))
    /// .get_result::<Value>(connection)?;
    /// assert_eq!(json!({"a":9,"c":8}), result);
    ///
    /// let result = diesel::select(json_patch::<Json, Json, _, _>(
    ///     json!( {"a":{"x":1,"y":2},"b":3} ),
    ///     json!( {"a":{"y":9},"c":8} ),
    /// ))
    /// .get_result::<Value>(connection)?;
    /// assert_eq!(
    ///     json!({"a":{"x":1,"y":9},"b":3,"c":8}),
    ///     result
    /// );
    ///
    /// // Nullable input yields nullable output
    /// let result = diesel::select(json_patch::<Nullable<Json>, Json, _, _>(
    ///     None::<Value>,
    ///     json!({}),
    /// ))
    /// .get_result::<Option<Value>>(connection)?;
    /// assert!(result.is_none());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn json_patch<
        T: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue,
        P: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue + CombinedNullableValue<T, Json>,
    >(
        target: T,
        patch: P,
    ) -> P::Out;

    /// Applies an RFC 7396 MergePatch `patch` to the input JSON `target` and
    /// returns the patched JSON value in SQLite's binary JSONB format.
    ///
    /// See [`json_patch`](json_patch()) for details about the MergePatch semantics.
    ///
    /// # Examples
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
    /// #     use diesel::dsl::jsonb_patch;
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    /// #     assert_version!(connection, 3, 45, 0);
    ///
    /// let result = diesel::select(jsonb_patch::<Jsonb, Jsonb, _, _>(
    ///     json!( {"a":1,"b":2} ),
    ///     json!( {"c":3,"d":4} ),
    /// ))
    /// .get_result::<Value>(connection)?;
    /// assert_eq!(json!({"a":1,"b":2,"c":3,"d":4}), result);
    ///
    /// // Nullable input yields nullable output
    /// let result = diesel::select(jsonb_patch::<Nullable<Jsonb>, Jsonb, _, _>(
    ///     None::<Value>,
    ///     json!({}),
    /// ))
    /// .get_result::<Option<Value>>(connection)?;
    /// assert!(result.is_none());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn jsonb_patch<
        T: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue,
        P: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue + CombinedNullableValue<T, Jsonb>,
    >(
        target: T,
        patch: P,
    ) -> P::Out;
}

pub(super) mod return_type_helpers_reexported {
    #[allow(unused_imports)]
    #[doc(inline)]
    pub use super::return_type_helpers::*;
}
