pub mod utils {
    //! Low-level utilities, traits, and macros for implementing custom SQLite Virtual File Systems (VFS).

    use libsqlite3_sys::*;

    use alloc::boxed::Box;
    use alloc::ffi::CString;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::{cell::RefCell, ffi::CStr, ops::Deref};

    /// A macro to return a specific SQLite error code if a condition is true.
    ///
    /// The default error code is SQLITE_ERROR.
    #[macro_export]
    macro_rules! bail {
        ($ex:expr) => {
            bail!($ex, SQLITE_ERROR);
        };
        ($ex:expr, $code: expr) => {
            if $ex {
                return $code;
            }
        };
    }

    /// A macro to safely unwrap an `Option<T>`, returning a SQLite error code on `None`.
    ///
    /// The default error code is SQLITE_ERROR.
    #[macro_export]
    macro_rules! check_option {
        ($ex:expr) => {
            check_option!($ex, SQLITE_ERROR)
        };
        ($ex:expr, $code: expr) => {
            if let Some(v) = $ex {
                v
            } else {
                return $code;
            }
        };
    }

    /// A macro to safely unwrap a `Result<T, E>`, returning a SQLite error code on `Err`.
    ///
    /// The default err code is SQLITE_ERROR.
    #[macro_export]
    macro_rules! check_result {
        ($ex:expr) => {
            check_result!($ex, SQLITE_ERROR)
        };
        ($ex:expr, $code: expr) => {
            if let Ok(v) = $ex {
                v
            } else {
                return $code;
            }
        };
    }

    /// A macro to explicitly mark a parameter as unused, suppressing compiler warnings.
    #[macro_export]
    macro_rules! unused {
        ($ex:expr) => {
            let _ = $ex;
        };
    }

    /// The header of the SQLite file is used to determine whether the imported file is legal.
    pub const SQLITE3_HEADER: &str = "SQLite format 3";

    /// Generates a random, temporary filename, typically used when SQLite requests a file with a NULL name.
    pub fn random_name() -> alloc::string::String {
        todo!()
        // let random = Number::from(Math::random()).to_string(36).unwrap();
        // random.slice(2, random.length()).as_string().unwrap()
    }

    /// An in-memory file implementation that stores data in fixed-size chunks. Suitable for temporary files.
    pub struct MemChunksFile {
        chunks: Vec<Vec<u8>>,
        chunk_size: Option<usize>,
        file_size: usize,
    }

    impl Default for MemChunksFile {
        fn default() -> Self {
            Self::new(512)
        }
    }

    impl MemChunksFile {
        /// Creates a new `MemChunksFile` with a specified chunk size.
        pub fn new(chunk_size: usize) -> Self {
            assert!(chunk_size != 0, "chunk size can't be zero");
            MemChunksFile {
                chunks: Vec::new(),
                chunk_size: Some(chunk_size),
                file_size: 0,
            }
        }

        /// Creates a `MemChunksFile` where the chunk size is determined by the size of the first write operation.
        ///
        /// This is often used for the main DB file implementation.
        pub fn waiting_for_write() -> Self {
            MemChunksFile {
                chunks: Vec::new(),
                chunk_size: None,
                file_size: 0,
            }
        }
    }

