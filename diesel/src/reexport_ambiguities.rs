// This macro exists to workaround
// https://github.com/rust-lang/rust/issues/149845
//
// The underlying problem there is that for sql function
// diesel generates items in multiple namespaces (type and function)
// with the same name. We often want only to import from one namespace
// which can be done by shadowing the relevant items from the other
// namespace with a private item after the glob import.
//
// TODO: Fixme Diesel 3.0, resolve this ambiguity in the macro itself
macro_rules! make_proxy_mod {
    ($name:ident, $path:path) => {
        #[allow(hidden_glob_reexports, non_camel_case_types, dead_code)]
        mod $name {
            #[doc(inline)]
            pub use $path::*;

            // Drop unintended types
            type abbrev = ();
            type array_append = ();
            type array_cat = ();
            type array_dims = ();
            type array_fill_with_lower_bound = ();
            type array_fill = ();
            type array_length = ();
            type array_lower = ();
            type array_ndims = ();
            type array_position_with_subscript = ();
            type array_position = ();
            type array_positions = ();
            type array_prepend = ();
            type array_remove = ();
            type array_replace = ();
            type array_sample = ();
            type array_shuffle = ();
            type array_to_json = ();
            type array_to_string_with_null_string = ();
            type array_to_string = ();
            type array_upper = ();
            type avg = ();
            type broadcast = ();
            type cardinality = ();
            type daterange = ();
            type family = ();
            type first_value = ();
            type host = ();
            type hostmask = ();
            type inet_merge = ();
            type inet_same_family = ();
            type int4range = ();
            type int8range = ();
            type isempty = ();
            type json_array_length = ();
            type json_build_array_0 = ();
            type json_build_array_1 = ();
            type json_build_array_2 = ();
            type json_extract_path_1 = ();
            type json_extract_path_2 = ();
            type json_extract_path_text_1 = ();
            type json_extract_path_text_2 = ();
            type json_object_with_keys_and_values = ();
            type json_object = ();
            type json_populate_record = ();
            type json_strip_nulls = ();
            type json_typeof = ();
            type jsonb_array_length = ();
            type jsonb_build_array_0 = ();
            type jsonb_build_array_1 = ();
            type jsonb_build_array_2 = ();
            type jsonb_extract_path_1 = ();
            type jsonb_extract_path_2 = ();
            type jsonb_extract_path_text_1 = ();
            type jsonb_extract_path_text_2 = ();
            type jsonb_insert_with_insert_after = ();
            type jsonb_insert = ();
            type jsonb_object_with_keys_and_values = ();
            type jsonb_object = ();
            type jsonb_populate_record = ();
            type jsonb_pretty = ();
            type jsonb_set_create_if_missing = ();
            type jsonb_set_lax = ();
            type jsonb_set = ();
            type jsonb_strip_nulls = ();
            type jsonb_typeof = ();
            type lag_with_offset_and_default = ();
            type lag_with_offset = ();
            type lag = ();
            type last_value = ();
            type lead_with_offset_and_default = ();
            type lead_with_offset = ();
            type lead = ();
            type lower_inc = ();
            type lower_inf = ();
            type lower = ();
            type masklen = ();
            type max = ();
            type min = ();
            type multirange_merge = ();
            type netmask = ();
            type network = ();
            type nth_value = ();
            type numrange = ();
            type range_merge = ();
            type row_to_json = ();
            type set_masklen = ();
            type sum = ();
            type to_json = ();
            type to_jsonb = ();
            type trim_array = ();
            type tsrange = ();
            type tstzrange = ();
            type upper_inc = ();
            type upper_inf = ();
            type upper = ();

            // sqlite
            type json = ();
            type json_array_0 = ();
            type json_array_1 = ();
            type json_array_2 = ();
            type json_array_length_with_path = ();
            type json_error_position = ();
            type json_group_array = ();
            type json_group_object = ();
            type json_object_0 = ();
            type json_object_1 = ();
            type json_object_2 = ();
            type json_patch = ();
            type json_pretty = ();
            type json_pretty_with_indentation = ();
            type json_quote = ();
            type json_remove_0 = ();
            type json_remove_1 = ();
            type json_remove_2 = ();
            type json_type = ();
            type json_type_with_path = ();
            type json_valid = ();
            type json_valid_with_flags = ();
            type jsonb = ();
            type jsonb_array_0 = ();
            type jsonb_array_1 = ();
            type jsonb_array_2 = ();
            type jsonb_group_array = ();
            type jsonb_group_object = ();
            type jsonb_object_0 = ();
            type jsonb_object_1 = ();
            type jsonb_object_2 = ();
            type jsonb_patch = ();
            type jsonb_remove_0 = ();
            type jsonb_remove_1 = ();
            type jsonb_remove_2 = ();
        }
    };
}
