#![allow(warnings)]
#![no_std]
#![no_main]
extern crate alloc;
extern crate tinyrlibc;

use alloc::string::String;
use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_println::{print, println};

use core::ffi::{c_char, c_int, c_void};

esp_bootloader_esp_idf::esp_app_desc!();

use diesel::connection::SimpleConnection;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

struct EmbeddedVfs;

impl rsqlite_vfs::OsCallback for EmbeddedVfs {
    fn sleep(_: core::time::Duration) {
        unimplemented!("Not called by the demo")
    }
    fn random(_: &mut [u8]) {
        unimplemented!("Not called by the demo")
    }
    fn epoch_timestamp_in_ms() -> i64 {
        unimplemented!("Not called by the demo")
    }
}

#[unsafe(no_mangle)]
extern "C" fn sqlite3_os_init() -> core::ffi::c_int {
    println!("Register a basic memory VFS implementation");
    unsafe {
        rsqlite_vfs::memvfs::install::<EmbeddedVfs>();
    }
    libsqlite3_sys::SQLITE_OK
}

#[esp_hal::main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let peripherals = esp_hal::init(config);

    println!("Firmware starting");

    esp_alloc::heap_allocator!(size: 100 * 1024);
    println!("Before sqlite");
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.set_instrumentation(|event: diesel::connection::InstrumentationEvent<'_>| {
        println!("Execute query: {event:?}")
    });

    conn.batch_execute("CREATE TABLE users(id INTEGER NOT NULL PRIMARY KEY, name TEXT NOT NULL)")
        .unwrap();

    diesel::insert_into(users::table)
        .values([users::name.eq("John"), users::name.eq("Jane")])
        .execute(&mut conn)
        .unwrap();

    let delay = Delay::new();
    loop {
        let data = users::table
            .filter(users::name.eq("Jane"))
            .first::<(i32, String)>(&mut conn)
            .unwrap();
        println!("----");
        println!("Query data:");
        println!("Loaded user data: {} -> {}", data.0, data.1);
        println!("---");
        delay.delay_millis(500);
    }
}
