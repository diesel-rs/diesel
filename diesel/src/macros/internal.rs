/// This will implement `SelectableExpression` and `AppearsOnTable` for "simple"
/// composite nodes where the where clause is roughly `AllTyParams:
/// SelectableExpression<QS>, Self: Expression`.
///
/// This macro is exported because we want to be able to call it from other
/// macros that are exported, but it is not part of our public API.
#[macro_export]
#[doc(hidden)]
macro_rules! impl_selectable_expression {
    ($struct_name:ident) => {
        impl_selectable_expression!(ty_params = (), struct_ty = $struct_name,);
    };

    ($struct_name:ident<$($ty_params:ident),+>) => {
        impl_selectable_expression!(
            ty_params = ($($ty_params),+),
            struct_ty = $struct_name<$($ty_params),+>,
        );
    };

    (ty_params = ($($ty_params:ident),*), struct_ty = $struct_ty:ty,) => {
        impl<$($ty_params,)* QS> $crate::expression::SelectableExpression<QS>
            for $struct_ty where
                $struct_ty: $crate::expression::AppearsOnTable<QS>,
                $($ty_params: $crate::expression::SelectableExpression<QS>,)*
        {
        }

        impl<$($ty_params,)* QS> $crate::expression::AppearsOnTable<QS>
            for $struct_ty where
                $struct_ty: $crate::expression::Expression,
                $($ty_params: $crate::expression::AppearsOnTable<QS>,)*
        {
        }
    };
}