    impl VfsFile for MemChunksFile {
        fn read(&self, buf: &mut [u8], offset: usize) -> VfsResult<bool> {
            let Some(chunk_size) = self.chunk_size else {
                buf.fill(0);
                return Ok(false);
            };

            if self.file_size <= offset {
                buf.fill(0);
                return Ok(false);
            }

            if chunk_size == buf.len() && offset % chunk_size == 0 {
                buf.copy_from_slice(&self.chunks[offset / chunk_size]);
                Ok(true)
            } else {
                let mut size = buf.len();
                let chunk_idx = offset / chunk_size;
                let mut remaining_idx = offset % chunk_size;
                let mut offset = 0;

                for chunk in &self.chunks[chunk_idx..] {
                    let n = core::cmp::min(chunk_size.min(self.file_size) - remaining_idx, size);
                    buf[offset..offset + n]
                        .copy_from_slice(&chunk[remaining_idx..remaining_idx + n]);
                    offset += n;
                    size -= n;
                    remaining_idx = 0;
                    if size == 0 {
                        break;
                    }
                }

                if offset < buf.len() {
                    buf[offset..].fill(0);
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
        }

        fn write(&mut self, buf: &[u8], offset: usize) -> VfsResult<()> {
            if buf.is_empty() {
                return Ok(());
            }

            let chunk_size = if let Some(chunk_size) = self.chunk_size {
                chunk_size
            } else {
                let size = buf.len();
                self.chunk_size = Some(size);
                size
            };

            let new_length = self.file_size.max(offset + buf.len());

            if chunk_size == buf.len() && offset % chunk_size == 0 {
                for _ in self.chunks.len()..offset / chunk_size {
                    self.chunks.push(vec![0; chunk_size]);
                }
                if let Some(buffer) = self.chunks.get_mut(offset / chunk_size) {
                    buffer.copy_from_slice(buf);
                } else {
                    self.chunks.push(buf.to_vec());
                }
            } else {
                let mut size = buf.len();
                let chunk_start_idx = offset / chunk_size;
                let chunk_end_idx = (offset + size - 1) / chunk_size;
                let chunks_length = self.chunks.len();

                for _ in chunks_length..=chunk_end_idx {
                    self.chunks.push(vec![0; chunk_size]);
                }

                let mut remaining_idx = offset % chunk_size;
                let mut offset = 0;

                for idx in chunk_start_idx..=chunk_end_idx {
                    let n = core::cmp::min(chunk_size - remaining_idx, size);
                    self.chunks[idx][remaining_idx..remaining_idx + n]
                        .copy_from_slice(&buf[offset..offset + n]);
                    offset += n;
                    size -= n;
                    remaining_idx = 0;
                    if size == 0 {
                        break;
                    }
                }
            }

            self.file_size = new_length;

            Ok(())
        }

        fn truncate(&mut self, size: usize) -> VfsResult<()> {
            if let Some(chunk_size) = self.chunk_size {
                if size == 0 {
                    core::mem::take(&mut self.chunks);
                } else {
                    let idx = ((size - 1) / chunk_size) + 1;
                    self.chunks.drain(idx..);
                }
            } else if size != 0 {
                assert_eq!(self.file_size, 0);
                return Err(VfsError::new(SQLITE_IOERR, "Failed to truncate".into()));
            }
            self.file_size = size;
            Ok(())
        }

        fn flush(&mut self) -> VfsResult<()> {
            Ok(())
        }

        fn size(&self) -> VfsResult<usize> {
            Ok(self.file_size)
        }
    }

    /// The core file-handle structure for a custom VFS, designed to be compatible with SQLite's C interface.
    ///
    /// `szOsFile` must be set to the size of `SQLiteVfsFile`.
    #[repr(C)]
    pub struct SQLiteVfsFile {
        /// The first field must be of type sqlite_file.
        /// In C layout, the pointer to SQLiteVfsFile is the pointer to io_methods.
        pub io_methods: sqlite3_file,
        /// The vfs where the file is located, usually used to manage files.
        pub vfs: *mut sqlite3_vfs,
        /// Flags used to open the database.
        pub flags: i32,
        /// The pointer to the file name.
        /// If it is a leaked static pointer, you need to drop it manually when xClose it.
        pub name_ptr: *const u8,
        /// Length of the file name, on wasm32 platform, usize is u32.
        pub name_length: usize,
    }

    impl SQLiteVfsFile {
        /// Convert a `sqlite3_file` pointer to a `SQLiteVfsFile` pointer.
        ///
        /// # Safety
        ///
        /// You must ensure that the pointer passed in is `SQLiteVfsFile`
        pub unsafe fn from_file(file: *mut sqlite3_file) -> &'static SQLiteVfsFile {
            &*file.cast::<Self>()
        }

        /// Get the file name.
        ///
        /// # Safety
        ///
        /// When xClose, you can free the memory by `drop(Box::from_raw(ptr));`.
        ///
        /// Do not use again after free.
        pub unsafe fn name(&self) -> &'static mut str {
            // emm, `from_raw_parts_mut` is unstable
            core::str::from_utf8_unchecked_mut(core::slice::from_raw_parts_mut(
                self.name_ptr.cast_mut(),
                self.name_length,
            ))
        }

