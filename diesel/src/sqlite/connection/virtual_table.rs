//! The `sqlite3_module` behind SQLite table function registration.
//!
//! SQLite owns `Module` until the last table disconnects, `VirtualTable` from `connect` to `disconnect`, and `Cursor` from `open` to `close`.
//! The `repr(C)` table and cursor headers must remain first so SQLite's header pointers retain the original `Box` addresses.
//! Each closing callback rebuilds its `Box` exactly once from the pointer handed to SQLite.
#![allow(unsafe_code)] // ffi calls
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::bind_collector::OwnedSqliteBindValue;
use super::functions::FunctionRow;
use super::raw::RawConnection;
use super::sqlite_value::OwnedSqliteValue;
use super::stmt::ensure_sqlite_ok;
use super::table_function::TableFunction;
use crate::result::QueryResult;
use crate::sqlite::SqliteConnection;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ffi as libc;
use core::ptr::{self, NonNull};

struct Module {
    module: ffi::sqlite3_module,
    function: TableFunction,
}

#[repr(C)]
struct VirtualTable {
    base: ffi::sqlite3_vtab,
    // Valid while the table is connected, as SQLite keeps the module alive until then.
    function: *const TableFunction,
    database: NonNull<ffi::sqlite3>,
}

#[repr(C)]
struct Cursor {
    base: ffi::sqlite3_vtab_cursor,
    columns: usize,
    values: Vec<OwnedSqliteBindValue>,
    arguments: Vec<Option<OwnedSqliteValue>>,
    row: usize,
}

pub(super) fn register(connection: &RawConnection, function: TableFunction) -> QueryResult<()> {
    // SAFETY: every field of `sqlite3_module` is an integer or an optional function
    // pointer, for which all zero bits are valid, and zeroing skips fields that only
    // some supported SQLite versions declare.
    let mut module: ffi::sqlite3_module = unsafe { core::mem::zeroed() };
    module.iVersion = 1;
    // `xCreate` stays null, which makes the table eponymous-only.
    module.xConnect = Some(connect);
    module.xBestIndex = Some(best_index);
    module.xDisconnect = Some(disconnect);
    module.xOpen = Some(open);
    module.xClose = Some(close);
    module.xFilter = Some(filter);
    module.xNext = Some(next);
    module.xEof = Some(eof);
    module.xColumn = Some(column);
    module.xRowid = Some(rowid);

    let data = Box::into_raw(Box::new(Module { module, function }));
    let database = connection.internal_connection.as_ptr();
    // SAFETY: `database` is open, and `data` with the module and its name inside it stays
    // allocated until SQLite calls `destroy_module`, which it also does when the
    // registration fails, after copying the name.
    let result = unsafe {
        ffi::sqlite3_create_module_v2(
            database,
            (*data).function.name.as_ptr(),
            &raw const (*data).module,
            data.cast(),
            Some(destroy_module),
        )
    };
    ensure_sqlite_ok(result, database)
}

/// # Safety
///
/// `data` is the `Module` that `register` leaked, and SQLite calls this once.
unsafe extern "C" fn destroy_module(data: *mut libc::c_void) {
    // SAFETY: see the function contract.
    drop(unsafe { Box::from_raw(data.cast::<Module>()) });
}

