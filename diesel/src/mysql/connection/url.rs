extern crate url;

use std::ffi::{CString, CStr};
use self::url::Url;

use result::{ConnectionResult, ConnectionError};

pub struct ConnectionOptions {
    host: Option<CString>,
    user: CString,
    password: Option<CString>,
    database: Option<CString>,
    port: Option<u16>,
    ssl_mode: Option<SslMode>,
}

#[derive(Clone, Copy, Debug)]
pub enum SslMode {
    Disabled,
    Preferred,
    Required,
    VerifyCa,
    VerifyIdentity,
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

        let host = match url.host_str() {
            Some(host) => Some(try!(CString::new(host.as_bytes()))),
            None => None,
        };
        let user = try!(CString::new(url.username().as_bytes()));
        let password = match url.password() {
            Some(password) => Some(try!(CString::new(password.as_bytes()))),
            None => None,
        };
        let database = match url.path_segments().and_then(|mut iter| iter.nth(0)) {
            Some("") | None => None,
            Some(segment) => Some(try!(CString::new(segment.as_bytes()))),
        };
        let ssl_mode = url.query_pairs().find(|&(ref k, _)| k == "sslmode")
            .map(|(_, v)| match &*v {
                "disable" | "disabled" => Ok(Some(SslMode::Disabled)),
                "prefer" | "preferred" => Ok(Some(SslMode::Preferred)),
                "require" | "required" => Ok(Some(SslMode::Required)),
                "verify-ca" | "verify_ca" => Ok(Some(SslMode::VerifyCa)),
                "verify-identity" |
                "verify_identity" => Ok(Some(SslMode::VerifyIdentity)),
                _ => {
                    let msg = format!("Invalid sslmode `{}`. Valid options are \
                        `disable`, `prefer`, `require`, `verify-ca`, or \
                        `verify-identity`.", v);
                    Err(ConnectionError::InvalidConnectionUrl(msg.into()))
                }
            }).unwrap_or(Ok(None));

        Ok(ConnectionOptions {
            host: host,
            user: user,
            password: password,
            database: database,
            port: url.port(),
            ssl_mode: ssl_mode?,
        })
    }

    pub fn host(&self) -> Option<&CStr> {
        self.host.as_ref().map(|x| &**x)
    }

    pub fn user(&self) -> &CStr {
        &self.user
    }

    pub fn password(&self) -> Option<&CStr> {
        self.password.as_ref().map(|x| &**x)
    }

    pub fn database(&self) -> Option<&CStr> {
        self.database.as_ref().map(|x| &**x)
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn ssl_mode(&self) -> Option<SslMode> {
        self.ssl_mode
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
    let foo_cstr = CString::new("foo").unwrap();
    let bar_cstr = CString::new("bar").unwrap();
    assert_eq!(
        Some(&*foo_cstr),
        ConnectionOptions::parse("mysql://localhost/foo").unwrap().database()
    );
    assert_eq!(
        Some(&*bar_cstr),
        ConnectionOptions::parse("mysql://localhost/bar").unwrap().database()
    );
    assert_eq!(
        None,
        ConnectionOptions::parse("mysql://localhost").unwrap().database()
    );
}

#[test]
fn sslmode_is_extracted_from_url() {
    fn ssl_mode(url: &str) -> ConnectionResult<Option<SslMode>> {
        ConnectionOptions::parse(url).map(|opts| opts.ssl_mode())
    }

    assert_matches!(
        ssl_mode("mysql://localhost"),
        Ok(None)
    );
    assert_matches!(
        ssl_mode("mysql://localhost?sslmode=invalid"),
        Err(ConnectionError::InvalidConnectionUrl(_))
    );
    assert_matches!(
        ssl_mode("mysql://localhost?sslmode=disable"),
        Ok(Some(SslMode::Disabled))
    );
    assert_matches!(
        ssl_mode("mysql://localhost?sslmode=prefer"),
        Ok(Some(SslMode::Preferred))
    );
    assert_matches!(
        ssl_mode("mysql://localhost?sslmode=require"),
        Ok(Some(SslMode::Required))
    );
    assert_matches!(
        ssl_mode("mysql://localhost?sslmode=verify-ca"),
        Ok(Some(SslMode::VerifyCa))
    );
    assert_matches!(
        ssl_mode("mysql://localhost?sslmode=verify-identity"),
        Ok(Some(SslMode::VerifyIdentity))
    );
}
