extern crate percent_encoding;
extern crate url;

use self::percent_encoding::percent_decode;
use self::url::{Host, Url};
use std::collections::HashMap;
use std::ffi::{CStr, CString};

use crate::result::{ConnectionError, ConnectionResult};

pub struct ConnectionOptions {
    host: Option<CString>,
    user: CString,
    password: Option<CString>,
    database: Option<CString>,
    port: Option<u16>,
    unix_socket: Option<CString>,
}

impl ConnectionOptions {
    pub fn parse(database_url: &str) -> ConnectionResult<Self> {
        let url = match Url::parse(database_url) {
            Ok(url) => url,
            Err(_) => return Err(connection_url_error()),
        };

        if url.scheme() != "mysql" {
            return Err(connection_url_error());
        }

        if url.path_segments().map(Iterator::count).unwrap_or(0) > 1 {
            return Err(connection_url_error());
        }

        let query_pairs = url.query_pairs().into_owned().collect::<HashMap<_, _>>();
        if query_pairs.get("database").is_some() {
            return Err(connection_url_error());
        }

        let unix_socket = match query_pairs.get("unix_socket") {
            Some(v) => Some(CString::new(v.as_bytes())?),
            _ => None,
        };

        let host = match url.host() {
            Some(Host::Ipv6(host)) => Some(CString::new(host.to_string())?),
            Some(host) if host.to_string() == "localhost" && unix_socket != None => None,
            Some(host) => Some(CString::new(host.to_string())?),
            None => None,
        };
        let user = decode_into_cstring(url.username())?;
        let password = match url.password() {
            Some(password) => Some(decode_into_cstring(password)?),
            None => None,
        };

        let database = match url.path_segments().and_then(|mut iter| iter.next()) {
            Some("") | None => None,
            Some(segment) => Some(CString::new(segment.as_bytes())?),
        };

        Ok(ConnectionOptions {
            host: host,
            user: user,
            password: password,
            database: database,
            port: url.port(),
            unix_socket: unix_socket,
        })
    }

    pub fn host(&self) -> Option<&CStr> {
        self.host.as_deref()
    }

    pub fn user(&self) -> &CStr {
        &self.user
    }

    pub fn password(&self) -> Option<&CStr> {
        self.password.as_deref()
    }

    pub fn database(&self) -> Option<&CStr> {
        self.database.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn unix_socket(&self) -> Option<&CStr> {
        self.unix_socket.as_deref()
    }
}

fn decode_into_cstring(s: &str) -> ConnectionResult<CString> {
    let decoded = percent_decode(s.as_bytes())
        .decode_utf8()
        .map_err(|_| connection_url_error())?;
    CString::new(decoded.as_bytes()).map_err(Into::into)
}

fn connection_url_error() -> ConnectionError {
    let msg = "MySQL connection URLs must be in the form \
               `mysql://[[user]:[password]@]host[:port][/database][?unix_socket=socket-path]`";
    ConnectionError::InvalidConnectionUrl(msg.into())
}

#[test]
fn urls_with_schemes_other_than_mysql_are_errors() {
    assert!(ConnectionOptions::parse("postgres://localhost").is_err());
    assert!(ConnectionOptions::parse("http://localhost").is_err());
    assert!(ConnectionOptions::parse("file:///tmp/mysql.sock").is_err());
    assert!(ConnectionOptions::parse("socket:///tmp/mysql.sock").is_err());
    assert!(ConnectionOptions::parse("mysql://localhost?database=somedb").is_err());
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
    use self::percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
    const USERINFO_ENCODE_SET: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'<')
        .add(b'>')
        .add(b'`')
        .add(b'#')
        .add(b'?')
        .add(b'{')
        .add(b'}')
        .add(b'/')
        .add(b':')
        .add(b';')
        .add(b'=')
        .add(b'@')
        .add(b'[')
        .add(b'\\')
        .add(b']')
        .add(b'^')
        .add(b'|');

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

#[test]
fn unix_socket_tests() {
    let unix_socket = "/var/run/mysqld.sock";
    let username = "foo";
    let password = "bar";
    let db_url = format!(
        "mysql://{}:{}@localhost?unix_socket={}",
        username, password, unix_socket
    );
    let conn_opts = ConnectionOptions::parse(db_url.as_str()).unwrap();
    let cstring = |s| CString::new(s).unwrap();
    assert_eq!(None, conn_opts.host);
    assert_eq!(None, conn_opts.port);
    assert_eq!(cstring(username), conn_opts.user);
    assert_eq!(cstring(password), conn_opts.password.unwrap());
    assert_eq!(
        CString::new(unix_socket).unwrap(),
        conn_opts.unix_socket.unwrap()
    );
}
