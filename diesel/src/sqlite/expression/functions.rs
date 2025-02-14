//! SQLite specific functions
use crate::expression::functions::declare_sql_function;
use crate::sql_types::*;
use crate::sqlite::expression::expression_methods::BinaryOrNullableBinary;
use crate::sqlite::expression::expression_methods::JsonOrNullableJsonOrJsonbOrNullableJsonb;
use crate::sqlite::expression::expression_methods::MaybeNullableValue;
use crate::sqlite::expression::expression_methods::TextOrNullableText;

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
}
