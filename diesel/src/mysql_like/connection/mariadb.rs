#![cfg(feature = "mariadb")]
#![allow(unsafe_code)]
use core::ffi::c_int;
use mysqlclient_sys as ffi;
use std::sync::OnceLock;
// The option enum is a 4-byte int in the C ABI, so c_int avoids naming the bindgen type.
pub(super) type OptionsV = unsafe extern "C" fn(*mut ffi::MYSQL, c_int, ...) -> c_int;

#[cfg(unix)]
unsafe fn lookup() -> Option<OptionsV> {
    let lib = libloading::os::unix::Library::this();
    let f = unsafe { lib.get::<OptionsV>(b"mysql_optionsv\0").ok().map(|s| *s) };
    core::mem::forget(lib);
    f
}

#[cfg(windows)]
unsafe fn lookup() -> Option<OptionsV> {
    use windows_sys::Win32::System::LibraryLoader::*;
    let anchor = ffi::mysql_options as usize as *const u16;
    let mut module = std::mem::zeroed();
    let flags =
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    if GetModuleHandleExW(flags, anchor, &mut module) == 0 {
        return None;
    }
    GetProcAddress(module, c"mysql_optionsv".as_ptr().cast())
        .map(|p| std::mem::transmute::<_, OptionsV>(p))
}

/// Returns `mysql_optionsv`, or panics if the loaded client library lacks it.
pub(super) fn optionsv() -> OptionsV {
    static F: OnceLock<Option<OptionsV>> = OnceLock::new();
    F.get_or_init(|| unsafe { lookup() })
            .expect("the `mariadb` feature requires a client library that exports `mysql_optionsv` (MariaDB Connector/C); the loaded libmysqlclient does not. Please install libmariadb-dev")
}
