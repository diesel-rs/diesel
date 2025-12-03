//! memory vfs, used as the default VFS
//!
//! ```rust
//! use sqlite_wasm_rs as ffi;
//!
//! fn open_db() {
//!     // open with memory vfs
//!     let mut db = std::ptr::null_mut();
//!     let ret = unsafe {
//!         ffi::sqlite3_open_v2(
//!             c"mem.db".as_ptr().cast(),
//!             &mut db as *mut _,
//!             ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE,
//!             std::ptr::null()
//!         )
//!     };
//!     assert_eq!(ffi::SQLITE_OK, ret);
//! }
//! ```
//!
//! Data is stored in memory, this is the default vfs, and reading
//! and writing are very fast, after all, in memory.
//!
//! Refresh the page and data will be lost, and you also need to
//! pay attention to the memory size limit of the browser page.

use crate::vfs::utils::{
    check_import_db, ImportDbError, MemChunksFile, SQLiteIoMethods, SQLiteVfs, SQLiteVfsFile,
    VfsAppData, VfsError, VfsFile, VfsResult, VfsStore,
};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ffi::CStr;
use hashbrown::HashMap;
use libsqlite3_sys::*;

const VFS_NAME: &CStr = c"memvfs";

type Result<T> = core::result::Result<T, MemVfsError>;

pub(crate) enum MemFile {
    Main(MemChunksFile),
    Temp(MemChunksFile),
}

impl MemFile {
    fn new(flags: i32) -> Self {
        if flags & SQLITE_OPEN_MAIN_DB == 0 {
            Self::Temp(MemChunksFile::default())
        } else {
            Self::Main(MemChunksFile::waiting_for_write())
        }
    }

    fn file(&self) -> &MemChunksFile {
        let (MemFile::Main(file) | MemFile::Temp(file)) = self;
        file
    }

    fn file_mut(&mut self) -> &mut MemChunksFile {
        let (MemFile::Main(file) | MemFile::Temp(file)) = self;
        file
    }
}

impl VfsFile for MemFile {
    fn read(&self, buf: &mut [u8], offset: usize) -> VfsResult<bool> {
        self.file().read(buf, offset)
    }

    fn write(&mut self, buf: &[u8], offset: usize) -> VfsResult<()> {
        self.file_mut().write(buf, offset)
    }

    fn truncate(&mut self, size: usize) -> VfsResult<()> {
        self.file_mut().truncate(size)
    }

    fn flush(&mut self) -> VfsResult<()> {
        self.file_mut().flush()
    }

    fn size(&self) -> VfsResult<usize> {
        self.file().size()
    }
}

type MemAppData = RefCell<HashMap<String, MemFile>>;

struct MemStore;

impl VfsStore<MemFile, MemAppData> for MemStore {
    fn add_file(vfs: *mut sqlite3_vfs, file: &str, flags: i32) -> VfsResult<()> {
        let app_data = unsafe { Self::app_data(vfs) };
        app_data
            .borrow_mut()
            .insert(file.into(), MemFile::new(flags));
        Ok(())
    }

    fn contains_file(vfs: *mut sqlite3_vfs, file: &str) -> VfsResult<bool> {
        let app_data = unsafe { Self::app_data(vfs) };
        Ok(app_data.borrow().contains_key(file))
    }

    fn delete_file(vfs: *mut sqlite3_vfs, file: &str) -> VfsResult<()> {
        let app_data = unsafe { Self::app_data(vfs) };
        if app_data.borrow_mut().remove(file).is_none() {
            return Err(VfsError::new(
                SQLITE_IOERR_DELETE,
                alloc::format!("{file} not found"),
            ));
        }
        Ok(())
    }

    fn with_file<F: Fn(&MemFile) -> VfsResult<i32>>(
        vfs_file: &SQLiteVfsFile,
        f: F,
    ) -> VfsResult<i32> {
        let name = unsafe { vfs_file.name() };
        let app_data = unsafe { Self::app_data(vfs_file.vfs) };
        match app_data.borrow().get(name) {
            Some(file) => f(file),
            None => Err(VfsError::new(
                SQLITE_IOERR,
                alloc::format!("{name} not found"),
            )),
        }
    }

    fn with_file_mut<F: Fn(&mut MemFile) -> VfsResult<i32>>(
        vfs_file: &SQLiteVfsFile,
        f: F,
    ) -> VfsResult<i32> {
        let name = unsafe { vfs_file.name() };
        let app_data = unsafe { Self::app_data(vfs_file.vfs) };
        match app_data.borrow_mut().get_mut(name) {
            Some(file) => f(file),
            None => Err(VfsError::new(
                SQLITE_IOERR,
                alloc::format!("{name} not found"),
            )),
        }
    }
}

struct MemIoMethods;

