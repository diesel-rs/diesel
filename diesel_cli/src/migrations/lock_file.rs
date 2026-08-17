use std::{
    fs::{File, remove_file},
    os::fd::AsFd,
    path::PathBuf,
};

/// Wrapper around a file and its path, deletes the file on drop
pub struct LockFile {
    pub path: PathBuf,
    pub file: File,
}

impl AsFd for LockFile {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // Ignore the error, best not to panic here.
        let _ = remove_file(&self.path);
    }
}
