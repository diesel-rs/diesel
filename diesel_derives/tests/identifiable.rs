use diesel::associations::Identifiable;

table! {
    foos {
        id -> Integer,
    }
}

table! {
    bars {
        id -> VarChar,
    }
}

#[test]
fn derive_identifiable_on_simple_struct() {
    #[derive(Identifiable)]
    struct Foo {
        id: i32,
        #[allow(dead_code)]
        foo: i32,
    }

    let foo1 = Foo { id: 1, foo: 2 };
    let foo2 = Foo { id: 2, foo: 3 };
    assert_eq!(&1, foo1.id());
    assert_eq!(&2, foo2.id());
}

#[test]
fn derive_identifiable_on_tuple_struct() {
    #[derive(Identifiable)]
    struct Foo(
        #[diesel(column_name = id)] i32,
        #[allow(dead_code)]
        #[diesel(column_name = lol)]
        i32,
    );

    let foo1 = Foo(1, 2);
    let foo2 = Foo(2, 3);
    assert_eq!(&1, foo1.id());
    assert_eq!(&2, foo2.id());
}

#[test]
fn derive_identifiable_when_id_is_not_first_field() {
    #[derive(Identifiable)]
    struct Foo {
        #[allow(dead_code)]
        foo: i32,
        id: i32,
    }

    let foo1 = Foo { id: 1, foo: 2 };
    let foo2 = Foo { id: 2, foo: 3 };
    assert_eq!(&1, foo1.id());
    assert_eq!(&2, foo2.id());
}

#[test]
fn derive_identifiable_on_struct_with_non_integer_pk() {
    #[derive(Identifiable)]
    #[diesel(table_name = bars)]
    struct Foo {
        id: &'static str,
        #[allow(dead_code)]
        foo: i32,
    }

    let foo1 = Foo { id: "hi", foo: 2 };
    let foo2 = Foo {
        id: "there",
        foo: 3,
    };
    assert_eq!(&"hi", foo1.id());
    assert_eq!(&"there", foo2.id());
}

#[test]
fn derive_identifiable_on_struct_with_lifetime() {
    #[derive(Identifiable)]
    #[diesel(table_name = bars)]
    struct Foo<'a> {
        id: &'a str,
        #[allow(dead_code)]
        foo: i32,
    }

    let foo1 = Foo { id: "hi", foo: 2 };
    let foo2 = Foo {
        id: "there",
        foo: 3,
    };
    assert_eq!(&"hi", foo1.id());
    assert_eq!(&"there", foo2.id());
}

#[test]
fn derive_identifiable_with_non_standard_pk() {
    #[allow(dead_code)]
    #[derive(Identifiable)]
    #[diesel(table_name = bars)]
    #[diesel(primary_key(foo_id))]
    struct Foo<'a> {
        id: i32,
        foo_id: &'a str,
        foo: i32,
    }

    let foo1 = Foo {
        id: 1,
        foo_id: "hi",
        foo: 2,
    };
    let foo2 = Foo {
        id: 2,
        foo_id: "there",
        foo: 3,
    };
    assert_eq!(&"hi", foo1.id());
    assert_eq!(&"there", foo2.id());
}

#[test]
fn derive_identifiable_with_composite_pk() {
    #[allow(dead_code)]
    #[derive(Identifiable)]
    #[diesel(table_name = bars)]
    #[diesel(primary_key(foo_id, bar_id))]
    struct Foo {
        id: i32,
        foo_id: i32,
        bar_id: i32,
        foo: i32,
    }

    let foo1 = Foo {
        id: 1,
        foo_id: 2,
        bar_id: 3,
        foo: 4,
    };
    let foo2 = Foo {
        id: 5,
        foo_id: 6,
        bar_id: 7,
        foo: 8,
    };
    assert_eq!((&2, &3), foo1.id());
    assert_eq!((&6, &7), foo2.id());
}
