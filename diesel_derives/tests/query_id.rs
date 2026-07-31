use core::any::TypeId;
use diesel::query_builder::QueryId;

#[derive(QueryId)]
struct Borrowed<'a> {
    _value: &'a str,
}

#[derive(QueryId)]
struct BoundedLifetimes<'a: 'b, 'b> {
    _outer: &'a str,
    _inner: &'b str,
}

#[derive(QueryId)]
struct Mixed<'a, T> {
    _value: &'a str,
    _other: T,
}

#[derive(QueryId)]
struct MarkerA;

#[derive(QueryId)]
struct MarkerB;

fn query_id_of<T: QueryId>(_: &T) -> Option<TypeId> {
    T::query_id()
}

#[test]
fn a_lifetime_does_not_change_the_query_id() {
    let owned = String::from("borrowed for a short time");
    let short = Borrowed { _value: &owned };
    let long = Borrowed {
        _value: "borrowed for the whole program",
    };

    // The two values hold different lifetimes but render the same SQL, so they must
    // share a prepared statement.
    assert_eq!(query_id_of(&short), query_id_of(&long));
    assert_eq!(
        query_id_of(&short),
        Some(TypeId::of::<Borrowed<'static>>()),
        "the lifetime should be erased to 'static"
    );
}

#[test]
fn a_bounded_lifetime_is_erased_as_well() {
    let outer = String::from("outer");
    let inner = String::from("inner");
    let value = BoundedLifetimes {
        _outer: &outer,
        _inner: &inner,
    };

    assert_eq!(
        query_id_of(&value),
        Some(TypeId::of::<BoundedLifetimes<'static, 'static>>())
    );
}

#[test]
fn a_type_parameter_still_contributes_to_the_query_id() {
    let owned = String::from("value");
    let with_a = Mixed {
        _value: &owned,
        _other: MarkerA,
    };
    let with_b = Mixed {
        _value: &owned,
        _other: MarkerB,
    };

    assert_ne!(query_id_of(&with_a), query_id_of(&with_b));
}

#[derive(QueryId)]
struct ConstBeforeType<'a, const N: usize, T> {
    _value: &'a str,
    _other: T,
}

#[test]
fn generic_arguments_keep_their_declared_order() {
    let owned = String::from("value");
    let value: ConstBeforeType<'_, 3, MarkerA> = ConstBeforeType {
        _value: &owned,
        _other: MarkerA,
    };

    assert_eq!(
        query_id_of(&value),
        Some(TypeId::of::<ConstBeforeType<'static, 3, MarkerA>>())
    );
}

#[derive(QueryId)]
struct TypeParamBorrowedForALifetime<'a, T: 'a> {
    _value: &'a T,
}

#[derive(QueryId)]
struct ConstBetweenTypes<A, const N: usize, B> {
    _a: A,
    _n: [u8; N],
    _b: B,
}

#[derive(QueryId)]
enum BorrowedOrOwned<'a, T> {
    _Borrowed(&'a str),
    _Owned(T),
}

#[derive(QueryId)]
struct Defaulted<T = MarkerA, const N: usize = 3> {
    _value: T,
    _n: [u8; N],
}

#[test]
fn a_type_parameter_may_borrow_for_the_erased_lifetime() {
    // `T::QueryId` has to outlive the erased `'static`, which only holds because
    // `QueryId::QueryId` is bound by `Any`.
    let marker = MarkerA;
    let value = TypeParamBorrowedForALifetime { _value: &marker };

    assert_eq!(
        query_id_of(&value),
        Some(TypeId::of::<TypeParamBorrowedForALifetime<'static, MarkerA>>())
    );
}

#[test]
fn a_const_between_two_type_parameters_stays_put() {
    let value: ConstBetweenTypes<MarkerA, 2, MarkerB> = ConstBetweenTypes {
        _a: MarkerA,
        _n: [0, 0],
        _b: MarkerB,
    };

    assert_eq!(
        query_id_of(&value),
        Some(TypeId::of::<ConstBetweenTypes<MarkerA, 2, MarkerB>>())
    );
}

#[test]
fn an_enum_is_handled_like_a_struct() {
    let value: BorrowedOrOwned<'_, MarkerA> = BorrowedOrOwned::_Owned(MarkerA);

    assert_eq!(
        query_id_of(&value),
        Some(TypeId::of::<BorrowedOrOwned<'static, MarkerA>>())
    );
}

#[test]
fn defaulted_parameters_are_not_repeated_in_the_query_id() {
    let value: Defaulted = Defaulted {
        _value: MarkerA,
        _n: [0, 0, 0],
    };

    assert_eq!(query_id_of(&value), Some(TypeId::of::<Defaulted>()));
}
