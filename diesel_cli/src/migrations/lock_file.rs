use std::{
    fs::{File, remove_file},
    path::PathBuf,
};

/// Wrapper around a file and its path, deletes the file on drop
pub struct LockFile {
    pub path: PathBuf,
    pub file: File,
}

#[cfg(unix)]
impl std::os::fd::AsFd for LockFile {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(windows)]
impl std::os::windows::io::AsHandle for LockFile {
    fn as_handle(&self) -> std::os::windows::io::BorrowedHandle<'_> {
        self.file.as_handle()
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // Ignore the error, best not to panic here.
        let _ = remove_file(&self.path);
    }
}
