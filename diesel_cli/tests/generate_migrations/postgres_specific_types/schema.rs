table! {
    users {
        id -> Integer,
        array -> Array<Nullable<Integer>>,
        record -> Record<(Integer, Text)>,
        range -> Range<Integer>,
        multirange -> Multirange<Integer>,
        timestamp -> Timestamp,
        timestamptz -> Timestamptz,

        nullable_array -> Nullable<Array<Nullable<Integer>>>,
        nullable_record -> Nullable<Record<(Integer, Text)>>,
        nullable_range -> Nullable<Range<Integer>>,
        nullable_multirange -> Nullable<Multirange<Integer>>,
        nullable_timestamp -> Nullable<Timestamp>,
        nullable_timestamptz -> Nullable<Timestamptz>,

        nested_1 -> Array<Range<Integer>>,
        nested_2 -> Array<Record<(Integer, Text)>>,
        nested_3 -> Array<Record<(Array<Integer>, Range<Integer>)>>,
        nested_4 -> Record<(Record<(Integer, Array<Text>)>, Nullable<Integer>)>,
        nested_5 -> Nullable<Array<Record<(Nullable<Array<Record<(Integer, Text)>>>, Integer)>>>,

        deep_nested -> Record<(Record<(Record<(Integer, Array<Record<(Integer, Text)>>)>, Array<Multirange<Integer>>)>, Integer)>,
    }
}
