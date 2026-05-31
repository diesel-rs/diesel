# Running Diesel + Sqlite on an embedded target

This example project assumes that you use an [`esp32-c6`](https://www.espressif.com/en/products/socs/esp32-c6). For other embedded targets you need to make adjustments to fit the project to your target platform. Please checkout the documentation of the corresponding hal crate to get information about the setup for your microcontroller.

Diesel supports running in a no-std mode. For this you need to add diesel like this to your project:

```toml
diesel = { version = "2.3.0", path = "../../../diesel/", default-features = false, features = ["sqlite", "hashbrown"] }
```

The important point here are:

* Disabling default features to disable the `std` feature
* Only the `sqlite` backend is supported for embedded targets
* Enabling the `hashbrown` feature, otherwise diesel cannot be compiled

You also need to add `libsqlite3-sys` to your cargo toml to enable the bundled build feature:

``` toml
libsqlite3-sys = { version = "0.35.0", features = ["bundled"] }
```

Finally you need to depend on `tinyrlibc` (or a similar crate) to provide some libc functions required by libsqlite3:

```toml
tinyrlibc = "0.5"
```

To compile diesel + the sqlite library in no-std mode you need to perform the following steps:

* Get a cross compiler for your target, for the `esp32-c6` you can get it from [here](https://github.com/riscv-collab/riscv-gnu-toolchain)
* Set `TARGET_CC` to point to the cross compiler. For the `esp32-c6` this needs to be set to `riscv32-unknown-elf-gcc` (or the fully qualified path to that binary)
* Set `LIBSQLITE3_FLAGS` to configure building libsqlite for an embedded target. This need to be set to `-DSQLITE_SMALL_STACK=1 -DSQLITE_THREADSAFE=0 -DSQLITE_OS_OTHER=1 -DSQLITE_OMIT_LOCALTIME=1 -USQLITE_ENABLE_FTS3 -UQLITE_ENABLE_DBSTAT_VTAB -UDSQLITE_ENABLE_JSON1 -USQLITE_ENABLE_FTS5 -USQLITE_ENABLE_STAT4 -DSQLITE_ENABLE_MEMSYS3=1` or similar.

You application needs to use the `alloc` crate and provide an allocator implementation as Diesel requires support for allocations.


Finally your application needs to provide a `sqlite3_os_init()` function to configure SQLite for the embedded target. This function needs to register at least one VFS. For demonstration purposes you can use the `sqlite_memvfs` crate there:

``` rust
#[unsafe(no_mangle)]
extern "C" fn sqlite3_os_init() -> core::ffi::c_int {
    sqlite_memvfs::install();
    libsqlite3_sys::SQLITE_OK
}

```
