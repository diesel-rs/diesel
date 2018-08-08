//! The internal methods used to parse the database URL.
//!
//! Normally, you won't need to use this functionality, but
//! it can be useful, for example to print the database credentials
//! without the password.
//!
//! This struct uses "C" strings so that the system libraries for
//! the mysql backend (written in "C") work correctly.
//!
//! # Examples
//!
//! ```rust
//! // We can safely unwrap here as we have hard-coded a valid url.
//! let opts = ConnectionOptions::parse("mysql://root:password@127.0.0.1:3306/db_name").unwrap();
//! assert_eq!(opts.host, &CString::new("127.0.0.1"));
//! assert_eq!(opts.user, &CString::new("root"));
//! assert_eq!(opts.password, Some(&CString::new("password")));
//! assert_eq!(opts.database, Some(&CString::new("db_name")));
//! assert_eq!(opts.port, Some(3306));
//! // prints the url without the password.
//! // (We know this won't panic because we created the url, in general you'd
//! // need to be more clever when handling percent-encoded urls and optional
//! // parts).
//! println!("mysql://{}:xxx@{}:{}/{}",
//!          opts.user.to_str().unwrap(),
//!          opts.host.to_str().unwrap(),
//!          opts.port.unwrap(),
//!          opts.database.unwrap().to_str().unwrap());
//! ```
extern crate url;

use self::url::percent_encoding::percent_decode;
use self::url::{Host, Url};
use std::ffi::{CStr, CString};

use result::{ConnectionError, ConnectionResult};

/// The connections options for a mysql connection.
///
/// The only way to create an instance of this struct is using
/// `ConnectionOptions::parse`.
pub struct ConnectionOptions {
    host: Option<CString>,
    user: CString,
    password: Option<CString>,
    database: Option<CString>,
    port: Option<u16>,
}

impl ConnectionOptions {
    /// Takes a connection string, and parses it into connection options.
    pub fn parse(database_url: &str) -> ConnectionResult<Self> {
        let url = match Url::parse(database_url) {
            Ok(url) => url,
            Err(_) => return Err(connection_url_error()),
        };

        if url.scheme() != "mysql" {
            return Err(connection_url_error());
        }

        if url.path_segments().map(|x| x.count()).unwrap_or(0) > 1 {
            return Err(connection_url_error());
        }

        let host = match url.host() {
            Some(Host::Ipv6(host)) => Some(try!(CString::new(host.to_string()))),
            Some(host) => Some(try!(CString::new(host.to_string()))),
            None => None,
        };
        let user = try!(decode_into_cstring(url.username()));
        let password = match url.password() {
            Some(password) => Some(try!(decode_into_cstring(password))),
            None => None,
        };
        let database = match url.path_segments().and_then(|mut iter| iter.nth(0)) {
            Some("") | None => None,
            Some(segment) => Some(try!(CString::new(segment.as_bytes()))),
        };

        Ok(ConnectionOptions {
            host: host,
            user: user,
            password: password,
            database: database,
            port: url.port(),
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
}

fn decode_into_cstring(s: &str) -> ConnectionResult<CString> {
    let decoded = try!(
        percent_decode(s.as_bytes())
            .decode_utf8()
            .map_err(|_| connection_url_error())
    );
    CString::new(decoded.as_bytes()).map_err(Into::into)
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
        ConnectionOptions::parse("mysql://localhost/foo")
            .unwrap()
            .database()
    );
    assert_eq!(
        Some(&*bar_cstr),
        ConnectionOptions::parse("mysql://localhost/bar")
            .unwrap()
            .database()
    );
    assert_eq!(
        None,
        ConnectionOptions::parse("mysql://localhost")
            .unwrap()
            .database()
    );
}

#[test]
fn userinfo_should_be_percent_decode() {
    use self::url::percent_encoding::{utf8_percent_encode, USERINFO_ENCODE_SET};

    let username = "x#gfuL?4Zuj{n73m}eeJt0";
    let encoded_username = utf8_percent_encode(username, USERINFO_ENCODE_SET);

    let password = "x/gfuL?4Zuj{n73m}eeJt1";
    let encoded_password = utf8_percent_encode(password, USERINFO_ENCODE_SET);

    let db_url = format!(
        "mysql://{}:{}@localhost/bar",
        encoded_username, encoded_password
    );
    let db_url = Url::parse(&db_url).unwrap();

    let conn_opts = ConnectionOptions::parse(db_url.as_str()).unwrap();
    let username = CString::new(username.as_bytes()).unwrap();
    let password = CString::new(password.as_bytes()).unwrap();
    assert_eq!(username, conn_opts.user);
    assert_eq!(password, conn_opts.password.unwrap());
}

#[test]
fn ipv6_host_not_wrapped_in_brackets() {
    let host1 = CString::new("::1").unwrap();
    let host2 = CString::new("2001:db8:85a3::8a2e:370:7334").unwrap();

    assert_eq!(
        Some(&*host1),
        ConnectionOptions::parse("mysql://[::1]").unwrap().host()
    );
    assert_eq!(
        Some(&*host2),
        ConnectionOptions::parse("mysql://[2001:db8:85a3::8a2e:370:7334]")
            .unwrap()
            .host()
    );
}
