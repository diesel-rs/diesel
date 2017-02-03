extern crate url;

use std::ffi::{CString, NulError};
use self::url::Url;

use result::{ConnectionResult, ConnectionError};

pub struct ConnectionOptions {
    url: Url,
}

impl ConnectionOptions {
    pub fn parse(database_url: &str) -> ConnectionResult<Self> {
        let url = match Url::parse(database_url) {
            Ok(url) => url,
            Err(_) => return Err(connection_url_error())
        };

        if url.scheme() != "mysql" {
            return Err(connection_url_error());
        }

        if url.path_segments().map(|x| x.count()).unwrap_or(0) > 1 {
            return Err(connection_url_error());
        }

        Ok(ConnectionOptions {
            url: url,
        })
    }

    pub fn host(&self) -> Result<Option<CString>, NulError> {
        match self.url.host_str() {
            Some(host) => CString::new(host.as_bytes()).map(Some),
            None => Ok(None),
        }
    }

    pub fn user(&self) -> Result<CString, NulError> {
        CString::new(self.url.username().as_bytes())
    }

    pub fn password(&self) -> Result<Option<CString>, NulError> {
        match self.url.password() {
            Some(pw) => CString::new(pw.as_bytes()).map(Some),
            None => Ok(None),
        }
    }

    pub fn database(&self) -> Result<Option<CString>, NulError> {
        match self.url.path_segments().and_then(|mut iter| iter.nth(0)) {
            Some("") | None => Ok(None),
            Some(segment) => CString::new(segment.as_bytes()).map(Some),
        }
    }

    pub fn port(&self) -> Option<u16> {
        self.url.port()
    }
}

fn connection_url_error() -> ConnectionError {
    let msg = "MySQL connection URLs must be in the form \
        `mysql://[[user]:[password]@]host[:port][/database]`";
    ConnectionError::InvalidConnectionUrl(msg.into())
}

#[test]
fn urls_with_schemes_other_than_mysql_are_errors() {
    assert!(ConnectionOptions::parse("postgres://localhost").is_err());
    assert!(ConnectionOptions::parse("http://localhost").is_err());
    assert!(ConnectionOptions::parse("file:///tmp/mysql.sock").is_err());
    assert!(ConnectionOptions::parse("socket:///tmp/mysql.sock").is_err());
    assert!(ConnectionOptions::parse("mysql://localhost").is_ok());
}

#[test]
fn urls_must_have_zero_or_one_path_segments() {
    assert!(ConnectionOptions::parse("mysql://localhost/foo/bar").is_err());
    assert!(ConnectionOptions::parse("mysql://localhost/foo").is_ok());
}

#[test]
fn first_path_segment_is_treated_as_database() {
    let foo_cstr = CString::new("foo".as_bytes()).unwrap();
    let bar_cstr = CString::new("bar".as_bytes()).unwrap();
    assert_eq!(
        Ok(Some(foo_cstr)),
        ConnectionOptions::parse("mysql://localhost/foo").unwrap().database()
    );
    assert_eq!(
        Ok(Some(bar_cstr)),
        ConnectionOptions::parse("mysql://localhost/bar").unwrap().database()
    );
    assert_eq!(
        Ok(None),
        ConnectionOptions::parse("mysql://localhost").unwrap().database()
    );
}
