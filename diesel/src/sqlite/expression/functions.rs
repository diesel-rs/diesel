//! SQLite specific functions
use crate::expression::functions::define_sql_function;
use crate::sql_types::*;
use crate::sqlite::expression::expression_methods::BinaryOrNullableBinary;
use crate::sqlite::expression::expression_methods::JsonOrNullableJsonOrJsonbOrNullableJsonb;
use crate::sqlite::expression::expression_methods::MaybeNullableValue;
use crate::sqlite::expression::expression_methods::TextOrNullableText;

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
}

#[cfg(feature = "sqlite")]
define_sql_function! {
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
}

#[cfg(feature = "sqlite")]
define_sql_function! {
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
    fn json_pretty<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Text>>(j: J) -> J::Out;
}

#[cfg(feature = "sqlite")]
define_sql_function! {
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
    fn json_pretty_with_indentation<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Text>>(j: J, indentation: Nullable<Text>) -> J::Out;
}

#[cfg(feature = "sqlite")]
define_sql_function! {
    /// Returns the "type" of the outermost element of X.
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
    /// #     use diesel::dsl::json_type;
    /// #     use diesel::sql_types::{Json, Jsonb, Nullable};
    /// #     use serde_json::{json, Value};
    /// #     let connection = &mut establish_connection();
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!({"a": "b", "c": 1})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("object".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!([1,2,3])))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("array".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!("abc")))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("text".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!(-123.4)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("real".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!(42)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("integer".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!(true)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("true".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!(false)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("false".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Json, _>(json!(null)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("null".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Nullable<Json>, _>(None::<Value>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    ///
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!({"a": "b", "c": 1})))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("object".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!([1,2,3])))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("array".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!("abc")))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("text".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!(-123.4)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("real".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!(42)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("integer".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!(true)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("true".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!(false)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("false".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Jsonb, _>(json!(null)))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("null".to_string(), result);
    ///
    /// let result = diesel::select(json_type::<Nullable<Jsonb>, _>(None::<Value>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn json_type<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Text>>(j: J) -> J::Out;
}

#[cfg(feature = "sqlite")]
define_sql_function! {
    /// Returns the "type" of the element in X that is selected by path P.
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
    /// #     use diesel::dsl::json_type_with_path;
    /// #     use diesel::sql_types::{Json, Jsonb, Nullable};
    /// #     use serde_json::{json, Value};
    /// #     let connection = &mut establish_connection();
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("object".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("array".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[0]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("integer".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[1]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("real".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[2]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("true".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[3]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("false".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[4]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("null".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Json, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[5]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("text".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Nullable<Json>, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[6]"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// let result = diesel::select(json_type_with_path::<Nullable<Json>, _, _>(None::<Value>, None::<&str>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    ///
    ///
    ///
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("object".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("array".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[0]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("integer".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[1]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("real".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[2]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("true".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[3]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("false".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[4]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("null".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Jsonb, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[5]"))
    ///     .get_result::<String>(connection)?;
    ///
    /// assert_eq!("text".to_string(), result);
    ///
    /// let result = diesel::select(json_type_with_path::<Nullable<Jsonb>, _, _>(json!({"a":[2,3.5,true,false,null,"x"]}), "$.a[6]"))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// let result = diesel::select(json_type_with_path::<Nullable<Jsonb>, _, _>(None::<Value>, None::<&str>))
    ///     .get_result::<Option<String>>(connection)?;
    ///
    /// assert!(result.is_none());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    #[sql_name = "json_type"]
    fn json_type_with_path<J: JsonOrNullableJsonOrJsonbOrNullableJsonb + MaybeNullableValue<Text>>(j: J, path: Nullable<Text>) -> J::Out;
}