/// # Safety
///
/// SQLite calls this with an open `database`, the `Module` passed as client data in
/// `client_data`, and a writable `vtab_out`.
unsafe extern "C" fn connect(
    database: *mut ffi::sqlite3,
    client_data: *mut libc::c_void,
    _argument_count: libc::c_int,
    _arguments: *const *const libc::c_char,
    vtab_out: *mut *mut ffi::sqlite3_vtab,
    error_out: *mut *mut libc::c_char,
) -> libc::c_int {
    // SAFETY: `client_data` is the live `Module`, only read here.
    let function = unsafe { &(*client_data.cast::<Module>()).function };
    // SAFETY: `database` is open and the declaration is a NUL-terminated string owned by the module.
    let result = unsafe { ffi::sqlite3_declare_vtab(database, function.declaration.as_ptr()) };
    if result != ffi::SQLITE_OK {
        // SAFETY: `database` is open, so `sqlite3_errmsg` returns a NUL-terminated string.
        let message = unsafe { libc::CStr::from_ptr(ffi::sqlite3_errmsg(database)) };
        // SAFETY: `error_out` is writable, and SQLite frees what it holds with `sqlite3_free`.
        unsafe { *error_out = sqlite_string(message.to_bytes()) };
        return result;
    }
    let Some(database) = NonNull::new(database) else {
        return ffi::SQLITE_ERROR;
    };
    let table = Box::new(VirtualTable {
        base: ffi::sqlite3_vtab {
            pModule: ptr::null(),
            nRef: 0,
            zErrMsg: ptr::null_mut(),
        },
        function,
        database,
    });
    // SAFETY: `vtab_out` is writable, and the first-field header preserves the `Box` address for `disconnect`.
    unsafe { *vtab_out = Box::into_raw(table).cast() };
    ffi::SQLITE_OK
}

/// # Safety
///
/// `vtab` is a table from `connect`.
unsafe extern "C" fn disconnect(vtab: *mut ffi::sqlite3_vtab) -> libc::c_int {
    // SAFETY: the header retains the `Box` address, and SQLite disconnects once after closing all cursors.
    drop(unsafe { Box::from_raw(vtab.cast::<VirtualTable>()) });
    ffi::SQLITE_OK
}

/// Accepts only plans that bind every parameter with `=`, so a call never runs with
/// an argument missing.
///
/// # Safety
///
/// `vtab` is a table from `connect` and `info` is the planner's index information.
unsafe extern "C" fn best_index(
    vtab: *mut ffi::sqlite3_vtab,
    info: *mut ffi::sqlite3_index_info,
) -> libc::c_int {
    // SAFETY: the table holds a pointer to the live function, only read here.
    let function = unsafe { &*(*vtab.cast::<VirtualTable>()).function };
    // SAFETY: SQLite hands the planner state to this call alone.
    let info = unsafe { &mut *info };
    let count = usize::try_from(info.nConstraint).unwrap_or(0);
    if count > 0 {
        // SAFETY: SQLite allocates `nConstraint` entries in both arrays.
        let constraints = unsafe { core::slice::from_raw_parts(info.aConstraint, count) };
        // SAFETY: as above, and the usage array is ours to write during this call.
        let usages = unsafe { core::slice::from_raw_parts_mut(info.aConstraintUsage, count) };
        for parameter in 0..function.parameters {
            let hidden_column = function.columns + parameter;
            let bound = constraints.iter().position(|constraint| {
                usize::try_from(constraint.iColumn) == Ok(hidden_column)
                    && libc::c_int::from(constraint.op) == ffi::SQLITE_INDEX_CONSTRAINT_EQ
                    && constraint.usable != 0
            });
            let (Some(index), Ok(argument)) = (bound, libc::c_int::try_from(parameter + 1)) else {
                return ffi::SQLITE_CONSTRAINT;
            };
            usages[index].argvIndex = argument;
            usages[index].omit = 1;
        }
    } else if function.parameters > 0 {
        return ffi::SQLITE_CONSTRAINT;
    }
    // The cost SQLite's own `json_each` reports once its argument is bound.
    info.estimatedCost = 1.0;
    ffi::SQLITE_OK
}

/// # Safety
///
/// `vtab` is a table from `connect` and `cursor_out` is writable.
unsafe extern "C" fn open(
    vtab: *mut ffi::sqlite3_vtab,
    cursor_out: *mut *mut ffi::sqlite3_vtab_cursor,
) -> libc::c_int {
    // SAFETY: the table holds a pointer to the live function, only read here.
    let columns = unsafe { (*(*vtab.cast::<VirtualTable>()).function).columns };
    let cursor = Box::new(Cursor {
        base: ffi::sqlite3_vtab_cursor {
            pVtab: ptr::null_mut(),
        },
        columns,
        values: Vec::new(),
        arguments: Vec::new(),
        row: 0,
    });
    // SAFETY: `cursor_out` is writable, and the first-field header preserves the `Box` address for `close`.
    unsafe { *cursor_out = Box::into_raw(cursor).cast() };
    ffi::SQLITE_OK
}