        /// Converts a reference to this VFS file structure into a raw `*mut sqlite3_file` pointer that can be passed to SQLite.
        pub fn sqlite3_file(&'static self) -> *mut sqlite3_file {
            self as *const SQLiteVfsFile as *mut sqlite3_file
        }
    }

    /// Represents errors that can occur during the VFS registration process.
    #[derive(Debug)]
    pub enum RegisterVfsError {
        ToCStr,
        RegisterVfs,
    }

    /// Checks if a VFS with the given name is already registered with SQLite and returns a pointer to it if found.
    pub fn registered_vfs(vfs_name: &str) -> Result<Option<*mut sqlite3_vfs>, RegisterVfsError> {
        let name = CString::new(vfs_name).map_err(|_| RegisterVfsError::ToCStr)?;
        let vfs = unsafe { sqlite3_vfs_find(name.as_ptr()) };
        Ok((!vfs.is_null()).then_some(vfs))
    }

    /// A generic function to register a custom VFS implementation with SQLite.
    pub fn register_vfs<IO: SQLiteIoMethods, V: SQLiteVfs<IO>>(
        vfs_name: &str,
        app_data: IO::AppData,
        default_vfs: bool,
    ) -> Result<*mut sqlite3_vfs, RegisterVfsError> {
        let name = CString::new(vfs_name).map_err(|_| RegisterVfsError::ToCStr)?;
        let name_ptr = name.into_raw();

        let app_data = VfsAppData::new(app_data).leak();
        let vfs = Box::leak(Box::new(V::vfs(name_ptr, app_data.cast())));
        let ret = unsafe { sqlite3_vfs_register(vfs, i32::from(default_vfs)) };

        if ret != SQLITE_OK {
            unsafe {
                drop(Box::from_raw(vfs));
                drop(CString::from_raw(name_ptr));
                drop(VfsAppData::from_raw(app_data));
            }
            return Err(RegisterVfsError::RegisterVfs);
        }

        Ok(vfs as *mut sqlite3_vfs)
    }

    /// A container for VFS-specific errors, holding both an error code and a descriptive message.
    #[derive(Debug)]
    pub struct VfsError {
        code: i32,
        message: String,
    }

    impl VfsError {
        pub fn new(code: i32, message: String) -> Self {
            VfsError { code, message }
        }
    }

    /// A specialized `Result` type for VFS operations.
    pub type VfsResult<T> = Result<T, VfsError>;

    /// A wrapper for the `pAppData` pointer in `sqlite3_vfs`, providing a safe way
    /// to manage VFS-specific application data and error states.
    pub struct VfsAppData<T> {
        data: T,
        last_err: RefCell<Option<(i32, String)>>,
    }

    impl<T> VfsAppData<T> {
        pub fn new(t: T) -> Self {
            VfsAppData {
                data: t,
                last_err: RefCell::new(None),
            }
        }

        /// Leak, then pAppData can be set to VFS
        pub fn leak(self) -> *mut Self {
            Box::into_raw(Box::new(self))
        }

        /// # Safety
        ///
        /// You have to make sure the pointer is correct
        pub unsafe fn from_raw(t: *mut Self) -> VfsAppData<T> {
            *Box::from_raw(t)
        }

        /// Retrieves and clears the last error recorded for the VFS.
        pub fn pop_err(&self) -> Option<(i32, String)> {
            self.last_err.borrow_mut().take()
        }

