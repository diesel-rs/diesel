//! SQLite specific functions
use crate::expression::{functions::declare_sql_function, AsExpression};
use crate::sql_types::*;
use crate::sqlite::expression::expression_methods::BinaryOrNullableBinary;
use crate::sqlite::expression::expression_methods::JsonOrNullableJson;
use crate::sqlite::expression::expression_methods::JsonOrNullableJsonOrJsonbOrNullableJsonb;
use crate::sqlite::expression::expression_methods::MaybeNullableValue;
use crate::sqlite::expression::expression_methods::TextOrNullableText;
use crate::sqlite::expression::expression_methods::TextOrNullableTextOrBinaryOrNullableBinary;

#[cfg(feature = "sqlite")]
#[declare_sql_function]
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 45) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 46) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 46) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 46) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    /// let result = diesel::select(json_error_position::<Text, _>(json5))
    ///     .get_result::<i32>(connection)?;
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 46) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 46) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 46) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 38) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 38) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
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

    /// The json_extract(X,P1,P2,...) extracts and returns one or more values from the well-formed JSON at X.
    ///
    /// If only a single path P1 is provided, then the SQL datatype of the result is NULL for a JSON null,
    /// INTEGER or REAL for a JSON numeric value, an INTEGER zero for a JSON false value, an INTEGER one for a JSON true value,
    /// the dequoted text for a JSON string value, and a text representation for JSON object and array values.
    ///
    /// If there are multiple path arguments (P1, P2, and so forth) then this routine returns SQLite text
    /// which is a well-formed JSON array holding the various values.
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
    /// #     use diesel::dsl::{sql, json_extract};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Json, Nullable};
    /// #     let connection = &mut establish_connection();
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 38) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
    ///
    /// let json_obj = json!({"a": 2, "c": [4, 5, {"f": 7}]});
    ///
    /// // Extract the entire JSON object
    /// let result = diesel::select(json_extract::<Json, _>(json_obj.clone(), "$"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(result, json!({"a": 2, "c": [4, 5, {"f": 7}]}));
    ///
    /// // Extract a nested array
    /// let result = diesel::select(json_extract::<Json, _>(json_obj.clone(), "$.c"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(result, json!([4, 5, {"f": 7}]));
    ///
    /// // Extract a nested object
    /// let result = diesel::select(json_extract::<Json, _>(json_obj.clone(), "$.c[2]"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(result, json!({"f": 7}));
    ///
    /// // Extract a numeric value
    /// let result = diesel::select(json_extract::<Json, _>(json_obj.clone(), "$.c[2].f"))
    ///     .get_result::<i64>(connection)?;
    /// assert_eq!(result, 7);
    ///
    /// // Extract a non-existent path (returns NULL)
    /// let result = diesel::select(json_extract::<Json, _>(json_obj.clone(), "$.x"))
    ///     .get_result::<Option<Value>>(connection)?;
    /// assert_eq!(result, None);
    ///
    /// // Extract a string value
    /// let json_with_string = json!({"a": "xyz"});
    /// let result = diesel::select(json_extract::<Json, _>(json_with_string, "$.a"))
    ///     .get_result::<String>(connection)?;
    /// assert_eq!(result, "xyz");
    ///
    /// // Extract a null value
    /// let json_with_null = json!({"a": null});
    /// let result = diesel::select(json_extract::<Json, _>(json_with_null, "$.a"))
    ///     .get_result::<Option<Value>>(connection)?;
    /// assert_eq!(result, None);
    ///
    /// // Test with NULL JSON input
    /// let result = diesel::select(json_extract::<Nullable<Json>, _>(None::<Value>, "$.a"))
    ///     .get_result::<Option<Value>>(connection)?;
    /// assert_eq!(result, None);
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn json_extract<
        J: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue,
        P: AsExpression<Text> + SingleValue,
    >(
        j: J,
        path: P,
    ) -> Nullable<Text>;

    /// The jsonb_extract() function works the same as the json_extract() function, except in cases
    /// where json_extract() would normally return a text JSON array object, this routine returns
    /// the array or object in the JSONB format.
    ///
    /// For the common case where a text, numeric, null, or boolean JSON element is returned,
    /// this routine works exactly the same as json_extract().
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
    /// #     use diesel::dsl::{sql, jsonb_extract};
    /// #     use serde_json::{json, Value};
    /// #     use diesel::sql_types::{Text, Jsonb, Nullable};
    /// #     let connection = &mut establish_connection();
    ///
    /// let version = diesel::select(sql::<Text>("sqlite_version();"))
    ///         .get_result::<String>(connection)?;
    ///
    /// // Querying SQLite version should not fail.
    /// let version_components: Vec<&str> = version.split('.').collect();
    /// let major: u32 = version_components[0].parse().unwrap();
    /// let minor: u32 = version_components[1].parse().unwrap();
    /// let patch: u32 = version_components[2].parse().unwrap();
    ///
    /// if major > 3 || (major == 3 && minor >= 45) {
    ///     /* Valid sqlite version, do nothing */
    /// } else {
    ///     println!("SQLite version is too old, skipping the test.");
    ///     return Ok(());
    /// }
    ///
    /// let json_obj = json!({"a": 2, "c": [4, 5, {"f": 7}]});
    ///
    /// // Extract the entire JSON object
    /// let result = diesel::select(jsonb_extract::<Jsonb, _>(json_obj.clone(), "$"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(result, json!({"a": 2, "c": [4, 5, {"f": 7}]}));
    ///
    /// // Extract a nested array
    /// let result = diesel::select(jsonb_extract::<Jsonb, _>(json_obj.clone(), "$.c"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(result, json!([4, 5, {"f": 7}]));
    ///
    /// // Extract a nested object
    /// let result = diesel::select(jsonb_extract::<Jsonb, _>(json_obj.clone(), "$.c[2]"))
    ///     .get_result::<Value>(connection)?;
    /// assert_eq!(result, json!({"f": 7}));
    ///
    /// // Extract a numeric value
    /// let result = diesel::select(jsonb_extract::<Jsonb, _>(json_obj.clone(), "$.c[2].f"))
    ///     .get_result::<i64>(connection)?;
    /// assert_eq!(result, 7);
    ///
    /// // Extract a non-existent path (returns NULL)
    /// let result = diesel::select(jsonb_extract::<Jsonb, _>(json_obj.clone(), "$.x"))
    ///     .get_result::<Option<Value>>(connection)?;
    /// assert_eq!(result, None);
    ///
    /// // Extract a string value
    /// let json_with_string = json!({"a": "xyz"});
    /// let result = diesel::select(jsonb_extract::<Jsonb, _>(json_with_string, "$.a"))
    ///     .get_result::<String>(connection)?;
    /// assert_eq!(result, "xyz");
    ///
    /// // Extract a null value
    /// let json_with_null = json!({"a": null});
    /// let result = diesel::select(jsonb_extract::<Jsonb, _>(json_with_null, "$.a"))
    ///     .get_result::<Option<Value>>(connection)?;
    /// assert_eq!(result, None);
    ///
    /// // Test with NULL JSON input
    /// let result = diesel::select(jsonb_extract::<Nullable<Jsonb>, _>(None::<Value>, "$.a"))
    ///     .get_result::<Option<Value>>(connection)?;
    /// assert_eq!(result, None);
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn jsonb_extract<
        J: JsonOrNullableJsonOrJsonbOrNullableJsonb + SingleValue,
        P: AsExpression<Text> + SingleValue,
    >(
        j: J,
        path: P,
    ) -> Nullable<Text>;
}