/// # Safety
///
/// `cursor` is a cursor from `open`.
unsafe extern "C" fn close(cursor: *mut ffi::sqlite3_vtab_cursor) -> libc::c_int {
    // SAFETY: the header retains the `Box` address, and SQLite closes each cursor exactly once.
    drop(unsafe { Box::from_raw(cursor.cast::<Cursor>()) });
    ffi::SQLITE_OK
}

/// Runs the callback for one call and stores every row it returns on the cursor.
///
/// # Safety
///
/// `cursor` is a cursor from `open` whose `pVtab` SQLite has set, and `values` holds
/// `value_count` values, one per parameter as `best_index` assigned them.
unsafe extern "C" fn filter(
    cursor: *mut ffi::sqlite3_vtab_cursor,
    _index_number: libc::c_int,
    _index_name: *const libc::c_char,
    value_count: libc::c_int,
    values: *mut *mut ffi::sqlite3_value,
) -> libc::c_int {
    // No reference to the table or cursor is held across the callback, which may
    // filter this same table again through a nested query.
    // SAFETY: SQLite set `pVtab` to the table from `connect`.
    let vtab = unsafe { (*cursor).pVtab };
    // SAFETY: the table holds a pointer to the live function, never mutated after registration.
    let function = unsafe { &*(*vtab.cast::<VirtualTable>()).function };
    // SAFETY: as above, `database` is a plain field copied out.
    let database = unsafe { (*vtab.cast::<VirtualTable>()).database };
    let arguments: &mut [*mut ffi::sqlite3_value] = match usize::try_from(value_count) {
        Ok(count) if count > 0 && !values.is_null() => {
            // SAFETY: SQLite passes `value_count` values in `values`, valid for this call.
            unsafe { core::slice::from_raw_parts_mut(values, count) }
        }
        _ => &mut [],
    };
    let copies = arguments
        .iter()
        .map(|&value| match NonNull::new(value) {
            Some(value) => OwnedSqliteValue::copy_from_ptr(value),
            None => Ok(None),
        })
        .collect::<QueryResult<Vec<_>>>();

    let outcome = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let arguments = FunctionRow::new(arguments, database);
        // SAFETY: `database` is the open connection running the statement that called this.
        unsafe {
            SqliteConnection::with_borrowed_connection(database, |connection| {
                (function.callback)(connection, &arguments)
            })
        }
    }));
    let stored = match outcome {
        Ok(result) => result.and_then(|values| Ok((values, copies?))),
        Err(_) => {
            // SAFETY: `vtab` is live for this call.
            unsafe {
                set_error(
                    vtab,
                    &format!("`{}` panicked", function.name.to_string_lossy()),
                )
            };
            return ffi::SQLITE_ERROR;
        }
    };
    match stored {
        Ok((values, arguments)) => {
            // SAFETY: the callback has returned, so this is the only reference to the cursor.
            let cursor = unsafe { &mut *cursor.cast::<Cursor>() };
            cursor.values = values;
            cursor.arguments = arguments;
            cursor.row = 0;
            ffi::SQLITE_OK
        }
        Err(error) => {
            // SAFETY: `vtab` is live for this call.
            unsafe { set_error(vtab, &error.to_string()) };
            ffi::SQLITE_ERROR
        }
    }
}

/// # Safety
///
/// `cursor` is a cursor from `open`.
unsafe extern "C" fn next(cursor: *mut ffi::sqlite3_vtab_cursor) -> libc::c_int {
    // SAFETY: SQLite uses a cursor from one call at a time.
    unsafe { (*cursor.cast::<Cursor>()).row += 1 };
    ffi::SQLITE_OK
}

