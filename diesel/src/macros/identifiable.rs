/// Implements the [`Identifiable`][identifiable] trait for a given struct. This
/// macro should be called by copy/pasting the definition of the struct into it.
///
/// The struct must have a field called `id`, and the type of that field must be
/// `Copy`. This macro does not work with tuple structs.
///
/// [identifiable]: query_source/trait.Identifiable.html
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// Identifiable! {
///     struct User {
///         id: i32,
///         name: String,
///     }
/// }
/// # fn main() {}
/// ```
///
/// To avoid copying your struct definition, you can use the
/// [custom_derive crate][custom_derive].
///
/// [custom_derive]: https://crates.io/crates/custom_derive
///
/// ```ignore
/// custom_derive! {
///     #[derive(Identifiable)]
///     struct User {
///         id: i32,
///         name: String,
///     }
/// }
/// ```
macro_rules! Identifiable {
    // Strip empty argument list if given (Passed by custom_derive macro)
    (() $($body:tt)*) => {
        Identifiable! {
            $($body)*
        }
    };

    // Strip meta items, pub (if present) and struct from definition
    (
        $(#[$ignore:meta])*
        $(pub)* struct $($body:tt)*
    ) => {
        Identifiable! {
            $($body)*
        }
    };

    // We found the `id` field, return the final impl
    (
        (
            struct_ty = $struct_ty:ty,
        ),
        fields = [{
            field_name: id,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
        } $($fields:tt)*],
    ) => {
        impl $crate::associations::Identifiable for $struct_ty {
            type Id = $field_ty;

            fn id(&self) -> Self::Id {
                self.id
            }
        }
    };

    // Search for the `id` field and continue
    (
        (
            struct_ty = $struct_ty:ty,
        ),
        fields = [{
            field_name: $field_name:ident,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
        } $($fields:tt)*],
    ) => {
        Identifiable! {
            (struct_ty = $struct_ty,),
            fields = [$($fields)*],
        }
    };


    // Handle struct with no generics
    (
        $struct_name:ident
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_ty = $struct_name,
            ),
            callback = Identifiable,
            body = $body,
        }
    };
}

#[test]
fn derive_identifiable_on_simple_struct() {
    use associations::Identifiable;

    struct Foo {
        id: i32,
        #[allow(dead_code)]
        foo: i32,
    }

    Identifiable! {
        struct Foo {
            id: i32,
            foo: i32,
        }
    }

    let foo1 = Foo { id: 1, foo: 2 };
    let foo2 = Foo { id: 2, foo: 3 };
    assert_eq!(1, foo1.id());
    assert_eq!(2, foo2.id());
}

#[test]
fn derive_identifiable_when_id_is_not_first_field() {
    use associations::Identifiable;

    struct Foo {
        #[allow(dead_code)]
        foo: i32,
        id: i32,
    }

    Identifiable! {
        struct Foo {
            foo: i32,
            id: i32,
        }
    }

    let foo1 = Foo { id: 1, foo: 2 };
    let foo2 = Foo { id: 2, foo: 3 };
    assert_eq!(1, foo1.id());
    assert_eq!(2, foo2.id());
}

#[test]
fn derive_identifiable_on_struct_with_non_integer_pk() {
    use associations::Identifiable;

    struct Foo {
        id: &'static str,
        #[allow(dead_code)]
        foo: i32,
    }

    Identifiable! {
        struct Foo {
            id: &'static str,
            foo: i32,
        }
    }

    let foo1 = Foo { id: "hi", foo: 2 };
    let foo2 = Foo { id: "there", foo: 3 };
    assert_eq!("hi", foo1.id());
    assert_eq!("there", foo2.id());
}