/// Parses a sequence of type parameters and their bounds.
///
/// Ideally we would just be able to write this as
/// `$($param:ident $(: $bound:ty)*),* $(,)*`, but Rust doesn't allow a `ty`
/// fragment in a trait bound position.
///
/// Assuming we don't care about lifetimes or existentials, we could also
/// possibly write `$($param:ident $(: $bound:path)+*),* $(,)*` but Rust doesn't
/// allow `+` as a separator. In fact, we can't even use the `path` fragment
/// at all, as Rust does not allow it to be followed by `+` or `tt`.
///
/// This macro takes three arguments.
///
/// - `data`: Any arbitrary token tree you'd like given back to you when parsing
///   is complete.
/// - `callback`: The name of the macro to call when parsing is complete
/// - `tokens`: The tokens to be parsed.
///
/// This macro will consume tokens until it reaches a `>` that was not
/// preceded by a `<`. It will then invoke `callback` with 4 arguments:
///
/// - `data`: Whatever you gave this macro.
/// - `type_args`: The names of the type parameters. Always contains a trailing
///   comma if non-empty.
/// - `type_args_with_bounds`: The arguments and their bounds. Always contains
///   a trailing comma if non-empty
/// - `unparsed_tokens`: Any tokens that were not consumed by this macro.
#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_parse_type_args {
    // ENTRYPOINT
    //
    // Set up our lists.
    // Continues to NEXT_ARG, NEXT_ARG_NO_BOUNDS, or EXIT
    (
        data = $data:tt,
        callback = $callback:ident,
        tokens = $tokens:tt,
    ) => {
        __diesel_parse_type_args! {
            data = $data,
            callback = $callback,
            args = (),
            bounds = (),
            tokens = $tokens,
        }
    };

    // NEXT_ARG
    //
    // No arg currently being parsed, next token is an argument with a `:`.
    // From here we set up a stack of `<` to track,
    // and start putting tokens into the bounds.
    //
    // Continues to BOUNDS or OPENING_BRACKET
    (
        data = $data:tt,
        callback = $callback:ident,
        args = ($($args:tt)*),
        bounds = ($($bounds:tt)*),
        tokens = ($next_arg:ident : $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            brackets = (),
            data = $data,
            callback = $callback,
            args = ($($args)* $next_arg,),
            bounds = ($($bounds)* $next_arg:),
            tokens = ($($tokens)*),
        }
    };

    // NEXT_ARG_NO_BOUNDS
    //
    // No arg currently being parsed, next token is an argument without a :.
    //
    // Continues to NEXT_ARG, NEXT_ARG_NO_BOUNDS, NO_ARG_COMMA, or EXIT
    (
        data = $data:tt,
        callback = $callback:ident,
        args = ($($args:tt)*),
        bounds = ($($bounds:tt)*),
        tokens = ($next_arg:ident $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            data = $data,
            callback = $callback,
            args = ($($args)* $next_arg,),
            bounds = ($($bounds)* $next_arg,),
            tokens = ($($tokens)*),
        }
    };

    // FINAL_CLOSING_BRACKET
    //
    // > encountered, no bracket on the stack. We're done.
    //
    // Continues to EXIT
    (
        brackets = (),
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = ($($bounds:tt)*),
        tokens = (> $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            data = $data,
            callback = $callback,
            args = $args,
            bounds = ($($bounds)*,),
            tokens = (> $($tokens)*),
        }
    };

    // END_OF_BOUND
    //
    // , encountered, no bracket on the stack. We're done.
    //
    // Continues to NEXT_ARG, NEXT_ARG_NO_BOUNDS, or EXIT
    (
        brackets = (),
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = ($($bounds:tt)*),
        tokens = (, $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            data = $data,
            callback = $callback,
            args = $args,
            bounds = ($($bounds)* ,),
            tokens = ($($tokens)*),
        }
    };

    // OPENING_BRACKET
    //
    // Token is <, push it on the stack.
    //
    // Continues to BOUNDS, OPENING_BRACKET, CLOSING_BRACKET, or
    // DOUBLE_CLOSING_BRACKET.
    (
        brackets = ($($brackets:tt)*),
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = ($($bounds:tt)*),
        tokens = (< $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            brackets = (< $($brackets)*),
            data = $data,
            callback = $callback,
            args = $args,
            bounds = ($($bounds)* <),
            tokens = ($($tokens)*),
        }
    };

    // DOUBLE_CLOSING_BRACKET
    //
    // Rust treats >> as a single token, so we have to special case it.
    //
    // Continues to CLOSING_BRACKET or FINAL_CLOSING_BRACKET.
    (
        brackets = (< $($brackets:tt)*),
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = ($($bounds:tt)*),
        tokens = (>> $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            brackets = ($($brackets)*),
            data = $data,
            callback = $callback,
            args = $args,
            bounds = ($($bounds)* >),
            tokens = (> $($tokens)*),
        }
    };

    // CLOSING_BRACKET
    //
    // Token is >, and we have a non-empty bracket stack.
    // Pop it and continue.
    //
    // Continues to BOUNDS, OPENING_BRACKET, CLOSING_BRACKET,
    // DOUBLE_CLOSING_BRACKET, FINAL_CLOSING_BRACKET, or END_OF_BOUND
    (
        brackets = (< $($brackets:tt)*),
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = ($($bounds:tt)*),
        tokens = (> $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            brackets = ($($brackets)*),
            data = $data,
            callback = $callback,
            args = $args,
            bounds = ($($bounds)* >),
            tokens = ($($tokens)*),
        }
    };

    // BOUNDS
    //
    // Token is not a , or >. It's part of the trait bounds.
    //
    // Continues to BOUNDS, OPENING_BRACKET, CLOSING_BRACKET,
    // DOUBLE_CLOSING_BRACKET, FINAL_CLOSING_BRACKET, or END_OF_BOUND.
    (
        brackets = $brackets:tt,
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = ($($bounds:tt)*),
        tokens = ($token:tt $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            brackets = $brackets,
            data = $data,
            callback = $callback,
            args = $args,
            bounds = ($($bounds)* $token),
            tokens = ($($tokens)*),
        }
    };

    // NO_ARG_COMMA
    //
    // No arg currently being parsed, , encountered. Skip.
    //
    // Continues to NEXT_ARG, NEXT_ARG_NO_BOUNDS, or EXIT
    (
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = $bounds:tt,
        tokens = (, $($tokens:tt)*),
    ) => {
        __diesel_parse_type_args! {
            data = $data,
            callback = $callback,
            args = $args,
            bounds = $bounds,
            tokens = ($($tokens)*),
        }
    };

    // EXIT
    //
    // No arg currently being parsed, > encountered.
    (
        data = $data:tt,
        callback = $callback:ident,
        args = $args:tt,
        bounds = $bounds:tt,
        tokens = (> $($tokens:tt)*),
    ) => {
        $callback! {
            data = $data,
            type_args = $args,
            type_args_with_bounds = $bounds,
            unparsed_tokens = ($($tokens)*),
        }
    };
}

