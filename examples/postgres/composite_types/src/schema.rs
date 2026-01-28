// @generated automatically by Diesel CLI.

diesel::table! {
    colors (color_id) {
        color_id -> Int4,
        red -> Int4,
        green -> Int4,
        blue -> Int4,
        color_name -> Nullable<Text>,
    }
}

diesel::table! {
    coordinates (coord_id) {
        coord_id -> Int4,
        xcoord -> Int4,
        ycoord -> Int4,
    }
}

diesel::allow_tables_to_appear_in_same_query!(colors, coordinates,);
