/// Parses the body of a struct field, extracting the relevant information the
/// relevant information that we care about. This macro can handle either named
/// structs or tuple structs. It does not handle unit structs.
///
/// When calling this macro from the outside, it takes three arguments. The
/// first is a single token tree of passthrough information, which will be given
/// to the callback unchanged. The second is the name of the macro to call with
/// the parsed field. The third is the *entire* body of the struct, including
/// either the curly braces or parens.
///
/// If a tuple struct is given, all fields *must* be annotated with
/// `#[column_name(name)]`. Due to the nature of non-procedural macros, we
/// cannot give a helpful error message in this case.
///
/// The callback will be called with the given headers, and a list of fields
/// in record form with the following properties:
///
/// - `field_name` is the name of the field on the struct. This will not be
///   present if the struct is a tuple struct.
/// - `column_name` is the column the field corresponds to. This will either be
///   the value of a `#[column_name]` attribute on the field, or the field name
///   if not present.
/// - `field_type` is the type of the field on the struct.
/// - `field_kind` Will be either `regular` or `option` depending on whether
///   the type of the field was an option or not.
///
/// # Example
///
/// If this macro is called with:
///
/// ```ignore
/// __diesel_parse_struct_body {
///     (my original arguments),
///     callback = my_macro,
///     body = {
///         pub foo: i32,
///         bar: Option<i32>,
///         #[column_name(other)]
///         baz: String,
///     }
/// }
/// ```
///
/// Then the resulting expansion will be:
///
/// ```ignore
/// my_macro! {
///     (my original arguments),
///     fields = [{
///         field_name: foo,
///         column_name: foo,
///         field_ty: i32,
///         field_kind: regular,
///     }, {
///         field_name: bar,
///         column_name: bar,
///         field_ty: Option<i32>,
///         field_kind: option,
///     }, {
///         field_name: baz,
///         column_name: other,
///         field_ty: String,
///         field_kind: regular,
///     }],
/// }
#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_parse_struct_body {
    // Entry point for named structs
    (
        $headers:tt,
        callback = $callback:ident,
        body = {$($body:tt)*},
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = [],
            body = ($($body)*,),
        }
    };

    // Entry point for tuple structs
    (
        $headers:tt,
        callback = $callback:ident,
        body = ($($body:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = [],
            body = ($($body)*,),
        }
    };

    // FIXME: Replace with `vis` specifier if relevant RFC lands
    // First, strip `pub` if it exists
    (
        $headers:tt,
        callback = $callback:ident,
        fields = $fields:tt,
        body = (
            $(#$meta:tt)*
            pub $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = $fields,
            body = ($(#$meta)* $($tail)*),
        }
    };

    // Since we blindly add a comma to the end of the body, we might have a
    // double trailing comma.  If it's the only token left, that's what
    // happened. Strip it.
    (
        $headers:tt,
        callback = $callback:ident,
        fields = $fields:tt,
        body = (,),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = $fields,
            body = (),
        }
    };

    // When we find #[column_name] followed by an option type, handle the
    // tuple struct field
    (
        $headers:tt,
        callback = $callback:ident,
        fields = [$($fields:tt)*],
        body = (
            #[column_name($column_name:ident)]
            Option<$field_ty:ty> , $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = [$($fields)* {
                column_name: $column_name,
                field_ty: Option<$field_ty>,
                field_kind: option,
            }],
            body = ($($tail)*),
        }
    };

    // When we find #[column_name] followed by a type, handle the tuple struct
    // field
    (
        $headers:tt,
        callback = $callback:ident,
        fields = [$($fields:tt)*],
        body = (
            #[column_name($column_name:ident)]
            $field_ty:ty , $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = [$($fields)* {
                column_name: $column_name,
                field_ty: $field_ty,
                field_kind: regular,
            }],
            body = ($($tail)*),
        }
    };

    // When we find #[column_name] followed by a named field, handle it
    (
        $headers:tt,
        callback = $callback:ident,
        fields = $fields:tt,
        body = (
            #[column_name($column_name:ident)]
            $field_name:ident : $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = $fields,
            body = ($field_name as $column_name : $($tail)*),
        }
    };

    // If we got here and didn't have a #[column_name] attr,
    // then the column name is the same as the field name
    (
        $headers:tt,
        callback = $callback:ident,
        fields = $fields:tt,
        body = ($field_name:ident : $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = $fields,
            body = ($field_name as $field_name : $($tail)*),
        }
    };

    // At this point we know the column and field name, handle when the type is option
    (
        $headers:tt,
        callback = $callback:ident,
        fields = [$($fields:tt)*],
        body = ($field_name:ident as $column_name:ident : Option<$field_ty:ty>, $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = [$($fields)* {
                field_name: $field_name,
                column_name: $column_name,
                field_ty: Option<$field_ty>,
                field_kind: option,
            }],
            body = ($($tail)*),
        }
    };

    // Handle any type other than option
    (
        $headers:tt,
        callback = $callback:ident,
        fields = [$($fields:tt)*],
        body = ($field_name:ident as $column_name:ident : $field_ty:ty, $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = [$($fields)* {
                field_name: $field_name,
                column_name: $column_name,
                field_ty: $field_ty,
                field_kind: regular,
            }],
            body = ($($tail)*),
        }
    };

    // When we reach a type with no column name annotation, handle the unnamed
    // tuple struct field. Since we require that either all fields are annotated
    // or none are, we could actually handle the whole body in one pass for this
    // case. However, anything using tuple structs without the column name
    // likely needs some ident per field to be useable and by handling each
    // field separately this way, the `field_kind` acts as a fresh ident each
    // time.
    (
        $headers:tt,
        callback = $callback:ident,
        fields = [$($fields:tt)*],
        body = ($field_ty:ty , $($tail:tt)*),
    ) => {
        __diesel_parse_struct_body! {
            $headers,
            callback = $callback,
            fields = [$($fields)* {
                field_ty: $field_ty,
                field_kind: bare,
            }],
            body = ($($tail)*),
        }
    };

    // At this point we've parsed the entire body. We create the pattern
    // for destructuring, and pass all the information back to the main macro
    // to generate the final impl
    (
        $headers:tt,
        callback = $callback:ident,
        fields = $fields:tt,
        body = (),
    ) => {
        $callback! {
            $headers,
            fields = $fields,
        }
    };
}

/// Hack to tell the compiler that something is in fact an item. This is needed
/// when `tt` fragments are used in specific positions.
#[doc(hidden)]
#[macro_export]
macro_rules!  __diesel_parse_as_item {
    ($i:item) => { $i }
}
