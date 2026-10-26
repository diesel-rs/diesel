//! The SQLSTATE codes PostgreSQL defines, as listed in
//! [its documentation](https://www.postgresql.org/docs/current/errcodes-appendix.html)
//! and taken from
//! [`errcodes.txt`](https://github.com/postgres/postgres/blob/REL_18_0/src/backend/utils/errcodes.txt).
//!
//! Each constant is named after PostgreSQL's `ERRCODE_` macro and documented
//! with its PL/pgSQL condition name.
//!
//! ```rust
//! use diesel::pg::sql_state;
//! use diesel::result::SqlState;
//!
//! fn retryable(code: SqlState) -> bool {
//!     matches!(
//!         code,
//!         sql_state::T_R_SERIALIZATION_FAILURE | sql_state::T_R_DEADLOCK_DETECTED
//!     )
//! }
//!
//! assert!(retryable("40001".parse().unwrap()));
//! assert!(!retryable("23505".parse().unwrap()));
//! ```

use crate::result::SqlState;

macro_rules! codes {
    ($($(#[$doc:meta])* $name:ident = $code:literal;)*) => {
        $(
            $(#[$doc])*
            pub const $name: SqlState = match SqlState::new(*$code) {
                Some(code) => code,
                None => panic!("invalid SQLSTATE"),
            };
        )*
    };
}

codes! {
    // Class 00 - Successful Completion
    /// `00000`, `successful_completion`.
    SUCCESSFUL_COMPLETION = b"00000";
    // Class 01 - Warning
    /// `01000`, `warning`.
    WARNING = b"01000";
    /// `0100C`, `dynamic_result_sets_returned`.
    WARNING_DYNAMIC_RESULT_SETS_RETURNED = b"0100C";
    /// `01008`, `implicit_zero_bit_padding`.
    WARNING_IMPLICIT_ZERO_BIT_PADDING = b"01008";
    /// `01003`, `null_value_eliminated_in_set_function`.
    WARNING_NULL_VALUE_ELIMINATED_IN_SET_FUNCTION = b"01003";
    /// `01007`, `privilege_not_granted`.
    WARNING_PRIVILEGE_NOT_GRANTED = b"01007";
    /// `01006`, `privilege_not_revoked`.
    WARNING_PRIVILEGE_NOT_REVOKED = b"01006";
    /// `01004`, `string_data_right_truncation`.
    WARNING_STRING_DATA_RIGHT_TRUNCATION = b"01004";
    /// `01P01`, `deprecated_feature`.
    WARNING_DEPRECATED_FEATURE = b"01P01";
    // Class 02 - No Data (this is also a warning class per the SQL standard)
    /// `02000`, `no_data`.
    NO_DATA = b"02000";
    /// `02001`, `no_additional_dynamic_result_sets_returned`.
    NO_ADDITIONAL_DYNAMIC_RESULT_SETS_RETURNED = b"02001";
    // Class 03 - SQL Statement Not Yet Complete
    /// `03000`, `sql_statement_not_yet_complete`.
    SQL_STATEMENT_NOT_YET_COMPLETE = b"03000";
    // Class 08 - Connection Exception
    /// `08000`, `connection_exception`.
    CONNECTION_EXCEPTION = b"08000";
    /// `08003`, `connection_does_not_exist`.
    CONNECTION_DOES_NOT_EXIST = b"08003";
    /// `08006`, `connection_failure`.
    CONNECTION_FAILURE = b"08006";
    /// `08001`, `sqlclient_unable_to_establish_sqlconnection`.
    SQLCLIENT_UNABLE_TO_ESTABLISH_SQLCONNECTION = b"08001";
    /// `08004`, `sqlserver_rejected_establishment_of_sqlconnection`.
    SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION = b"08004";
    /// `08007`, `transaction_resolution_unknown`.
    TRANSACTION_RESOLUTION_UNKNOWN = b"08007";
    /// `08P01`, `protocol_violation`.
    PROTOCOL_VIOLATION = b"08P01";
    // Class 09 - Triggered Action Exception
    /// `09000`, `triggered_action_exception`.
    TRIGGERED_ACTION_EXCEPTION = b"09000";
    // Class 0A - Feature Not Supported
    /// `0A000`, `feature_not_supported`.
    FEATURE_NOT_SUPPORTED = b"0A000";
    // Class 0B - Invalid Transaction Initiation
    /// `0B000`, `invalid_transaction_initiation`.
    INVALID_TRANSACTION_INITIATION = b"0B000";
    // Class 0F - Locator Exception
    /// `0F000`, `locator_exception`.
    LOCATOR_EXCEPTION = b"0F000";
    /// `0F001`, `invalid_locator_specification`.
    L_E_INVALID_SPECIFICATION = b"0F001";
    // Class 0L - Invalid Grantor
    /// `0L000`, `invalid_grantor`.
    INVALID_GRANTOR = b"0L000";
    /// `0LP01`, `invalid_grant_operation`.
    INVALID_GRANT_OPERATION = b"0LP01";
    // Class 0P - Invalid Role Specification
    /// `0P000`, `invalid_role_specification`.
    INVALID_ROLE_SPECIFICATION = b"0P000";
    // Class 0Z - Diagnostics Exception
    /// `0Z000`, `diagnostics_exception`.
    DIAGNOSTICS_EXCEPTION = b"0Z000";
    /// `0Z002`, `stacked_diagnostics_accessed_without_active_handler`.
    STACKED_DIAGNOSTICS_ACCESSED_WITHOUT_ACTIVE_HANDLER = b"0Z002";
    // Class 10 - XQuery Error
    /// `10608`, `invalid_argument_for_xquery`.
    INVALID_ARGUMENT_FOR_XQUERY = b"10608";
    // Class 20 - Case Not Found
    /// `20000`, `case_not_found`.
    CASE_NOT_FOUND = b"20000";
    // Class 21 - Cardinality Violation
    /// `21000`, `cardinality_violation`.
    CARDINALITY_VIOLATION = b"21000";
    // Class 22 - Data Exception
    /// `22000`, `data_exception`.
    DATA_EXCEPTION = b"22000";
    /// `2202E`, `array_subscript_error`.
    ARRAY_SUBSCRIPT_ERROR = b"2202E";
    /// `22021`, `character_not_in_repertoire`.
    CHARACTER_NOT_IN_REPERTOIRE = b"22021";
    /// `22008`, `datetime_field_overflow`.
    DATETIME_FIELD_OVERFLOW = b"22008";
    /// `22012`, `division_by_zero`.
    DIVISION_BY_ZERO = b"22012";
    /// `22005`, `error_in_assignment`.
    ERROR_IN_ASSIGNMENT = b"22005";
    /// `2200B`, `escape_character_conflict`.
    ESCAPE_CHARACTER_CONFLICT = b"2200B";
    /// `22022`, `indicator_overflow`.
    INDICATOR_OVERFLOW = b"22022";
    /// `22015`, `interval_field_overflow`.
    INTERVAL_FIELD_OVERFLOW = b"22015";
    /// `2201E`, `invalid_argument_for_logarithm`.
    INVALID_ARGUMENT_FOR_LOG = b"2201E";
    /// `22014`, `invalid_argument_for_ntile_function`.
    INVALID_ARGUMENT_FOR_NTILE = b"22014";
    /// `22016`, `invalid_argument_for_nth_value_function`.
    INVALID_ARGUMENT_FOR_NTH_VALUE = b"22016";
    /// `2201F`, `invalid_argument_for_power_function`.
    INVALID_ARGUMENT_FOR_POWER_FUNCTION = b"2201F";
    /// `2201G`, `invalid_argument_for_width_bucket_function`.
    INVALID_ARGUMENT_FOR_WIDTH_BUCKET_FUNCTION = b"2201G";
    /// `22018`, `invalid_character_value_for_cast`.
    INVALID_CHARACTER_VALUE_FOR_CAST = b"22018";
    /// `22007`, `invalid_datetime_format`.
    INVALID_DATETIME_FORMAT = b"22007";
    /// `22019`, `invalid_escape_character`.
    INVALID_ESCAPE_CHARACTER = b"22019";
    /// `2200D`, `invalid_escape_octet`.
    INVALID_ESCAPE_OCTET = b"2200D";
    /// `22025`, `invalid_escape_sequence`.
    INVALID_ESCAPE_SEQUENCE = b"22025";
    /// `22P06`, `nonstandard_use_of_escape_character`.
    NONSTANDARD_USE_OF_ESCAPE_CHARACTER = b"22P06";
    /// `22010`, `invalid_indicator_parameter_value`.
    INVALID_INDICATOR_PARAMETER_VALUE = b"22010";
    /// `22023`, `invalid_parameter_value`.
    INVALID_PARAMETER_VALUE = b"22023";
    /// `22013`, `invalid_preceding_or_following_size`.
    INVALID_PRECEDING_OR_FOLLOWING_SIZE = b"22013";
    /// `2201B`, `invalid_regular_expression`.
    INVALID_REGULAR_EXPRESSION = b"2201B";
    /// `2201W`, `invalid_row_count_in_limit_clause`.
    INVALID_ROW_COUNT_IN_LIMIT_CLAUSE = b"2201W";
    /// `2201X`, `invalid_row_count_in_result_offset_clause`.
    INVALID_ROW_COUNT_IN_RESULT_OFFSET_CLAUSE = b"2201X";
    /// `2202H`, `invalid_tablesample_argument`.
    INVALID_TABLESAMPLE_ARGUMENT = b"2202H";
    /// `2202G`, `invalid_tablesample_repeat`.
    INVALID_TABLESAMPLE_REPEAT = b"2202G";
    /// `22009`, `invalid_time_zone_displacement_value`.
    INVALID_TIME_ZONE_DISPLACEMENT_VALUE = b"22009";
    /// `2200C`, `invalid_use_of_escape_character`.
    INVALID_USE_OF_ESCAPE_CHARACTER = b"2200C";
    /// `2200G`, `most_specific_type_mismatch`.
    MOST_SPECIFIC_TYPE_MISMATCH = b"2200G";
    /// `22004`, `null_value_not_allowed`.
    NULL_VALUE_NOT_ALLOWED = b"22004";
    /// `22002`, `null_value_no_indicator_parameter`.
    NULL_VALUE_NO_INDICATOR_PARAMETER = b"22002";
    /// `22003`, `numeric_value_out_of_range`.
    NUMERIC_VALUE_OUT_OF_RANGE = b"22003";
    /// `2200H`, `sequence_generator_limit_exceeded`.
    SEQUENCE_GENERATOR_LIMIT_EXCEEDED = b"2200H";
    /// `22026`, `string_data_length_mismatch`.
    STRING_DATA_LENGTH_MISMATCH = b"22026";
    /// `22001`, `string_data_right_truncation`.
    STRING_DATA_RIGHT_TRUNCATION = b"22001";
    /// `22011`, `substring_error`.
    SUBSTRING_ERROR = b"22011";
    /// `22027`, `trim_error`.
    TRIM_ERROR = b"22027";
    /// `22024`, `unterminated_c_string`.
    UNTERMINATED_C_STRING = b"22024";
    /// `2200F`, `zero_length_character_string`.
    ZERO_LENGTH_CHARACTER_STRING = b"2200F";
    /// `22P01`, `floating_point_exception`.
    FLOATING_POINT_EXCEPTION = b"22P01";
    /// `22P02`, `invalid_text_representation`.
    INVALID_TEXT_REPRESENTATION = b"22P02";
    /// `22P03`, `invalid_binary_representation`.
    INVALID_BINARY_REPRESENTATION = b"22P03";
    /// `22P04`, `bad_copy_file_format`.
    BAD_COPY_FILE_FORMAT = b"22P04";
    /// `22P05`, `untranslatable_character`.
    UNTRANSLATABLE_CHARACTER = b"22P05";
    /// `2200L`, `not_an_xml_document`.
    NOT_AN_XML_DOCUMENT = b"2200L";
    /// `2200M`, `invalid_xml_document`.
    INVALID_XML_DOCUMENT = b"2200M";
    /// `2200N`, `invalid_xml_content`.
    INVALID_XML_CONTENT = b"2200N";
    /// `2200S`, `invalid_xml_comment`.
    INVALID_XML_COMMENT = b"2200S";
    /// `2200T`, `invalid_xml_processing_instruction`.
    INVALID_XML_PROCESSING_INSTRUCTION = b"2200T";
    /// `22030`, `duplicate_json_object_key_value`.
    DUPLICATE_JSON_OBJECT_KEY_VALUE = b"22030";
    /// `22031`, `invalid_argument_for_sql_json_datetime_function`.
    INVALID_ARGUMENT_FOR_SQL_JSON_DATETIME_FUNCTION = b"22031";
    /// `22032`, `invalid_json_text`.
    INVALID_JSON_TEXT = b"22032";
    /// `22033`, `invalid_sql_json_subscript`.
    INVALID_SQL_JSON_SUBSCRIPT = b"22033";
    /// `22034`, `more_than_one_sql_json_item`.
    MORE_THAN_ONE_SQL_JSON_ITEM = b"22034";
    /// `22035`, `no_sql_json_item`.
    NO_SQL_JSON_ITEM = b"22035";
    /// `22036`, `non_numeric_sql_json_item`.
    NON_NUMERIC_SQL_JSON_ITEM = b"22036";
    /// `22037`, `non_unique_keys_in_a_json_object`.
    NON_UNIQUE_KEYS_IN_A_JSON_OBJECT = b"22037";
    /// `22038`, `singleton_sql_json_item_required`.
    SINGLETON_SQL_JSON_ITEM_REQUIRED = b"22038";
    /// `22039`, `sql_json_array_not_found`.
    SQL_JSON_ARRAY_NOT_FOUND = b"22039";
    /// `2203A`, `sql_json_member_not_found`.
    SQL_JSON_MEMBER_NOT_FOUND = b"2203A";
    /// `2203B`, `sql_json_number_not_found`.
    SQL_JSON_NUMBER_NOT_FOUND = b"2203B";
    /// `2203C`, `sql_json_object_not_found`.
    SQL_JSON_OBJECT_NOT_FOUND = b"2203C";
    /// `2203D`, `too_many_json_array_elements`.
    TOO_MANY_JSON_ARRAY_ELEMENTS = b"2203D";
    /// `2203E`, `too_many_json_object_members`.
    TOO_MANY_JSON_OBJECT_MEMBERS = b"2203E";
    /// `2203F`, `sql_json_scalar_required`.
    SQL_JSON_SCALAR_REQUIRED = b"2203F";
    /// `2203G`, `sql_json_item_cannot_be_cast_to_target_type`.
    SQL_JSON_ITEM_CANNOT_BE_CAST_TO_TARGET_TYPE = b"2203G";
    // Class 23 - Integrity Constraint Violation
    /// `23000`, `integrity_constraint_violation`.
    INTEGRITY_CONSTRAINT_VIOLATION = b"23000";
    /// `23001`, `restrict_violation`.
    RESTRICT_VIOLATION = b"23001";
    /// `23502`, `not_null_violation`.
    NOT_NULL_VIOLATION = b"23502";
    /// `23503`, `foreign_key_violation`.
    FOREIGN_KEY_VIOLATION = b"23503";
    /// `23505`, `unique_violation`.
    UNIQUE_VIOLATION = b"23505";
    /// `23514`, `check_violation`.
    CHECK_VIOLATION = b"23514";
    /// `23P01`, `exclusion_violation`.
    EXCLUSION_VIOLATION = b"23P01";
    // Class 24 - Invalid Cursor State
    /// `24000`, `invalid_cursor_state`.
    INVALID_CURSOR_STATE = b"24000";
    // Class 25 - Invalid Transaction State
    /// `25000`, `invalid_transaction_state`.
    INVALID_TRANSACTION_STATE = b"25000";
    /// `25001`, `active_sql_transaction`.
    ACTIVE_SQL_TRANSACTION = b"25001";
    /// `25002`, `branch_transaction_already_active`.
    BRANCH_TRANSACTION_ALREADY_ACTIVE = b"25002";
    /// `25008`, `held_cursor_requires_same_isolation_level`.
    HELD_CURSOR_REQUIRES_SAME_ISOLATION_LEVEL = b"25008";
    /// `25003`, `inappropriate_access_mode_for_branch_transaction`.
    INAPPROPRIATE_ACCESS_MODE_FOR_BRANCH_TRANSACTION = b"25003";
    /// `25004`, `inappropriate_isolation_level_for_branch_transaction`.
    INAPPROPRIATE_ISOLATION_LEVEL_FOR_BRANCH_TRANSACTION = b"25004";
    /// `25005`, `no_active_sql_transaction_for_branch_transaction`.
    NO_ACTIVE_SQL_TRANSACTION_FOR_BRANCH_TRANSACTION = b"25005";
    /// `25006`, `read_only_sql_transaction`.
    READ_ONLY_SQL_TRANSACTION = b"25006";
    /// `25007`, `schema_and_data_statement_mixing_not_supported`.
    SCHEMA_AND_DATA_STATEMENT_MIXING_NOT_SUPPORTED = b"25007";
    /// `25P01`, `no_active_sql_transaction`.
    NO_ACTIVE_SQL_TRANSACTION = b"25P01";
    /// `25P02`, `in_failed_sql_transaction`.
    IN_FAILED_SQL_TRANSACTION = b"25P02";
    /// `25P03`, `idle_in_transaction_session_timeout`.
    IDLE_IN_TRANSACTION_SESSION_TIMEOUT = b"25P03";
    /// `25P04`, `transaction_timeout`.
    TRANSACTION_TIMEOUT = b"25P04";
    // Class 26 - Invalid SQL Statement Name
    /// `26000`, `invalid_sql_statement_name`.
    INVALID_SQL_STATEMENT_NAME = b"26000";
    // Class 27 - Triggered Data Change Violation
    /// `27000`, `triggered_data_change_violation`.
    TRIGGERED_DATA_CHANGE_VIOLATION = b"27000";
    // Class 28 - Invalid Authorization Specification
    /// `28000`, `invalid_authorization_specification`.
    INVALID_AUTHORIZATION_SPECIFICATION = b"28000";
    /// `28P01`, `invalid_password`.
    INVALID_PASSWORD = b"28P01";
    // Class 2B - Dependent Privilege Descriptors Still Exist
    /// `2B000`, `dependent_privilege_descriptors_still_exist`.
    DEPENDENT_PRIVILEGE_DESCRIPTORS_STILL_EXIST = b"2B000";
    /// `2BP01`, `dependent_objects_still_exist`.
    DEPENDENT_OBJECTS_STILL_EXIST = b"2BP01";
    // Class 2D - Invalid Transaction Termination
    /// `2D000`, `invalid_transaction_termination`.
    INVALID_TRANSACTION_TERMINATION = b"2D000";
    // Class 2F - SQL Routine Exception
    /// `2F000`, `sql_routine_exception`.
    SQL_ROUTINE_EXCEPTION = b"2F000";
    /// `2F005`, `function_executed_no_return_statement`.
    S_R_E_FUNCTION_EXECUTED_NO_RETURN_STATEMENT = b"2F005";
    /// `2F002`, `modifying_sql_data_not_permitted`.
    S_R_E_MODIFYING_SQL_DATA_NOT_PERMITTED = b"2F002";
    /// `2F003`, `prohibited_sql_statement_attempted`.
    S_R_E_PROHIBITED_SQL_STATEMENT_ATTEMPTED = b"2F003";
    /// `2F004`, `reading_sql_data_not_permitted`.
    S_R_E_READING_SQL_DATA_NOT_PERMITTED = b"2F004";
    // Class 34 - Invalid Cursor Name
    /// `34000`, `invalid_cursor_name`.
    INVALID_CURSOR_NAME = b"34000";
    // Class 38 - External Routine Exception
    /// `38000`, `external_routine_exception`.
    EXTERNAL_ROUTINE_EXCEPTION = b"38000";
    /// `38001`, `containing_sql_not_permitted`.
    E_R_E_CONTAINING_SQL_NOT_PERMITTED = b"38001";
    /// `38002`, `modifying_sql_data_not_permitted`.
    E_R_E_MODIFYING_SQL_DATA_NOT_PERMITTED = b"38002";
    /// `38003`, `prohibited_sql_statement_attempted`.
    E_R_E_PROHIBITED_SQL_STATEMENT_ATTEMPTED = b"38003";
    /// `38004`, `reading_sql_data_not_permitted`.
    E_R_E_READING_SQL_DATA_NOT_PERMITTED = b"38004";
    // Class 39 - External Routine Invocation Exception
    /// `39000`, `external_routine_invocation_exception`.
    EXTERNAL_ROUTINE_INVOCATION_EXCEPTION = b"39000";
    /// `39001`, `invalid_sqlstate_returned`.
    E_R_I_E_INVALID_SQLSTATE_RETURNED = b"39001";
    /// `39004`, `null_value_not_allowed`.
    E_R_I_E_NULL_VALUE_NOT_ALLOWED = b"39004";
    /// `39P01`, `trigger_protocol_violated`.
    E_R_I_E_TRIGGER_PROTOCOL_VIOLATED = b"39P01";
    /// `39P02`, `srf_protocol_violated`.
    E_R_I_E_SRF_PROTOCOL_VIOLATED = b"39P02";
    /// `39P03`, `event_trigger_protocol_violated`.
    E_R_I_E_EVENT_TRIGGER_PROTOCOL_VIOLATED = b"39P03";
    // Class 3B - Savepoint Exception
    /// `3B000`, `savepoint_exception`.
    SAVEPOINT_EXCEPTION = b"3B000";
    /// `3B001`, `invalid_savepoint_specification`.
    S_E_INVALID_SPECIFICATION = b"3B001";
    // Class 3D - Invalid Catalog Name
    /// `3D000`, `invalid_catalog_name`.
    INVALID_CATALOG_NAME = b"3D000";
    // Class 3F - Invalid Schema Name
    /// `3F000`, `invalid_schema_name`.
    INVALID_SCHEMA_NAME = b"3F000";
    // Class 40 - Transaction Rollback
    /// `40000`, `transaction_rollback`.
    TRANSACTION_ROLLBACK = b"40000";
    /// `40002`, `transaction_integrity_constraint_violation`.
    T_R_INTEGRITY_CONSTRAINT_VIOLATION = b"40002";
    /// `40001`, `serialization_failure`.
    T_R_SERIALIZATION_FAILURE = b"40001";
    /// `40003`, `statement_completion_unknown`.
    T_R_STATEMENT_COMPLETION_UNKNOWN = b"40003";
    /// `40P01`, `deadlock_detected`.
    T_R_DEADLOCK_DETECTED = b"40P01";
    // Class 42 - Syntax Error or Access Rule Violation
    /// `42000`, `syntax_error_or_access_rule_violation`.
    SYNTAX_ERROR_OR_ACCESS_RULE_VIOLATION = b"42000";
    /// `42601`, `syntax_error`.
    SYNTAX_ERROR = b"42601";
    /// `42501`, `insufficient_privilege`.
    INSUFFICIENT_PRIVILEGE = b"42501";
    /// `42846`, `cannot_coerce`.
    CANNOT_COERCE = b"42846";
    /// `42803`, `grouping_error`.
    GROUPING_ERROR = b"42803";
    /// `42P20`, `windowing_error`.
    WINDOWING_ERROR = b"42P20";
    /// `42P19`, `invalid_recursion`.
    INVALID_RECURSION = b"42P19";
    /// `42830`, `invalid_foreign_key`.
    INVALID_FOREIGN_KEY = b"42830";
    /// `42602`, `invalid_name`.
    INVALID_NAME = b"42602";
    /// `42622`, `name_too_long`.
    NAME_TOO_LONG = b"42622";
    /// `42939`, `reserved_name`.
    RESERVED_NAME = b"42939";
    /// `42804`, `datatype_mismatch`.
    DATATYPE_MISMATCH = b"42804";
    /// `42P18`, `indeterminate_datatype`.
    INDETERMINATE_DATATYPE = b"42P18";
    /// `42P21`, `collation_mismatch`.
    COLLATION_MISMATCH = b"42P21";
    /// `42P22`, `indeterminate_collation`.
    INDETERMINATE_COLLATION = b"42P22";
    /// `42809`, `wrong_object_type`.
    WRONG_OBJECT_TYPE = b"42809";
    /// `428C9`, `generated_always`.
    GENERATED_ALWAYS = b"428C9";
    /// `42703`, `undefined_column`.
    UNDEFINED_COLUMN = b"42703";
    /// `42883`, `undefined_function`.
    UNDEFINED_FUNCTION = b"42883";
    /// `42P01`, `undefined_table`.
    UNDEFINED_TABLE = b"42P01";
    /// `42P02`, `undefined_parameter`.
    UNDEFINED_PARAMETER = b"42P02";
    /// `42704`, `undefined_object`.
    UNDEFINED_OBJECT = b"42704";
    /// `42701`, `duplicate_column`.
    DUPLICATE_COLUMN = b"42701";
    /// `42P03`, `duplicate_cursor`.
    DUPLICATE_CURSOR = b"42P03";
    /// `42P04`, `duplicate_database`.
    DUPLICATE_DATABASE = b"42P04";
    /// `42723`, `duplicate_function`.
    DUPLICATE_FUNCTION = b"42723";
    /// `42P05`, `duplicate_prepared_statement`.
    DUPLICATE_PSTATEMENT = b"42P05";
    /// `42P06`, `duplicate_schema`.
    DUPLICATE_SCHEMA = b"42P06";
    /// `42P07`, `duplicate_table`.
    DUPLICATE_TABLE = b"42P07";
    /// `42712`, `duplicate_alias`.
    DUPLICATE_ALIAS = b"42712";
    /// `42710`, `duplicate_object`.
    DUPLICATE_OBJECT = b"42710";
    /// `42702`, `ambiguous_column`.
    AMBIGUOUS_COLUMN = b"42702";
    /// `42725`, `ambiguous_function`.
    AMBIGUOUS_FUNCTION = b"42725";
    /// `42P08`, `ambiguous_parameter`.
    AMBIGUOUS_PARAMETER = b"42P08";
    /// `42P09`, `ambiguous_alias`.
    AMBIGUOUS_ALIAS = b"42P09";
    /// `42P10`, `invalid_column_reference`.
    INVALID_COLUMN_REFERENCE = b"42P10";
    /// `42611`, `invalid_column_definition`.
    INVALID_COLUMN_DEFINITION = b"42611";
    /// `42P11`, `invalid_cursor_definition`.
    INVALID_CURSOR_DEFINITION = b"42P11";
    /// `42P12`, `invalid_database_definition`.
    INVALID_DATABASE_DEFINITION = b"42P12";
    /// `42P13`, `invalid_function_definition`.
    INVALID_FUNCTION_DEFINITION = b"42P13";
    /// `42P14`, `invalid_prepared_statement_definition`.
    INVALID_PSTATEMENT_DEFINITION = b"42P14";
    /// `42P15`, `invalid_schema_definition`.
    INVALID_SCHEMA_DEFINITION = b"42P15";
    /// `42P16`, `invalid_table_definition`.
    INVALID_TABLE_DEFINITION = b"42P16";
    /// `42P17`, `invalid_object_definition`.
    INVALID_OBJECT_DEFINITION = b"42P17";
    // Class 44 - WITH CHECK OPTION Violation
    /// `44000`, `with_check_option_violation`.
    WITH_CHECK_OPTION_VIOLATION = b"44000";
    // Class 53 - Insufficient Resources
    /// `53000`, `insufficient_resources`.
    INSUFFICIENT_RESOURCES = b"53000";
    /// `53100`, `disk_full`.
    DISK_FULL = b"53100";
    /// `53200`, `out_of_memory`.
    OUT_OF_MEMORY = b"53200";
    /// `53300`, `too_many_connections`.
    TOO_MANY_CONNECTIONS = b"53300";
    /// `53400`, `configuration_limit_exceeded`.
    CONFIGURATION_LIMIT_EXCEEDED = b"53400";
    // Class 54 - Program Limit Exceeded
    /// `54000`, `program_limit_exceeded`.
    PROGRAM_LIMIT_EXCEEDED = b"54000";
    /// `54001`, `statement_too_complex`.
    STATEMENT_TOO_COMPLEX = b"54001";
    /// `54011`, `too_many_columns`.
    TOO_MANY_COLUMNS = b"54011";
    /// `54023`, `too_many_arguments`.
    TOO_MANY_ARGUMENTS = b"54023";
    // Class 55 - Object Not In Prerequisite State
    /// `55000`, `object_not_in_prerequisite_state`.
    OBJECT_NOT_IN_PREREQUISITE_STATE = b"55000";
    /// `55006`, `object_in_use`.
    OBJECT_IN_USE = b"55006";
    /// `55P02`, `cant_change_runtime_param`.
    CANT_CHANGE_RUNTIME_PARAM = b"55P02";
    /// `55P03`, `lock_not_available`.
    LOCK_NOT_AVAILABLE = b"55P03";
    /// `55P04`, `unsafe_new_enum_value_usage`.
    UNSAFE_NEW_ENUM_VALUE_USAGE = b"55P04";
    // Class 57 - Operator Intervention
    /// `57000`, `operator_intervention`.
    OPERATOR_INTERVENTION = b"57000";
    /// `57014`, `query_canceled`.
    QUERY_CANCELED = b"57014";
    /// `57P01`, `admin_shutdown`.
    ADMIN_SHUTDOWN = b"57P01";
    /// `57P02`, `crash_shutdown`.
    CRASH_SHUTDOWN = b"57P02";
    /// `57P03`, `cannot_connect_now`.
    CANNOT_CONNECT_NOW = b"57P03";
    /// `57P04`, `database_dropped`.
    DATABASE_DROPPED = b"57P04";
    /// `57P05`, `idle_session_timeout`.
    IDLE_SESSION_TIMEOUT = b"57P05";
    // Class 58 - System Error (errors external to PostgreSQL itself)
    /// `58000`, `system_error`.
    SYSTEM_ERROR = b"58000";
    /// `58030`, `io_error`.
    IO_ERROR = b"58030";
    /// `58P01`, `undefined_file`.
    UNDEFINED_FILE = b"58P01";
    /// `58P02`, `duplicate_file`.
    DUPLICATE_FILE = b"58P02";
    /// `58P03`, `file_name_too_long`.
    FILE_NAME_TOO_LONG = b"58P03";
    // Class F0 - Configuration File Error
    /// `F0000`, `config_file_error`.
    CONFIG_FILE_ERROR = b"F0000";
    /// `F0001`, `lock_file_exists`.
    LOCK_FILE_EXISTS = b"F0001";
    // Class HV - Foreign Data Wrapper Error (SQL/MED)
    /// `HV000`, `fdw_error`.
    FDW_ERROR = b"HV000";
    /// `HV005`, `fdw_column_name_not_found`.
    FDW_COLUMN_NAME_NOT_FOUND = b"HV005";
    /// `HV002`, `fdw_dynamic_parameter_value_needed`.
    FDW_DYNAMIC_PARAMETER_VALUE_NEEDED = b"HV002";
    /// `HV010`, `fdw_function_sequence_error`.
    FDW_FUNCTION_SEQUENCE_ERROR = b"HV010";
    /// `HV021`, `fdw_inconsistent_descriptor_information`.
    FDW_INCONSISTENT_DESCRIPTOR_INFORMATION = b"HV021";
    /// `HV024`, `fdw_invalid_attribute_value`.
    FDW_INVALID_ATTRIBUTE_VALUE = b"HV024";
    /// `HV007`, `fdw_invalid_column_name`.
    FDW_INVALID_COLUMN_NAME = b"HV007";
    /// `HV008`, `fdw_invalid_column_number`.
    FDW_INVALID_COLUMN_NUMBER = b"HV008";
    /// `HV004`, `fdw_invalid_data_type`.
    FDW_INVALID_DATA_TYPE = b"HV004";
    /// `HV006`, `fdw_invalid_data_type_descriptors`.
    FDW_INVALID_DATA_TYPE_DESCRIPTORS = b"HV006";
    /// `HV091`, `fdw_invalid_descriptor_field_identifier`.
    FDW_INVALID_DESCRIPTOR_FIELD_IDENTIFIER = b"HV091";
    /// `HV00B`, `fdw_invalid_handle`.
    FDW_INVALID_HANDLE = b"HV00B";
    /// `HV00C`, `fdw_invalid_option_index`.
    FDW_INVALID_OPTION_INDEX = b"HV00C";
    /// `HV00D`, `fdw_invalid_option_name`.
    FDW_INVALID_OPTION_NAME = b"HV00D";
    /// `HV090`, `fdw_invalid_string_length_or_buffer_length`.
    FDW_INVALID_STRING_LENGTH_OR_BUFFER_LENGTH = b"HV090";
    /// `HV00A`, `fdw_invalid_string_format`.
    FDW_INVALID_STRING_FORMAT = b"HV00A";
    /// `HV009`, `fdw_invalid_use_of_null_pointer`.
    FDW_INVALID_USE_OF_NULL_POINTER = b"HV009";
    /// `HV014`, `fdw_too_many_handles`.
    FDW_TOO_MANY_HANDLES = b"HV014";
    /// `HV001`, `fdw_out_of_memory`.
    FDW_OUT_OF_MEMORY = b"HV001";
    /// `HV00P`, `fdw_no_schemas`.
    FDW_NO_SCHEMAS = b"HV00P";
    /// `HV00J`, `fdw_option_name_not_found`.
    FDW_OPTION_NAME_NOT_FOUND = b"HV00J";
    /// `HV00K`, `fdw_reply_handle`.
    FDW_REPLY_HANDLE = b"HV00K";
    /// `HV00Q`, `fdw_schema_not_found`.
    FDW_SCHEMA_NOT_FOUND = b"HV00Q";
    /// `HV00R`, `fdw_table_not_found`.
    FDW_TABLE_NOT_FOUND = b"HV00R";
    /// `HV00L`, `fdw_unable_to_create_execution`.
    FDW_UNABLE_TO_CREATE_EXECUTION = b"HV00L";
    /// `HV00M`, `fdw_unable_to_create_reply`.
    FDW_UNABLE_TO_CREATE_REPLY = b"HV00M";
    /// `HV00N`, `fdw_unable_to_establish_connection`.
    FDW_UNABLE_TO_ESTABLISH_CONNECTION = b"HV00N";
    // Class P0 - PL/pgSQL Error
    /// `P0000`, `plpgsql_error`.
    PLPGSQL_ERROR = b"P0000";
    /// `P0001`, `raise_exception`.
    RAISE_EXCEPTION = b"P0001";
    /// `P0002`, `no_data_found`.
    NO_DATA_FOUND = b"P0002";
    /// `P0003`, `too_many_rows`.
    TOO_MANY_ROWS = b"P0003";
    /// `P0004`, `assert_failure`.
    ASSERT_FAILURE = b"P0004";
    // Class XX - Internal Error
    /// `XX000`, `internal_error`.
    INTERNAL_ERROR = b"XX000";
    /// `XX001`, `data_corrupted`.
    DATA_CORRUPTED = b"XX001";
    /// `XX002`, `index_corrupted`.
    INDEX_CORRUPTED = b"XX002";
}