/// # Safety
///
/// `cursor` is a cursor from `open`.
unsafe extern "C" fn eof(cursor: *mut ffi::sqlite3_vtab_cursor) -> libc::c_int {
    // SAFETY: SQLite uses a cursor from one call at a time.
    let cursor = unsafe { &*cursor.cast::<Cursor>() };
    libc::c_int::from(cursor.row * cursor.columns >= cursor.values.len())
}

/// # Safety
///
/// `cursor` is a cursor from `open` and `context` the result context for this call.
unsafe extern "C" fn column(
    cursor: *mut ffi::sqlite3_vtab_cursor,
    context: *mut ffi::sqlite3_context,
    index: libc::c_int,
) -> libc::c_int {
    // SAFETY: SQLite uses a cursor from one call at a time.
    let cursor = unsafe { &*cursor.cast::<Cursor>() };
    let Ok(index) = usize::try_from(index) else {
        return ffi::SQLITE_RANGE;
    };
    let value = if index < cursor.columns {
        if let Some(value) = cursor.values.get(cursor.row * cursor.columns + index) {
            // SAFETY: `context` is valid for this call.
            if value
                .as_value_ref()
                .result_of(unsafe { &mut *context })
                .is_err()
            {
                // SAFETY: as above.
                unsafe { ffi::sqlite3_result_error_toobig(context) };
            }
            return ffi::SQLITE_OK;
        }
        None
    } else {
        cursor
            .arguments
            .get(index - cursor.columns)
            .and_then(Option::as_ref)
    };
    match value {
        // SAFETY: `context` is valid for this call and SQLite copies the value.
        Some(value) => unsafe { ffi::sqlite3_result_value(context, value.value.as_ptr()) },
        // SAFETY: as above.
        None => unsafe { ffi::sqlite3_result_null(context) },
    }
    ffi::SQLITE_OK
}

/// # Safety
///
/// `cursor` is a cursor from `open` and `rowid` is writable.
unsafe extern "C" fn rowid(
    cursor: *mut ffi::sqlite3_vtab_cursor,
    rowid: *mut ffi::sqlite3_int64,
) -> libc::c_int {
    // SAFETY: SQLite uses a cursor from one call at a time.
    let row = unsafe { (*cursor.cast::<Cursor>()).row };
    let Ok(row) = ffi::sqlite3_int64::try_from(row) else {
        return ffi::SQLITE_RANGE;
    };
    // SAFETY: `rowid` is writable for this call.
    unsafe { *rowid = row + 1 };
    ffi::SQLITE_OK
}

/// Copies `message` into a NUL-terminated string from SQLite's allocator, or null when out of memory.
fn sqlite_string(message: &[u8]) -> *mut libc::c_char {
    let copy = match u64::try_from(message.len() + 1) {
        // SAFETY: `sqlite3_malloc64` has no preconditions.
        Ok(size) => unsafe { ffi::sqlite3_malloc64(size) }.cast::<u8>(),
        Err(_) => ptr::null_mut(),
    };
    if !copy.is_null() {
        // SAFETY: `copy` holds `message.len() + 1` bytes and does not overlap `message`.
        unsafe {
            ptr::copy_nonoverlapping(message.as_ptr(), copy, message.len());
            *copy.add(message.len()) = 0;
        }
    }
    copy.cast()
}

/// Hands `message` to SQLite as the error of the statement running the table.
///
/// # Safety
///
/// `vtab` points to a live table from `connect`.
unsafe fn set_error(vtab: *mut ffi::sqlite3_vtab, message: &str) {
    let copy = sqlite_string(message.as_bytes());
    // SAFETY: `zErrMsg` is null or a string from SQLite's allocator, which this replaces.
    unsafe {
        ffi::sqlite3_free((*vtab).zErrMsg.cast());
        (*vtab).zErrMsg = copy;
    }
}
