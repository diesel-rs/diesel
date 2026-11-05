//! The SQLSTATE codes MySQL and MariaDB report, as listed in the
//! [MySQL 8.4 server error reference](https://dev.mysql.com/doc/mysql-errors/8.4/en/server-error-reference.html)
//! and the [MariaDB error code reference](https://mariadb.com/docs/server/reference/error-codes/mariadb-error-code-reference),
//! and taken from
//! [`messages_to_clients.txt`](https://github.com/mysql/mysql-server/blob/mysql-8.4.6/share/messages_to_clients.txt)
//! and [`errmsg-utf8.txt`](https://github.com/MariaDB/server/blob/11.8/sql/share/errmsg-utf8.txt).
//!
//! Many server errors share one code, so a code names a condition rather
//! than an error. Each constant is named after the condition the SQL
//! standard, ODBC or X/Open gives the code, or after the MySQL errors that
//! report it when none does. Codes reported by only one of the two servers
//! say so.
//!
//! ```rust
//! use sql_state::{SqlState, mysql_like};
//!
//! fn retryable(code: SqlState) -> bool {
//!     matches!(code, mysql_like::SERIALIZATION_FAILURE)
//! }
//!
//! assert!(retryable("40001".parse().unwrap()));
//! assert!(!retryable("23000".parse().unwrap()));
//! ```

