#![allow(dead_code)] // this is a compile pass test
use diesel::dsl::*;
use diesel::helper_types::*;
use diesel::prelude::*;
use diesel::sql_types;
#[cfg(feature = "postgres")]
use std::ops::Bound;

table! {
    users {
        id -> Integer,
        name -> Text,
        time -> Timestamp,
        bigint -> BigInt,
        numeric -> Numeric,
        date -> Date,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
    }
}

table! {
    posts2 {
        id -> Integer,
        user_id -> Integer,
    }
}

table! {
    posts3 {
        id -> Integer,
        user_id -> Integer,
    }
}

#[cfg(feature = "postgres")]
table! {
    pg_extras(id) {
        id -> Integer,
        json -> Json,
        jsonb -> Jsonb,
        net -> Inet,
        array -> Array<Integer>,
        blob -> Binary,
        timestamp -> Timestamp,
        range -> Range<Integer>,
        multirange -> Multirange<Integer>,
        timestamptz -> Timestamptz,
        name -> Text,
        text_array -> Array<Text>,
        record -> Record<(Integer, Text, Date)>,
        boolean -> Bool,
    }
}

#[cfg(feature = "sqlite")]
table! {
    sqlite_extras {
        id -> Integer,
        text -> Text,
        blob -> Binary,
        json -> Json,
        jsonb -> Jsonb,
    }
}

joinable!(posts -> users(user_id));
joinable!(posts2 -> users(user_id));
joinable!(posts3 -> users(user_id));
allow_tables_to_appear_in_same_query!(users, posts, posts2, posts3);

#[auto_type]
fn test_all_query_dsl() -> _ {
    users::table
        .distinct()
        .filter(users::id.eq(42_i32))
        .find(42_i32)
        .or_filter(users::id.eq(42_i32))
        .limit(23_i64)
        .offset(12_i64)
        .order(users::id)
        .order_by(users::id)
        .then_order_by(users::id)
        .select(users::id)
        .group_by(users::id)
        .having(users::id.eq(32_i32))
        .inner_join(posts::table)
        .left_join(posts2::table)
        .inner_join(posts3::table.on(users::id.eq(posts3::user_id)))
    //.into_boxed()
}