        /// Stores an error code and message for the VFS, to be retrieved later by `xGetLastError`.
        pub fn store_err(&self, err: VfsError) -> i32 {
            let VfsError { code, message } = err;
            self.last_err.borrow_mut().replace((code, message));
            code
        }
    }

    /// Deref only, returns immutable reference, avoids race conditions
    impl<T> Deref for VfsAppData<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    /// A trait defining the basic I/O capabilities required for a VFS file implementation.
    pub trait VfsFile {
        /// Abstraction of `xRead`, returns true for `SQLITE_OK` and false for `SQLITE_IOERR_SHORT_READ`
        fn read(&self, buf: &mut [u8], offset: usize) -> VfsResult<bool>;
        /// Abstraction of `xWrite`
        fn write(&mut self, buf: &[u8], offset: usize) -> VfsResult<()>;
        /// Abstraction of `xTruncate`
        fn truncate(&mut self, size: usize) -> VfsResult<()>;
        /// Abstraction of `xSync`
        fn flush(&mut self) -> VfsResult<()>;
        /// Abstraction of `xFileSize`
        fn size(&self) -> VfsResult<usize>;
    }

    /// Make changes to files
    pub trait VfsStore<File, AppData> {
        /// Convert pAppData to the type we need
        ///
        /// # Safety
        ///
        /// As long as it is set through the abstract VFS interface, it is safe
        unsafe fn app_data(vfs: *mut sqlite3_vfs) -> &'static VfsAppData<AppData> {
            &*(*vfs).pAppData.cast()
        }
        /// Adding files to the Store, use for `xOpen` and `xAccess`
        fn add_file(vfs: *mut sqlite3_vfs, file: &str, flags: i32) -> VfsResult<()>;
        /// Checks if the specified file exists in the Store, use for `xOpen` and `xAccess`
        fn contains_file(vfs: *mut sqlite3_vfs, file: &str) -> VfsResult<bool>;
        /// Delete the specified file in the Store, use for `xClose` and `xDelete`
        fn delete_file(vfs: *mut sqlite3_vfs, file: &str) -> VfsResult<()>;
        /// Read the file contents, use for `xRead`, `xFileSize`
        fn with_file<F: Fn(&File) -> VfsResult<i32>>(
            vfs_file: &SQLiteVfsFile,
            f: F,
        ) -> VfsResult<i32>;
        /// Write the file contents, use for `xWrite`, `xTruncate` and `xSync`
        fn with_file_mut<F: Fn(&mut File) -> VfsResult<i32>>(
            vfs_file: &SQLiteVfsFile,
            f: F,
        ) -> VfsResult<i32>;
    }

    /// A trait that abstracts the `sqlite3_vfs` struct, allowing for a more idiomatic Rust implementation.
    #[allow(clippy::missing_safety_doc)]
    pub trait SQLiteVfs<IO: SQLiteIoMethods> {
        const VERSION: ::core::ffi::c_int;
        const MAX_PATH_SIZE: ::core::ffi::c_int = 1024;

        fn vfs(
            vfs_name: *const ::core::ffi::c_char,
            app_data: *mut VfsAppData<IO::AppData>,
        ) -> sqlite3_vfs {
            sqlite3_vfs {
                iVersion: Self::VERSION,
                szOsFile: core::mem::size_of::<SQLiteVfsFile>() as i32,
                mxPathname: Self::MAX_PATH_SIZE,
                pNext: core::ptr::null_mut(),
                zName: vfs_name,
                pAppData: app_data.cast(),
                xOpen: Some(Self::xOpen),
                xDelete: Some(Self::xDelete),
                xAccess: Some(Self::xAccess),
                xFullPathname: Some(Self::xFullPathname),
                xDlOpen: None,
                xDlError: None,
                xDlSym: None,
                xDlClose: None,
                xRandomness: Some(x_methods_shim::xRandomness),
                xSleep: Some(x_methods_shim::xSleep),
                xCurrentTime: Some(x_methods_shim::xCurrentTime),
                xGetLastError: Some(Self::xGetLastError),
                xCurrentTimeInt64: Some(x_methods_shim::xCurrentTimeInt64),
                xSetSystemCall: None,
                xGetSystemCall: None,
                xNextSystemCall: None,
            }
        }

        unsafe extern "C" fn xOpen(
            pVfs: *mut sqlite3_vfs,
            zName: sqlite3_filename,
            pFile: *mut sqlite3_file,
            flags: ::core::ffi::c_int,
            pOutFlags: *mut ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            Self::xOpenImpl(pVfs, zName, pFile, flags, pOutFlags)
        }

        unsafe extern "C" fn xOpenImpl(
            pVfs: *mut sqlite3_vfs,
            zName: sqlite3_filename,
            pFile: *mut sqlite3_file,
            flags: ::core::ffi::c_int,
            pOutFlags: *mut ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            let app_data = IO::Store::app_data(pVfs);

            let name = if zName.is_null() {
                random_name()
            } else {
                check_result!(CStr::from_ptr(zName).to_str()).into()
            };

            let exist = match IO::Store::contains_file(pVfs, &name) {
                Ok(exist) => exist,
                Err(err) => return app_data.store_err(err),
            };

            if !exist {
                if flags & SQLITE_OPEN_CREATE == 0 {
                    return app_data.store_err(VfsError::new(
                        SQLITE_CANTOPEN,
                        alloc::format!("file not found: {name}"),
                    ));
                }
                if let Err(err) = IO::Store::add_file(pVfs, &name, flags) {
                    return app_data.store_err(err);
                }
            }

            let leak = name.leak();
            let vfs_file = pFile.cast::<SQLiteVfsFile>();
            (*vfs_file).vfs = pVfs;
            (*vfs_file).flags = flags;
            (*vfs_file).name_ptr = leak.as_ptr();
            (*vfs_file).name_length = leak.len();

            (*pFile).pMethods = &IO::METHODS;

            if !pOutFlags.is_null() {
                *pOutFlags = flags;
            }

            SQLITE_OK
        }

        unsafe extern "C" fn xDelete(
            pVfs: *mut sqlite3_vfs,
            zName: *const ::core::ffi::c_char,
            syncDir: ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            unused!(syncDir);

            let app_data = IO::Store::app_data(pVfs);
            bail!(zName.is_null(), SQLITE_IOERR_DELETE);
            let s = check_result!(CStr::from_ptr(zName).to_str());
            if let Err(err) = IO::Store::delete_file(pVfs, s) {
                app_data.store_err(err)
            } else {
                SQLITE_OK
            }
        }

        unsafe extern "C" fn xAccess(
            pVfs: *mut sqlite3_vfs,
            zName: *const ::core::ffi::c_char,
            flags: ::core::ffi::c_int,
            pResOut: *mut ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            unused!(flags);

            *pResOut = if zName.is_null() {
                0
            } else {
                let app_data = IO::Store::app_data(pVfs);
                let file = check_result!(CStr::from_ptr(zName).to_str());
                let exist = match IO::Store::contains_file(pVfs, file) {
                    Ok(exist) => exist,
                    Err(err) => return app_data.store_err(err),
                };
                i32::from(exist)
            };

            SQLITE_OK
        }

        unsafe extern "C" fn xFullPathname(
            pVfs: *mut sqlite3_vfs,
            zName: *const ::core::ffi::c_char,
            nOut: ::core::ffi::c_int,
            zOut: *mut ::core::ffi::c_char,
        ) -> ::core::ffi::c_int {
            unused!(pVfs);
            bail!(zName.is_null() || zOut.is_null(), SQLITE_CANTOPEN);
            let len = CStr::from_ptr(zName).to_bytes_with_nul().len();
            bail!(len > nOut as usize, SQLITE_CANTOPEN);
            zName.copy_to(zOut, len);
            SQLITE_OK
        }

        unsafe extern "C" fn xGetLastError(
            pVfs: *mut sqlite3_vfs,
            nOut: ::core::ffi::c_int,
            zOut: *mut ::core::ffi::c_char,
        ) -> ::core::ffi::c_int {
            let app_data = IO::Store::app_data(pVfs);
            let Some((code, msg)) = app_data.pop_err() else {
                return SQLITE_OK;
            };
            if !zOut.is_null() {
                let nOut = nOut as usize;
                let count = msg.len().min(nOut);
                msg.as_ptr().copy_to(zOut.cast(), count);
                let zero = match nOut.cmp(&msg.len()) {
                    core::cmp::Ordering::Less | core::cmp::Ordering::Equal => nOut,
                    core::cmp::Ordering::Greater => msg.len() + 1,
                };
                if zero > 0 {
                    core::ptr::write(zOut.add(zero - 1), 0);
                }
            }
            code
        }
    }

    /// A trait that abstracts the `sqlite3_io_methods` struct, allowing for a more idiomatic Rust implementation.
    #[allow(clippy::missing_safety_doc)]
    pub trait SQLiteIoMethods {
        type File: VfsFile;
        type AppData: 'static;
        type Store: VfsStore<Self::File, Self::AppData>;

        const VERSION: ::core::ffi::c_int;

        const METHODS: sqlite3_io_methods = sqlite3_io_methods {
            iVersion: Self::VERSION,
            xClose: Some(Self::xClose),
            xRead: Some(Self::xRead),
            xWrite: Some(Self::xWrite),
            xTruncate: Some(Self::xTruncate),
            xSync: Some(Self::xSync),
            xFileSize: Some(Self::xFileSize),
            xLock: Some(Self::xLock),
            xUnlock: Some(Self::xUnlock),
            xCheckReservedLock: Some(Self::xCheckReservedLock),
            xFileControl: Some(Self::xFileControl),
            xSectorSize: Some(Self::xSectorSize),
            xDeviceCharacteristics: Some(Self::xDeviceCharacteristics),
            xShmMap: None,
            xShmLock: None,
            xShmBarrier: None,
            xShmUnmap: None,
            xFetch: None,
            xUnfetch: None,
        };

        unsafe extern "C" fn xClose(pFile: *mut sqlite3_file) -> ::core::ffi::c_int {
            Self::xCloseImpl(pFile)
        }

        unsafe extern "C" fn xCloseImpl(pFile: *mut sqlite3_file) -> ::core::ffi::c_int {
            let vfs_file = SQLiteVfsFile::from_file(pFile);
            let app_data = Self::Store::app_data(vfs_file.vfs);

            if vfs_file.flags & SQLITE_OPEN_DELETEONCLOSE != 0 {
                if let Err(err) = Self::Store::delete_file(vfs_file.vfs, vfs_file.name()) {
                    return app_data.store_err(err);
                }
            }

            drop(Box::from_raw(vfs_file.name()));

            SQLITE_OK
        }

        unsafe extern "C" fn xRead(
            pFile: *mut sqlite3_file,
            zBuf: *mut ::core::ffi::c_void,
            iAmt: ::core::ffi::c_int,
            iOfst: sqlite3_int64,
        ) -> ::core::ffi::c_int {
            let vfs_file = SQLiteVfsFile::from_file(pFile);
            let app_data = Self::Store::app_data(vfs_file.vfs);

            let f = |file: &Self::File| {
                let size = iAmt as usize;
                let offset = iOfst as usize;
                let slice = core::slice::from_raw_parts_mut(zBuf.cast::<u8>(), size);
                let code = if file.read(slice, offset)? {
                    SQLITE_OK
                } else {
                    SQLITE_IOERR_SHORT_READ
                };
                Ok(code)
            };

            match Self::Store::with_file(vfs_file, f) {
                Ok(code) => code,
                Err(err) => app_data.store_err(err),
            }
        }

        unsafe extern "C" fn xWrite(
            pFile: *mut sqlite3_file,
            zBuf: *const ::core::ffi::c_void,
            iAmt: ::core::ffi::c_int,
            iOfst: sqlite3_int64,
        ) -> ::core::ffi::c_int {
            let vfs_file = SQLiteVfsFile::from_file(pFile);
            let app_data = Self::Store::app_data(vfs_file.vfs);

            let f = |file: &mut Self::File| {
                let (offset, size) = (iOfst as usize, iAmt as usize);
                let slice = core::slice::from_raw_parts(zBuf.cast::<u8>(), size);
                file.write(slice, offset)?;
                Ok(SQLITE_OK)
            };

            match Self::Store::with_file_mut(vfs_file, f) {
                Ok(code) => code,
                Err(err) => app_data.store_err(err),
            }
        }

        unsafe extern "C" fn xTruncate(
            pFile: *mut sqlite3_file,
            size: sqlite3_int64,
        ) -> ::core::ffi::c_int {
            let vfs_file = SQLiteVfsFile::from_file(pFile);
            let app_data = Self::Store::app_data(vfs_file.vfs);

            let f = |file: &mut Self::File| {
                file.truncate(size as usize)?;
                Ok(SQLITE_OK)
            };

            match Self::Store::with_file_mut(vfs_file, f) {
                Ok(code) => code,
                Err(err) => app_data.store_err(err),
            }
        }

        unsafe extern "C" fn xSync(
            pFile: *mut sqlite3_file,
            flags: ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            unused!(flags);

            let vfs_file = SQLiteVfsFile::from_file(pFile);
            let app_data = Self::Store::app_data(vfs_file.vfs);

            let f = |file: &mut Self::File| {
                file.flush()?;
                Ok(SQLITE_OK)
            };

            match Self::Store::with_file_mut(vfs_file, f) {
                Ok(code) => code,
                Err(err) => app_data.store_err(err),
            }
        }

        unsafe extern "C" fn xFileSize(
            pFile: *mut sqlite3_file,
            pSize: *mut sqlite3_int64,
        ) -> ::core::ffi::c_int {
            let vfs_file = SQLiteVfsFile::from_file(pFile);
            let app_data = Self::Store::app_data(vfs_file.vfs);

            let f = |file: &Self::File| {
                file.size().map(|size| {
                    *pSize = size as sqlite3_int64;
                })?;
                Ok(SQLITE_OK)
            };

            match Self::Store::with_file(vfs_file, f) {
                Ok(code) => code,
                Err(err) => app_data.store_err(err),
            }
        }

        unsafe extern "C" fn xLock(
            pFile: *mut sqlite3_file,
            eLock: ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            unused!((pFile, eLock));
            SQLITE_OK
        }

        unsafe extern "C" fn xUnlock(
            pFile: *mut sqlite3_file,
            eLock: ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            unused!((pFile, eLock));
            SQLITE_OK
        }

        unsafe extern "C" fn xCheckReservedLock(
            pFile: *mut sqlite3_file,
            pResOut: *mut ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            unused!(pFile);
            *pResOut = 0;
            SQLITE_OK
        }

        unsafe extern "C" fn xFileControl(
            pFile: *mut sqlite3_file,
            op: ::core::ffi::c_int,
            pArg: *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int {
            unused!((pFile, op, pArg));
            SQLITE_NOTFOUND
        }

        unsafe extern "C" fn xSectorSize(pFile: *mut sqlite3_file) -> ::core::ffi::c_int {
            unused!(pFile);
            512
        }

        unsafe extern "C" fn xDeviceCharacteristics(
            pFile: *mut sqlite3_file,
        ) -> ::core::ffi::c_int {
            unused!(pFile);
            0
        }
    }

    /// A module containing shims for VFS methods that are implemented using JavaScript interoperability.
    #[allow(clippy::missing_safety_doc)]
    pub mod x_methods_shim {
        use super::*;

        /// thread::sleep is available when atomics is enabled
        #[cfg(target_feature = "atomics")]
        pub unsafe extern "C" fn xSleep(
            _pVfs: *mut sqlite3_vfs,
            microseconds: ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            todo!();
            // use ::{thread, time::Duration};
            // thread::sleep(Duration::from_micros(microseconds as u64));
            // SQLITE_OK
        }

        #[cfg(not(target_feature = "atomics"))]
        pub unsafe extern "C" fn xSleep(
            _pVfs: *mut sqlite3_vfs,
            _microseconds: ::core::ffi::c_int,
        ) -> ::core::ffi::c_int {
            SQLITE_OK
        }

        /// <https://github.com/sqlite/sqlite/blob/fb9e8e48fd70b463fb7ba6d99e00f2be54df749e/ext/wasm/api/sqlite3-vfs-opfs.c-pp.js#L951>
        pub unsafe extern "C" fn xRandomness(
            _pVfs: *mut sqlite3_vfs,
            nByte: ::core::ffi::c_int,
            zOut: *mut ::core::ffi::c_char,
        ) -> ::core::ffi::c_int {
            // for i in 0..nByte as usize {
            //     *zOut.add(i) = (Math::random() * 255000.0) as _;
            // }
            // nByte
            todo!()
        }

        /// <https://github.com/sqlite/sqlite/blob/fb9e8e48fd70b463fb7ba6d99e00f2be54df749e/ext/wasm/api/sqlite3-vfs-opfs.c-pp.js#L870>
        pub unsafe extern "C" fn xCurrentTime(
            _pVfs: *mut sqlite3_vfs,
            pTimeOut: *mut f64,
        ) -> ::core::ffi::c_int {
            todo!()
            // *pTimeOut = 2440587.5 + (Date::new_0().get_time() / 86400000.0);
            // SQLITE_OK
        }

        /// <https://github.com/sqlite/sqlite/blob/fb9e8e48fd70b463fb7ba6d99e00f2be54df749e/ext/wasm/api/sqlite3-vfs-opfs.c-pp.js#L877>
        pub unsafe extern "C" fn xCurrentTimeInt64(
            _pVfs: *mut sqlite3_vfs,
            pOut: *mut sqlite3_int64,
        ) -> ::core::ffi::c_int {
            // *pOut = ((2440587.5 * 86400000.0) + Date::new_0().get_time()) as sqlite3_int64;
            // SQLITE_OK
            todo!()
        }
    }

    #[derive(Debug)]
    pub enum ImportDbError {
        InvalidDbSize,
        InvalidHeader,
        InvalidPageSize,
    }

    /// Simple verification when importing db, and return page size;
    pub fn check_import_db(bytes: &[u8]) -> Result<usize, ImportDbError> {
        let length = bytes.len();

        if length < 512 || length % 512 != 0 {
            return Err(ImportDbError::InvalidDbSize);
        }

        if SQLITE3_HEADER
            .as_bytes()
            .iter()
            .zip(bytes)
            .any(|(x, y)| x != y)
        {
            return Err(ImportDbError::InvalidHeader);
        }

        // The database page size in bytes.
        // Must be a power of two between 512 and 32768 inclusive, or the value 1 representing a page size of 65536.
        let page_size = u16::from_be_bytes([bytes[16], bytes[17]]);
        let page_size = if page_size == 1 {
            65536
        } else {
            usize::from(page_size)
        };

        Ok(page_size)
    }

    /// Check db and page size, page size must be a power of two between 512 and 65536 inclusive, db size must be a multiple of page size.
    pub fn check_db_and_page_size(db_size: usize, page_size: usize) -> Result<(), ImportDbError> {
        if !(page_size.is_power_of_two() && (512..=65536).contains(&page_size)) {
            return Err(ImportDbError::InvalidPageSize);
        }
        if db_size % page_size != 0 {
            return Err(ImportDbError::InvalidDbSize);
        }
        Ok(())
    }
}
