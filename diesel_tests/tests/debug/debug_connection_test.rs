extern crate log;
use self::log::{LogRecord, LogLevel, LogMetadata, SetLoggerError, LogLevelFilter};
use std::sync::Mutex;

use diesel::*;
use schema::*;

lazy_static! {
    static ref MESSAGE_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref LEVEL_BUFFER: Mutex<Vec<LogLevel>> = Mutex::new(Vec::new());
}

struct TestLogger;

impl log::Log for TestLogger {
    fn enabled(&self, _metadata: &LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        let mut m = MESSAGE_BUFFER.try_lock().unwrap();
        m.push(format!("{}",record.args()));
        let mut l = LEVEL_BUFFER.try_lock().unwrap();
        l.push(record.level());
    }
}

impl TestLogger {
    fn init() -> Result<(), SetLoggerError> {
        log::set_logger(|max_log_level| {
            max_log_level.set(LogLevelFilter::Trace);
            Box::new(TestLogger)
        })
    }
}


fn with_logging<F>(work: F) -> (Vec<String>, Vec<LogLevel>) where F: FnOnce() -> () {
    let _ = TestLogger::init();
    {
        let mut m = MESSAGE_BUFFER.lock().unwrap();
        m.clear();
        let mut l = LEVEL_BUFFER.lock().unwrap();
        l.clear();
    }
    work();
    let mut m = MESSAGE_BUFFER.lock().unwrap();
    let mut l = LEVEL_BUFFER.lock().unwrap();
    let messages = m.clone();
    let levels = l.clone();
    m.clear();
    l.clear();
    (messages, levels)
}

#[test]
fn simple_select_log() {
    use schema::users::dsl::*;
    let connection = connection();

    let (messages, levels) = with_logging(||{
        let _ = users.load::<User>(&connection).unwrap();
        let _ = users.filter(id.eq(1)).load::<User>(&connection).unwrap();
    });
    assert_eq!(levels, vec![LogLevel::Debug, LogLevel::Debug]);
    if cfg!(feature = "pg"){
        assert_eq!(messages, vec!["QueryAll: SELECT \"users\".\"id\", \"users\".\"name\", \"users\".\"hair_color\" FROM \"users\"", "QueryAll: SELECT \"users\".\"id\", \"users\".\"name\", \"users\".\"hair_color\" FROM \"users\" WHERE \"users\".\"id\" = $1"]);
    } else if cfg!(feature = "sq"){
        assert_eq!(messages, vec!["QueryAll: SELECT `users`.`id`, `users`.`name`, `users`.`hair_color` FROM `users`", "QueryAll: SELECT `users`.`id`, `users`.`name`, `users`.`hair_color` FROM `users` WHERE `users`.`id` = ?"])
    } else {
        panic!("unknown database system");
    }
}

#[test]
fn simple_insert_log() {
    use schema::users::dsl::*;
    let connection = connection();

    let (messages, levels) = with_logging(||{
        let u = NewUser::new("Sean", Some("Black"));
        insert(&u).into(users).execute(&connection).unwrap();
    });
    assert_eq!(levels, vec![LogLevel::Debug]);
    if cfg!(feature = "pg") {
        assert_eq!(messages, vec!["execute_returing_count: INSERT INTO \"users\" (\"name\", \"hair_color\") VALUES ($1, $2)"]);
    } else if cfg!(feature = "sq") {
        assert_eq!(messages, vec!["execute_returing_count: INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?)"]);
    } else {
        panic!("unknown database system");
    }
}