#[test]
fn parse_type_args_empty() {
    let expected = stringify!(
        data = (),
        type_args = (),
        type_args_with_bounds = (),
        unparsed_tokens = (),
    );
    let actual = __diesel_parse_type_args! {
        data = (),
        callback = stringify,
        tokens = (>),
    };

    assert_eq!(expected, actual);
}

#[test]
fn parse_type_args_simple() {
    let expected = stringify!(
        data = (foo),
        type_args = (T,),
        type_args_with_bounds = (T,),
        unparsed_tokens = (),
    );
    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T>),
    };

    assert_eq!(expected, actual);

    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T,>),
    };

    assert_eq!(expected, actual);
}

#[test]
fn parse_type_args_multiple() {
    let expected = stringify!(
        data = (foo),
        type_args = (T, U,),
        type_args_with_bounds = (T, U,),
        unparsed_tokens = (),
    );
    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T,U>),
    };

    assert_eq!(expected, actual);

    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T,U,>),
    };

    assert_eq!(expected, actual);
}

#[test]
fn parse_type_args_with_bounds() {
    let expected = stringify!(
        data = (foo),
        type_args = (T, U,),
        type_args_with_bounds = (T: Foo + Bar, U: Baz,),
        unparsed_tokens = (),
    );
    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T: Foo + Bar, U: Baz>),
    };

    assert_eq!(expected, actual);

    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T: Foo + Bar, U: Baz,>),
    };

    assert_eq!(expected, actual);
}

#[test]
fn parse_type_args_with_bounds_containing_braces_and_commas() {
    let expected = stringify!(
        data = (foo),
        type_args = (T,U,),
        type_args_with_bounds = (T: Foo<X> + Bar<U, V>,U: Baz<Vec<i32>, String>,),
        unparsed_tokens = (),
    );
    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T: Foo<X> + Bar<U, V>,U: Baz<Vec<i32>, String>>),
    };

    assert_eq!(expected, actual);

    let actual = __diesel_parse_type_args! {
        data = (foo),
        callback = stringify,
        tokens = (T: Foo<X> + Bar<U, V>,U: Baz<Vec<i32>, String>,>),
    };

    assert_eq!(expected, actual);
}

#[test]
fn parse_type_args_with_trailer() {
    let expected = stringify!(
        data = (
            meta = (#[aggregate]),
            fn_name = max,
        ),
        type_args = (ST,),
        type_args_with_bounds = (ST: SqlOrd + IntoNullable,),
        unparsed_tokens = ((expr: ST) -> ST::Nullable),
    );
    let actual = __diesel_parse_type_args! {
        data = (
            meta = (#[aggregate]),
            fn_name = max,
        ),
        callback = stringify,
        tokens = (ST: SqlOrd + IntoNullable>(expr: ST) -> ST::Nullable),
    };

    assert_eq!(expected, actual);
}

#[test]
fn parse_type_args_with_existentials_and_lifetimes() {
    let expected = stringify! (
        data = (),
        type_args = (ST,U,),
        type_args_with_bounds = (ST: for<'a> Foo<'a, U> + 'static, U,),
        unparsed_tokens = (),
    );
    let actual = __diesel_parse_type_args! {
        data = (),
        callback = stringify,
        tokens = (ST: for<'a> Foo<'a, U> + 'static, U>),
    };

    assert_eq!(expected, actual);
}