codes! {
    /// `01000`, warning.
    WARNING = b"01000";
    /// `01S00`, a warning that a spatial reference system ID already exists. Reported by MySQL only.
    ///
    /// Reported for `ER_WARN_SRS_ID_ALREADY_EXISTS`.
    WARN_SRS_ID_ALREADY_EXISTS = b"01S00";
    /// `01S01`, a warning that a spatial reference system ID lies in a reserved range. Reported by MySQL only.
    ///
    /// Reported for `ER_WARN_RESERVED_SRID_RANGE`.
    WARN_RESERVED_SRID_RANGE = b"01S01";
    /// `02000`, no data.
    ///
    /// Reported for `ER_SP_FETCH_NO_DATA` and `ER_SIGNAL_NOT_FOUND`.
    NO_DATA = b"02000";
    /// `08004`, SQL server rejected establishment of SQL connection.
    ///
    /// Reported for `ER_CON_COUNT_ERROR`, `ER_NOT_SUPPORTED_AUTH_MODE` and `ER_SECURE_TRANSPORT_REQUIRED`.
    SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION = b"08004";
    /// `08S01`, communication link failure.
    COMMUNICATION_LINK_FAILURE = b"08S01";
    /// `0A000`, feature not supported.
    FEATURE_NOT_SUPPORTED = b"0A000";
    /// `0K000`, resignal when handler not active.
    ///
    /// Reported for `ER_RESIGNAL_WITHOUT_ACTIVE_HANDLER`.
    RESIGNAL_WHEN_HANDLER_NOT_ACTIVE = b"0K000";
    /// `0L000`, invalid grantor. Reported by MariaDB only.
    ///
    /// Reported for `ER_MALFORMED_DEFINER` and `ER_INVALID_CURRENT_USER`.
    INVALID_GRANTOR = b"0L000";
    /// `0Z002`, stacked diagnostics accessed without active handler.
    ///
    /// Reported for `ER_GET_STACKED_DA_WITHOUT_ACTIVE_HANDLER`.
    STACKED_DIAGNOSTICS_ACCESSED_WITHOUT_ACTIVE_HANDLER = b"0Z002";
    /// `20000`, case not found for case statement.
    ///
    /// Reported for `ER_SP_CASE_NOT_FOUND`.
    CASE_NOT_FOUND = b"20000";
    /// `21000`, cardinality violation.
    ///
    /// Reported for `ER_WRONG_NUMBER_OF_COLUMNS_IN_SELECT`, `ER_OPERAND_COLUMNS` and `ER_SUBQUERY_NO_1_ROW`.
    CARDINALITY_VIOLATION = b"21000";
    /// `21S01`, insert value list does not match column list.
    ///
    /// Reported for `ER_WRONG_VALUE_COUNT` and `ER_WRONG_VALUE_COUNT_ON_ROW`.
    INSERT_VALUE_LIST_DOES_NOT_MATCH_COLUMN_LIST = b"21S01";
    /// `22001`, string data, right truncation.
    ///
    /// Reported for `ER_DATA_TOO_LONG` and `ER_FUNCTIONAL_INDEX_DATA_IS_TOO_LONG`.
    STRING_DATA_RIGHT_TRUNCATION = b"22001";
    /// `22003`, numeric value out of range.
    NUMERIC_VALUE_OUT_OF_RANGE = b"22003";
    /// `22004`, null value not allowed.
    ///
    /// Reported for `ER_INVALID_USE_OF_NULL` and `ER_WARN_NULL_TO_NOTNULL`.
    NULL_VALUE_NOT_ALLOWED = b"22004";
    /// `22007`, invalid datetime format.
    INVALID_DATETIME_FORMAT = b"22007";
    /// `22008`, datetime field overflow.
    ///
    /// Reported for `ER_DATETIME_FUNCTION_OVERFLOW`.
    DATETIME_FIELD_OVERFLOW = b"22008";
    /// `22012`, division by zero.
    ///
    /// Reported for `ER_DIVISION_BY_ZERO`.
    DIVISION_BY_ZERO = b"22012";
    /// `22018`, invalid character value for cast. Reported by MySQL only.
    ///
    /// Reported for `ER_INVALID_JSON_VALUE_FOR_CAST` and `ER_INVALID_JSON_VALUE_FOR_FUNC_INDEX`.
    INVALID_CHARACTER_VALUE_FOR_CAST = b"22018";
    /// `2201E`, invalid argument for natural logarithm.
    ///
    /// Reported for `ER_INVALID_ARGUMENT_FOR_LOGARITHM`.
    INVALID_ARGUMENT_FOR_LOGARITHM = b"2201E";
    /// `22023`, invalid parameter value.
    INVALID_PARAMETER_VALUE = b"22023";
    /// `2202H`, invalid tablesample argument. Reported by MySQL only.
    ///
    /// Reported for `ER_TABLESAMPLE_PERCENTAGE`.
    INVALID_TABLESAMPLE_ARGUMENT = b"2202H";
    /// `22032`, invalid JSON text. Reported by MySQL only.
    INVALID_JSON_TEXT = b"22032";
    /// `22034`, more than one SQL/JSON item. Reported by MySQL only.
    ///
    /// Reported for `ER_MULTIPLE_JSON_VALUES`.
    MORE_THAN_ONE_SQL_JSON_ITEM = b"22034";
    /// `22035`, no SQL/JSON item. Reported by MySQL only.
    ///
    /// Reported for `ER_MISSING_JSON_TABLE_VALUE` and `ER_MISSING_JSON_VALUE`.
    NO_SQL_JSON_ITEM = b"22035";
    /// `2203F`, SQL/JSON scalar required. Reported by MySQL only.
    ///
    /// Reported for `ER_WRONG_JSON_TABLE_VALUE`.
    SQL_JSON_SCALAR_REQUIRED = b"2203F";
    /// `22S00`, an operation the spatial reference system does not support. Reported by MySQL only.
    SRS_NOT_SUPPORTED = b"22S00";
    /// `22S01`, an unexpected geometry type. Reported by MySQL only.
    ///
    /// Reported for `ER_UNEXPECTED_GEOMETRY_TYPE` and `ER_INVALID_CAST_TO_GEOMETRY`.
    UNEXPECTED_GEOMETRY_TYPE = b"22S01";
    /// `22S02`, a longitude out of range. Reported by MySQL only.
    ///
    /// Reported for `ER_LONGITUDE_OUT_OF_RANGE` and `ER_GEOMETRY_PARAM_LONGITUDE_OUT_OF_RANGE`.
    LONGITUDE_OUT_OF_RANGE = b"22S02";
    /// `22S03`, a latitude out of range. Reported by MySQL only.
    ///
    /// Reported for `ER_LATITUDE_OUT_OF_RANGE` and `ER_GEOMETRY_PARAM_LATITUDE_OUT_OF_RANGE`.
    LATITUDE_OUT_OF_RANGE = b"22S03";
    /// `22S04`, a polygon cast with an invalid ring direction. Reported by MySQL only.
    ///
    /// Reported for `ER_INVALID_CAST_POLYGON_RING_DIRECTION`.
    INVALID_CAST_POLYGON_RING_DIRECTION = b"22S04";
    /// `22S05`, a spatial aggregate over different spatial reference systems. Reported by MySQL only.
    ///
    /// Reported for `ER_GIS_DIFFERENT_SRIDS_AGGREGATION`.
    GIS_DIFFERENT_SRIDS_AGGREGATION = b"22S05";
    /// `23000`, integrity constraint violation, such as a duplicate key or a foreign key violation.
    INTEGRITY_CONSTRAINT_VIOLATION = b"23000";
    /// `24000`, invalid cursor state.
    ///
    /// Reported for `ER_SP_CURSOR_ALREADY_OPEN` and `ER_SP_CURSOR_NOT_OPEN`.
    INVALID_CURSOR_STATE = b"24000";
    /// `25000`, invalid transaction state.
    ///
    /// Reported for `ER_CANT_DO_THIS_DURING_AN_TRANSACTION` and `ER_READ_ONLY_TRANSACTION`.
    INVALID_TRANSACTION_STATE = b"25000";
    /// `25001`, active SQL transaction.
    ///
    /// Reported for `ER_CANT_CHANGE_TX_CHARACTERISTICS`.
    ACTIVE_SQL_TRANSACTION = b"25001";
    /// `25006`, read-only SQL transaction.
    ///
    /// Reported for `ER_CANT_EXECUTE_IN_READ_ONLY_TRANSACTION`.
    READ_ONLY_SQL_TRANSACTION = b"25006";
    /// `28000`, invalid authorization specification.
    INVALID_AUTHORIZATION_SPECIFICATION = b"28000";
    /// `2F003`, prohibited SQL statement attempted.
    ///
    /// Reported for `ER_SP_NO_RECURSIVE_CREATE`.
    PROHIBITED_SQL_STATEMENT_ATTEMPTED = b"2F003";
    /// `2F005`, function executed no return statement.
    ///
    /// Reported for `ER_SP_NORETURNEND`.
    FUNCTION_EXECUTED_NO_RETURN_STATEMENT = b"2F005";
    /// `35000`, invalid condition number.
    ///
    /// Reported for `ER_DA_INVALID_CONDITION_NUMBER`.
    INVALID_CONDITION_NUMBER = b"35000";
    /// `3D000`, invalid catalog name.
    ///
    /// Reported for `ER_NO_DB_ERROR`.
    INVALID_CATALOG_NAME = b"3D000";
    /// `40000`, transaction rollback. Reported by MySQL only.
    ///
    /// Reported for `ER_TRANSACTION_ROLLBACK_DURING_COMMIT`.
    TRANSACTION_ROLLBACK = b"40000";
    /// `40001`, serialization failure, which both servers report for a deadlock.
    ///
    /// Reported for `ER_LOCK_DEADLOCK`.
    SERIALIZATION_FAILURE = b"40001";
    /// `42000`, syntax error or access rule violation.
    SYNTAX_ERROR_OR_ACCESS_RULE_VIOLATION = b"42000";
    /// `42S01`, base table or view already exists.
    ///
    /// Reported for `ER_TABLE_EXISTS_ERROR`.
    BASE_TABLE_OR_VIEW_ALREADY_EXISTS = b"42S01";
    /// `42S02`, base table or view not found.
    BASE_TABLE_OR_VIEW_NOT_FOUND = b"42S02";
    /// `42S12`, index not found.
    ///
    /// Reported for `ER_NO_SUCH_INDEX`.
    INDEX_NOT_FOUND = b"42S12";
    /// `42S21`, column already exists.
    ///
    /// Reported for `ER_DUP_FIELDNAME`.
    COLUMN_ALREADY_EXISTS = b"42S21";
    /// `42S22`, column not found.
    ///
    /// Reported for `ER_BAD_FIELD_ERROR` and `ER_ILLEGAL_REFERENCE`.
    COLUMN_NOT_FOUND = b"42S22";
    /// `42Y07`, a database that does not exist. Reported by MySQL only.
    ///
    /// Reported for `ER_NO_SUCH_DB`.
    NO_SUCH_DB = b"42Y07";
    /// `44000`, with check option violation. Reported by MariaDB only.
    ///
    /// Reported for `ER_VIEW_CHECK_FAILED`.
    WITH_CHECK_OPTION_VIOLATION = b"44000";
    /// `70100`, operation aborted, such as an interrupted query.
    OPERATION_ABORTED = b"70100";
    /// `HY000`, general error, which covers every server error without a code of its own and the errors the client library raises.
    GENERAL_ERROR = b"HY000";
    /// `HY001`, memory allocation error.
    ///
    /// Reported for `ER_OUTOFMEMORY` and `ER_OUT_OF_SORTMEMORY`.
    MEMORY_ALLOCATION_ERROR = b"HY001";
    /// `OP000`, invalid role specification, spelled with the letter `O` where the SQL standard has `0P000`. Reported by MariaDB only.
    ///
    /// Reported for `ER_INVALID_ROLE`.
    INVALID_ROLE = b"OP000";
    /// `SR000`, an attempt to modify spatial reference system 0. Reported by MySQL only.
    ///
    /// Reported for `ER_CANT_MODIFY_SRID_0`.
    CANT_MODIFY_SRID_0 = b"SR000";
    /// `SR001`, a spatial reference system that does not exist. Reported by MySQL only.
    ///
    /// Reported for `ER_SRS_NOT_CARTESIAN_UNDEFINED` and `ER_SRS_NOT_FOUND`.
    SRS_NOT_FOUND = b"SR001";
    /// `SR002`, an invalid spatial reference system definition. Reported by MySQL only.
    INVALID_SRS_DEFINITION = b"SR002";
    /// `SR003`, a spatial reference system definition missing a projection parameter. Reported by MySQL only.
    ///
    /// Reported for `ER_SRS_PROJ_PARAMETER_MISSING`.
    SRS_PROJ_PARAMETER_MISSING = b"SR003";
    /// `SR004`, a spatial reference system ID that already exists. Reported by MySQL only.
    ///
    /// Reported for `ER_SRS_ID_ALREADY_EXISTS`.
    SRS_ID_ALREADY_EXISTS = b"SR004";
    /// `SR005`, an attempt to modify a spatial reference system a column uses. Reported by MySQL only.
    ///
    /// Reported for `ER_CANT_MODIFY_SRS_USED_BY_COLUMN`.
    CANT_MODIFY_SRS_USED_BY_COLUMN = b"SR005";
    /// `SR006`, an invalid spatial reference system attribute. Reported by MySQL only.
    INVALID_SRS_ATTRIBUTE = b"SR006";
    /// `SU001`, an unknown unit of measure. Reported by MySQL only.
    ///
    /// Reported for `ER_GEOMETRY_IN_UNKNOWN_LENGTH_UNIT` and `ER_UNIT_NOT_FOUND`.
    UNIT_NOT_FOUND = b"SU001";
    /// `XA100`, `XA_RBROLLBACK`, the XA transaction branch was rolled back.
    ///
    /// Reported for `ER_XA_RBROLLBACK`.
    XA_RBROLLBACK = b"XA100";
    /// `XA102`, `XA_RBDEADLOCK`, the XA transaction branch was rolled back after a deadlock.
    ///
    /// Reported for `ER_XA_RBDEADLOCK`.
    XA_RBDEADLOCK = b"XA102";
    /// `XA106`, `XA_RBTIMEOUT`, the XA transaction branch was rolled back after a timeout.
    ///
    /// Reported for `ER_XA_RBTIMEOUT`.
    XA_RBTIMEOUT = b"XA106";
    /// `XAE03`, `XAER_RMERR`, a resource manager error in the XA transaction branch.
    ///
    /// Reported for `ER_XAER_RMERR`.
    XAER_RMERR = b"XAE03";
    /// `XAE04`, `XAER_NOTA`, an unknown XA transaction ID.
    ///
    /// Reported for `ER_XAER_NOTA`.
    XAER_NOTA = b"XAE04";
    /// `XAE05`, `XAER_INVAL`, invalid XA arguments.
    ///
    /// Reported for `ER_XAER_INVAL`.
    XAER_INVAL = b"XAE05";
    /// `XAE07`, `XAER_RMFAIL`, an XA command not allowed in the current transaction state.
    ///
    /// Reported for `ER_XAER_RMFAIL`.
    XAER_RMFAIL = b"XAE07";
    /// `XAE08`, `XAER_DUPID`, an XA transaction ID that already exists.
    ///
    /// Reported for `ER_XAER_DUPID`.
    XAER_DUPID = b"XAE08";
    /// `XAE09`, `XAER_OUTSIDE`, work done outside the XA transaction.
    ///
    /// Reported for `ER_XAER_OUTSIDE`.
    XAER_OUTSIDE = b"XAE09";
}