impl SQLiteIoMethods for MemIoMethods {
    type File = MemFile;
    type AppData = MemAppData;
    type Store = MemStore;

    const VERSION: ::core::ffi::c_int = 1;
}

struct MemVfs;

impl SQLiteVfs<MemIoMethods> for MemVfs {
    const VERSION: ::core::ffi::c_int = 1;
}

#[derive(Debug)]
pub enum MemVfsError {
    ImportDb(ImportDbError),
    Generic(String),
}

impl From<ImportDbError> for MemVfsError {
    fn from(e: ImportDbError) -> Self {
        Self::ImportDb(e)
    }
}

/// MemVfs management tool.
pub struct MemVfsUtil(&'static VfsAppData<MemAppData>);

impl Default for MemVfsUtil {
    fn default() -> Self {
        MemVfsUtil::new()
    }
}

impl MemVfsUtil {
    /// Get management tool
    pub fn new() -> Self {
        MemVfsUtil(unsafe { install() })
    }
}

impl MemVfsUtil {
    fn import_db_unchecked_impl(
        &self,
        filename: &str,
        bytes: &[u8],
        page_size: usize,
        clear_wal: bool,
    ) -> Result<()> {
        if self.exists(filename) {
            return Err(MemVfsError::Generic(alloc::format!(
                "{filename} file already exists"
            )));
        }

        self.0.borrow_mut().insert(filename.into(), {
            let mut file = MemFile::Main(MemChunksFile::new(page_size));
            file.write(bytes, 0).unwrap();
            if clear_wal {
                file.write(&[1, 1], 18).unwrap();
            }
            file
        });

        Ok(())
    }

    /// Import the database.
    ///
    /// If the database is imported with WAL mode enabled,
    /// it will be forced to write back to legacy mode, see
    /// <https://sqlite.org/forum/forumpost/67882c5b04>
    ///
    /// If the imported database is encrypted, use `import_db_unchecked` instead.
    pub fn import_db(&self, filename: &str, bytes: &[u8]) -> Result<()> {
        let page_size = check_import_db(bytes)?;
        self.import_db_unchecked_impl(filename, bytes, page_size, true)
    }

    /// `import_db` without checking, can be used to import encrypted database.
    pub fn import_db_unchecked(
        &self,
        filename: &str,
        bytes: &[u8],
        page_size: usize,
    ) -> Result<()> {
        self.import_db_unchecked_impl(filename, bytes, page_size, false)
    }

    /// Export the database.
    pub fn export_db(&self, filename: &str) -> Result<Vec<u8>> {
        let name2file = self.0.borrow();

        if let Some(file) = name2file.get(filename) {
            let file_size = file.size().unwrap();
            let mut ret = alloc::vec![0; file_size];
            file.read(&mut ret, 0).unwrap();
            Ok(ret)
        } else {
            Err(MemVfsError::Generic(
                "The file to be exported does not exist".into(),
            ))
        }
    }

    /// Delete the specified database, make sure that the database is closed.
    pub fn delete_db(&self, filename: &str) {
        self.0.borrow_mut().remove(filename);
    }

    /// Delete all database, make sure that all database is closed.
    pub fn clear_all(&self) {
        core::mem::take(&mut *self.0.borrow_mut());
    }

    /// Does the database exists.
    pub fn exists(&self, filename: &str) -> bool {
        self.0.borrow().contains_key(filename)
    }

    /// List all files.
    pub fn list(&self) -> Vec<String> {
        self.0.borrow().keys().cloned().collect()
    }

    /// Number of files.
    pub fn count(&self) -> usize {
        self.0.borrow().len()
    }
}

pub(crate) unsafe fn install() -> &'static VfsAppData<MemAppData> {
    let vfs = sqlite3_vfs_find(VFS_NAME.as_ptr());

    let vfs = if vfs.is_null() {
        let vfs = Box::leak(Box::new(MemVfs::vfs(
            VFS_NAME.as_ptr(),
            VfsAppData::new(MemAppData::default()).leak(),
        )));
        assert_eq!(
            sqlite3_vfs_register(vfs, 1),
            SQLITE_OK,
            "failed to register memvfs"
        );
        vfs as *mut sqlite3_vfs
    } else {
        vfs
    };

    MemStore::app_data(vfs)
}

pub(crate) unsafe fn uninstall() {
    let vfs = sqlite3_vfs_find(VFS_NAME.as_ptr());

    if !vfs.is_null() {
        assert_eq!(
            sqlite3_vfs_unregister(vfs),
            SQLITE_OK,
            "failed to unregister memvfs"
        );
        drop(VfsAppData::<MemAppData>::from_raw(
            (*vfs).pAppData as *mut _,
        ));
        drop(Box::from_raw(vfs));
    }
}