#[auto_type]
fn single_value() -> _ {
    users::table.select(users::id).find(42_i32).single_value()
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_distinct_on() -> _ {
    users::table.distinct_on(users::id)
}

#[auto_type]
fn test_lock_dsl1() -> _ {
    users::table.for_key_share().no_wait().skip_locked()
}

#[auto_type]
fn test_lock_dsl2() -> _ {
    users::table.for_no_key_update()
}

#[auto_type]
fn test_lock_dsl3() -> _ {
    users::table.for_share()
}

#[auto_type]
fn test_lock_dsl4() -> _ {
    users::table.for_update()
}

// #[auto_type]
// fn test_count_query() -> _ {
//     users::table.find(1_i32).count()
// }

#[auto_type]
fn test_expression_methods() -> _ {
    let v = 42_i32;
    let v2: &'static [i32] = &[42];
    users::id
        .eq(v)
        .and(users::id.ne(v))
        .and(users::id.eq_any(v2))
        .and(users::id.ne_all(v2))
        .and(users::id.gt(v))
        .and(users::id.lt(v))
        .and(users::id.is_not_null())
        .and(users::id.is_null())
        .and(users::id.le(v))
        .and(users::id.ge(v))
        .and(users::id.between(v, v))
        .and(users::id.not_between(v, v))
}

#[auto_type]
fn test_boolean_expression_methods() -> _ {
    let v = 42_i32;
    users::id.eq(v).and(users::id.eq(v)).or(users::id.eq(v))
}

#[auto_type]
fn test_nullable_expression_methods() -> _ {
    users::id.nullable().assume_not_null()
}

#[auto_type]
fn test_text_expression_methods() -> _ {
    let a: &'static str = "foo";
    users::name
        .like(a)
        .and(users::name.not_like(a))
        .and(users::name.concat(a).eq(a))
}

#[auto_type]
fn test_delete() -> _ {
    delete(users::table)
}

#[auto_type]
fn test_delete_2() -> _ {
    delete(users::table.find({
        // Test that type ascriptions via nested blocks work
        let id: i32 = 1;
        id
    }))
}

#[auto_type]
fn test_delete_3() -> _ {
    delete(users::table).filter(users::id.eq(1_i32))
}

// #[auto_type]
// fn test_update() -> _ {
//     update(users::table).set(users::id.eq(42_i32))
// }

#[auto_type]
fn test_insert1() -> _ {
    insert_into(users::table).values(users::id.eq(42_i32))
}

/*#[auto_type]
fn test_insert2() -> _ {
    users::table
        .insert_into(users::table)
        .into_columns(users::all_columns)
}*/

#[auto_type]
fn test_insert_or_ignore() -> _ {
    insert_or_ignore_into(users::table).values(users::id.eq(42_i32))
}

#[auto_type]
fn test_insert_or_replace() -> _ {
    replace_into(users::table).values(users::id.eq(42_i32))
}

#[auto_type]
fn test_bare_select() -> _ {
    select(1_i32.into_sql::<sql_types::Integer>())
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_expression_methods() -> _ {
    let v = 42_i32;
    users::id
        .is_not_distinct_from(v)
        .and(users::id.is_distinct_from(v))
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_text_expression_methods() -> _ {
    let a: &'static str = "foo";
    users::name
        .ilike(a)
        .and(users::name.not_ilike(a))
        .and(users::name.similar_to(a))
        .and(users::name.not_similar_to(a))
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_net_expression_methods() -> _ {
    // cannot be supported on diesel 2.x as the contains operator for net
    // is different than the "normal" contains operator
    // We could probably rename this function to `contains_net` to make it work
    //pg_extras::net.contains(pg_extras::net)
    pg_extras::net
        .contains_or_eq(pg_extras::net)
        // cannot be supported on diesel 2.x due to similar reasons
        // as `contains`
        //.and(pg_extras::net.is_contained_by(pg_extras::net))
        .and(pg_extras::net.is_contained_by_or_eq(pg_extras::net))
        .and(pg_extras::net.overlaps_with(pg_extras::net))
        // `.and()` and `or()` for inet cannot be supported as that name collides
        // with `BoolExpressionMethods`
        //.and(pg_extras::net.and(pg_extras::net).contains_or_eq(pg_extras::net))
        //.and(pg_extras::net.or(pg_extras::net).contains(pg_extras::net))
        .and(pg_extras::net.diff(pg_extras::net).eq(42_i64))
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_array_expression_methods() -> _ {
    let v = 42_i32;
    pg_extras::array
        .overlaps_with(pg_extras::array)
        .and(pg_extras::array.contains(pg_extras::array))
        .and(pg_extras::array.is_contained_by(pg_extras::array))
        .and(pg_extras::array.index(v).eq(v))
        .and(
            pg_extras::array
                .concat(pg_extras::array)
                .eq(pg_extras::array),
        )
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_jsonb_expression_methods() -> _ {
    let s: &'static str = "";
    let v: &'static [&'static str] = &[];

    pg_extras::jsonb
        .concat(pg_extras::jsonb)
        .eq(pg_extras::jsonb)
        .and(pg_extras::jsonb.has_any_key(v))
        .and(pg_extras::jsonb.has_all_keys(v))
        .and(pg_extras::jsonb.has_key(s))
        .and(pg_extras::jsonb.contains(pg_extras::jsonb))
        .and(pg_extras::jsonb.remove(1_i32).eq(pg_extras::jsonb))
        .and(pg_extras::jsonb.remove_by_path(v).eq(pg_extras::jsonb))
        .and(pg_extras::jsonb.is_contained_by(pg_extras::jsonb))
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_range_expression_methods() -> _ {
    let my_range: (Bound<i32>, Bound<i32>) = (Bound::Included(2), Bound::Included(7));

    pg_extras::range
        .contains_range(my_range)
        .and(pg_extras::range.is_contained_by(my_range))
        .and(pg_extras::range.overlaps_with(my_range))
        .and(pg_extras::range.lesser_than(my_range))
        .and(pg_extras::range.greater_than(my_range))
        .and(pg_extras::range.range_extends_right_to(my_range))
        .and(pg_extras::range.range_extends_left_to(my_range))
        .and(pg_extras::id.is_contained_by_range(my_range))
        .and(
            pg_extras::range
                .union_range(pg_extras::range)
                .eq(pg_extras::range),
        )
        .and(
            pg_extras::range
                .difference_range(pg_extras::range)
                .eq(pg_extras::range),
        )
        .and(
            pg_extras::range
                .intersection_range(pg_extras::range)
                .eq(pg_extras::range),
        )
    // `.contains()` cannot be supported here as
    // the type level constraints are slightly different
    // for `Range<>` than for the other types that provide a `contains()`
    // function. We could likely support it by
    // renaming the function to `.range_contains()` (or something similar)
    // .contains(42_i32)
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_binary_expression_methods() -> _ {
    let b: &'static [u8] = &[];
    pg_extras::blob
        .concat(pg_extras::blob)
        .like(pg_extras::blob)
        .and(pg_extras::blob.not_like(b))
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_any_json_expression_methods() -> _ {
    let s: &'static str = "";
    let s2: &'static [&'static str] = &[];

    pg_extras::jsonb
        .retrieve_as_object(s)
        .retrieve_as_text(s)
        .eq(s)
        .and(
            pg_extras::jsonb
                .retrieve_by_path_as_object(s2)
                .retrieve_by_path_as_text(s2)
                .eq(s),
        )
        .and(pg_extras::name.is_json())
        .and(pg_extras::name.is_not_json())
        .and(pg_extras::name.is_json_object())
        .and(pg_extras::name.is_not_json_object())
        .and(pg_extras::name.is_json_array())
        .and(pg_extras::name.is_not_json_array())
        .and(pg_extras::name.is_json_scalar())
        .and(pg_extras::name.is_not_json_scalar())
}

#[cfg(feature = "postgres")]
#[auto_type]
fn test_pg_timestamp_expression_methods() -> _ {
    let s: &'static str = "";
    pg_extras::timestamp.at_time_zone(s)
}

#[cfg(feature = "sqlite")]
#[auto_type]
fn test_sqlite_expression_methods() -> _ {
    users::id.is(42_i32).or(users::id.is_not(42_i32))
}

#[auto_type]
fn test_aggregate_functions() -> _ {
    users::table.select((
        avg(users::id),
        count(users::id),
        count_distinct(users::id),
        count_star(),
        max(users::id),
        min(users::id),
        sum(users::id),
    ))
}

#[auto_type]
fn test_normal_functions() -> _ {
    users::table.select((
        date(users::time),
        exists(posts::table.select(posts::id)),
        not(users::id.eq(1_i32)),
        case_when(users::id.eq(1_i32), users::id),
        case_when(users::id.eq(1_i32), users::id).when(users::id.eq(42_i32), users::id),
        case_when(users::id.eq(1_i32), users::id)
            .when(users::id.eq(42_i32), users::id)
            .otherwise(users::id),
        case_when(users::id.eq(1_i32), users::id).otherwise(users::id),
    ))
}

#[cfg(feature = "postgres")]
#[auto_type]
fn postgres_functions() -> _ {
    let bound: sql_types::RangeBound =
        sql_types::RangeBound::LowerBoundExclusiveUpperBoundExclusive;
    let null_value_treatment: sql_types::NullValueTreatment =
        sql_types::NullValueTreatment::UseJsonNull;
    (
        lower(pg_extras::range),
        upper(pg_extras::range),
        isempty(pg_extras::range),
        lower_inc(pg_extras::range),
        upper_inc(pg_extras::range),
        lower_inf(pg_extras::range),
        upper_inf(pg_extras::range),
        range_merge(pg_extras::range, pg_extras::range),
        multirange_merge(pg_extras::multirange),
        int4range(users::id.nullable(), users::id.nullable(), bound),
        int8range(users::bigint.nullable(), users::bigint.nullable(), bound),
        numrange(users::numeric.nullable(), users::numeric.nullable(), bound),
        daterange(users::date.nullable(), users::date.nullable(), bound),
        tsrange(users::time.nullable(), users::time.nullable(), bound),
        tstzrange(
            pg_extras::timestamptz.nullable(),
            pg_extras::timestamptz.nullable(),
            bound,
        ),
        array_append(pg_extras::array, pg_extras::id),
        array_replace(pg_extras::array, pg_extras::id, pg_extras::id),
        array_dims(pg_extras::array),
        array_prepend(pg_extras::id, pg_extras::array),
        array_remove(pg_extras::array, pg_extras::id),
        array_to_string(pg_extras::array, pg_extras::name),
        array_to_string_with_null_string(pg_extras::array, pg_extras::name, pg_extras::name),
        cardinality(pg_extras::array),
        trim_array(pg_extras::array, pg_extras::id),
        array_cat(pg_extras::array, pg_extras::array),
        array_length(pg_extras::array, 1_i32),
        array_fill(pg_extras::id, pg_extras::array),
        array_fill_with_lower_bound(pg_extras::id, pg_extras::array, pg_extras::array),
        array_lower(pg_extras::array, 1_i32),
        array_upper(pg_extras::array, 1_i32),
        array_position(pg_extras::array, pg_extras::id),
        array_position_with_subscript(pg_extras::array, pg_extras::id, pg_extras::id),
        array_positions(pg_extras::array, pg_extras::id),
        array_ndims(pg_extras::array),
        array_shuffle(pg_extras::array),
        array_sample(pg_extras::array, pg_extras::id),
        array_to_json(pg_extras::array),
        to_json(pg_extras::id),
        to_jsonb(pg_extras::id),
        json_object(pg_extras::text_array),
        json_object_with_keys_and_values(pg_extras::text_array, pg_extras::text_array),
        json_typeof(pg_extras::json),
        jsonb_typeof(pg_extras::jsonb),
        jsonb_pretty(pg_extras::jsonb),
        json_strip_nulls(pg_extras::json),
        jsonb_strip_nulls(pg_extras::jsonb),
        json_array_length(pg_extras::json),
        jsonb_array_length(pg_extras::jsonb),
        jsonb_object(pg_extras::text_array),
        jsonb_object_with_keys_and_values(pg_extras::text_array, pg_extras::text_array),
        row_to_json(pg_extras::record),
        json_populate_record(pg_extras::record, pg_extras::json),
        jsonb_populate_record(pg_extras::record, pg_extras::jsonb),
        jsonb_set(pg_extras::jsonb, pg_extras::text_array, pg_extras::jsonb),
        jsonb_set_create_if_missing(
            pg_extras::jsonb,
            pg_extras::text_array,
            pg_extras::jsonb,
            pg_extras::boolean,
        ),
        jsonb_set_lax(
            pg_extras::jsonb,
            pg_extras::text_array,
            pg_extras::jsonb,
            pg_extras::boolean,
            null_value_treatment,
        ),
        jsonb_insert(pg_extras::jsonb, pg_extras::text_array, pg_extras::jsonb),
        jsonb_insert_with_insert_after(
            pg_extras::jsonb,
            pg_extras::text_array,
            pg_extras::jsonb,
            pg_extras::boolean,
        ),
    )
}

#[cfg(feature = "sqlite")]
#[auto_type]
fn sqlite_functions() -> _ {
    (
        json(sqlite_extras::text),
        jsonb(sqlite_extras::blob),
        json_pretty(sqlite_extras::json),
        json_pretty(sqlite_extras::jsonb),
        json_pretty_with_indentation(sqlite_extras::json, "  "),
        json_pretty_with_indentation(sqlite_extras::jsonb, "  "),
        json_valid(sqlite_extras::jsonb),
        json_valid(sqlite_extras::json),
    )
}

#[auto_type]
fn with_lifetime<'a>(name: &'a str) -> _ {
    users::table.filter(users::name.eq(name))
}

#[auto_type]
fn with_type_generics<'a, T>(name: &'a T) -> _
where
    &'a T: diesel::expression::AsExpression<diesel::sql_types::Text>,
{
    users::name.eq(name)
}

#[auto_type]
fn with_const_generics<const N: i32>() -> _ {
    users::id.eq(N)
}

#[auto_type]
fn insert_returning() -> _ {
    insert_into(users::table)
        .values(users::id.eq(42_i32))
        .returning(users::id)
}

#[auto_type]
fn delete_returning() -> _ {
    delete(users::table).returning(users::id)
}

#[cfg(feature = "postgres")]
#[auto_type]
fn update_and_binary_operator_and_block() -> _ {
    update(pg_extras::table).set(pg_extras::timestamp.eq(pg_extras::timestamp + {
        let v: diesel::data_types::PgInterval = 1.year();
        v
    }))
}

#[auto_type]
fn count_query() -> _ {
    users::table.count()
}

// #[auto_type]
// fn test_sql_fragment() -> _ {
//     sql("foo")
// }

// #[auto_type]
// fn test_sql_query_1() -> _ {
//     sql_query("bar")
// }

// #[auto_type]
// fn test_sql_query_2() -> _ {
//     sql_query("bar").bind::<Integer, _>(1)
// }
