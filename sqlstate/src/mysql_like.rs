//! The SQLSTATE codes MySQL 8.4 and MariaDB 11.8 report, taken from
//! [`messages_to_clients.txt`](https://github.com/mysql/mysql-server/blob/mysql-8.4.6/share/messages_to_clients.txt)
//! and [`errmsg-utf8.txt`](https://github.com/MariaDB/server/blob/11.8/sql/share/errmsg-utf8.txt).
//!
//! ```rust
//! use sqlstate::{SqlState, mysql_like};
//!
//! let code: SqlState = "40001".parse().unwrap();
//! assert!(matches!(code, mysql_like::SERIALIZATION_FAILURE));
//! ```

codes! {
    /// `01000`, warning.
    WARNING = b"01000";
    /// `01S00`, SRS ID already exists warning. MySQL only.
    WARN_SRS_ID_ALREADY_EXISTS = b"01S00";
    /// `01S01`, reserved SRID range warning. MySQL only.
    WARN_RESERVED_SRID_RANGE = b"01S01";
    /// `02000`, no data.
    NO_DATA = b"02000";
    /// `08004`, SQL server rejected establishment of SQL connection.
    SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION = b"08004";
    /// `08S01`, communication link failure.
    COMMUNICATION_LINK_FAILURE = b"08S01";
    /// `0A000`, feature not supported.
    FEATURE_NOT_SUPPORTED = b"0A000";
    /// `0K000`, resignal when handler not active.
    RESIGNAL_WHEN_HANDLER_NOT_ACTIVE = b"0K000";
    /// `0L000`, invalid grantor. MariaDB only.
    INVALID_GRANTOR = b"0L000";
    /// `0Z002`, stacked diagnostics accessed without active handler.
    STACKED_DIAGNOSTICS_ACCESSED_WITHOUT_ACTIVE_HANDLER = b"0Z002";
    /// `20000`, case not found for case statement.
    CASE_NOT_FOUND = b"20000";
    /// `21000`, cardinality violation.
    CARDINALITY_VIOLATION = b"21000";
    /// `21S01`, insert value list does not match column list.
    INSERT_VALUE_LIST_DOES_NOT_MATCH_COLUMN_LIST = b"21S01";
    /// `22001`, string data, right truncation.
    STRING_DATA_RIGHT_TRUNCATION = b"22001";
    /// `22003`, numeric value out of range.
    NUMERIC_VALUE_OUT_OF_RANGE = b"22003";
    /// `22004`, null value not allowed.
    NULL_VALUE_NOT_ALLOWED = b"22004";
    /// `22007`, invalid datetime format.
    INVALID_DATETIME_FORMAT = b"22007";
    /// `22008`, datetime field overflow.
    DATETIME_FIELD_OVERFLOW = b"22008";
    /// `22012`, division by zero.
    DIVISION_BY_ZERO = b"22012";
    /// `22018`, invalid character value for cast. MySQL only.
    INVALID_CHARACTER_VALUE_FOR_CAST = b"22018";
    /// `2201E`, invalid argument for natural logarithm.
    INVALID_ARGUMENT_FOR_LOGARITHM = b"2201E";
    /// `22023`, invalid parameter value.
    INVALID_PARAMETER_VALUE = b"22023";
    /// `2202H`, invalid tablesample argument. MySQL only.
    INVALID_TABLESAMPLE_ARGUMENT = b"2202H";
    /// `22032`, invalid JSON text. MySQL only.
    INVALID_JSON_TEXT = b"22032";
    /// `22034`, more than one SQL/JSON item. MySQL only.
    MORE_THAN_ONE_SQL_JSON_ITEM = b"22034";
    /// `22035`, no SQL/JSON item. MySQL only.
    NO_SQL_JSON_ITEM = b"22035";
    /// `2203F`, SQL/JSON scalar required. MySQL only.
    SQL_JSON_SCALAR_REQUIRED = b"2203F";
    /// `22S00`, unsupported SRS operation. MySQL only.
    SRS_NOT_SUPPORTED = b"22S00";
    /// `22S01`, unexpected geometry type. MySQL only.
    UNEXPECTED_GEOMETRY_TYPE = b"22S01";
    /// `22S02`, longitude out of range. MySQL only.
    LONGITUDE_OUT_OF_RANGE = b"22S02";
    /// `22S03`, latitude out of range. MySQL only.
    LATITUDE_OUT_OF_RANGE = b"22S03";
    /// `22S04`, invalid polygon ring direction. MySQL only.
    INVALID_CAST_POLYGON_RING_DIRECTION = b"22S04";
    /// `22S05`, aggregate over different SRIDs. MySQL only.
    GIS_DIFFERENT_SRIDS_AGGREGATION = b"22S05";
    /// `23000`, integrity constraint violation.
    INTEGRITY_CONSTRAINT_VIOLATION = b"23000";
    /// `24000`, invalid cursor state.
    INVALID_CURSOR_STATE = b"24000";
    /// `25000`, invalid transaction state.
    INVALID_TRANSACTION_STATE = b"25000";
    /// `25001`, active SQL transaction.
    ACTIVE_SQL_TRANSACTION = b"25001";
    /// `25006`, read-only SQL transaction.
    READ_ONLY_SQL_TRANSACTION = b"25006";
    /// `28000`, invalid authorization specification.
    INVALID_AUTHORIZATION_SPECIFICATION = b"28000";
    /// `2F003`, prohibited SQL statement attempted.
    PROHIBITED_SQL_STATEMENT_ATTEMPTED = b"2F003";
    /// `2F005`, function executed no return statement.
    FUNCTION_EXECUTED_NO_RETURN_STATEMENT = b"2F005";
    /// `35000`, invalid condition number.
    INVALID_CONDITION_NUMBER = b"35000";
    /// `3D000`, invalid catalog name.
    INVALID_CATALOG_NAME = b"3D000";
    /// `40000`, transaction rollback. MySQL only.
    TRANSACTION_ROLLBACK = b"40000";
    /// `40001`, serialization failure.
    SERIALIZATION_FAILURE = b"40001";
    /// `42000`, syntax error or access rule violation.
    SYNTAX_ERROR_OR_ACCESS_RULE_VIOLATION = b"42000";
    /// `42S01`, base table or view already exists.
    BASE_TABLE_OR_VIEW_ALREADY_EXISTS = b"42S01";
    /// `42S02`, base table or view not found.
    BASE_TABLE_OR_VIEW_NOT_FOUND = b"42S02";
    /// `42S12`, index not found.
    INDEX_NOT_FOUND = b"42S12";
    /// `42S21`, column already exists.
    COLUMN_ALREADY_EXISTS = b"42S21";
    /// `42S22`, column not found.
    COLUMN_NOT_FOUND = b"42S22";
    /// `42Y07`, unknown database. MySQL only.
    NO_SUCH_DB = b"42Y07";
    /// `44000`, with check option violation. MariaDB only.
    WITH_CHECK_OPTION_VIOLATION = b"44000";
    /// `70100`, operation aborted.
    OPERATION_ABORTED = b"70100";
    /// `HY000`, general error.
    GENERAL_ERROR = b"HY000";
    /// `HY001`, memory allocation error.
    MEMORY_ALLOCATION_ERROR = b"HY001";
    /// `OP000`, invalid role specification. MariaDB only.
    INVALID_ROLE = b"OP000";
    /// `SR000`, SRID 0 cannot be modified. MySQL only.
    CANT_MODIFY_SRID_0 = b"SR000";
    /// `SR001`, SRS not found. MySQL only.
    SRS_NOT_FOUND = b"SR001";
    /// `SR002`, invalid SRS definition. MySQL only.
    INVALID_SRS_DEFINITION = b"SR002";
    /// `SR003`, missing SRS projection parameter. MySQL only.
    SRS_PROJ_PARAMETER_MISSING = b"SR003";
    /// `SR004`, SRS ID already exists. MySQL only.
    SRS_ID_ALREADY_EXISTS = b"SR004";
    /// `SR005`, SRS used by a column. MySQL only.
    CANT_MODIFY_SRS_USED_BY_COLUMN = b"SR005";
    /// `SR006`, invalid SRS attribute. MySQL only.
    INVALID_SRS_ATTRIBUTE = b"SR006";
    /// `SU001`, unknown unit. MySQL only.
    UNIT_NOT_FOUND = b"SU001";
    /// `XA100`, XA branch rolled back.
    XA_RBROLLBACK = b"XA100";
    /// `XA102`, XA branch rolled back after a deadlock.
    XA_RBDEADLOCK = b"XA102";
    /// `XA106`, XA branch rolled back after a timeout.
    XA_RBTIMEOUT = b"XA106";
    /// `XAE03`, XA resource manager error.
    XAER_RMERR = b"XAE03";
    /// `XAE04`, unknown XID.
    XAER_NOTA = b"XAE04";
    /// `XAE05`, invalid XA arguments.
    XAER_INVAL = b"XAE05";
    /// `XAE07`, XA command not allowed in this state.
    XAER_RMFAIL = b"XAE07";
    /// `XAE08`, XID already exists.
    XAER_DUPID = b"XAE08";
    /// `XAE09`, work outside the XA transaction.
    XAER_OUTSIDE = b"XAE09";
}
