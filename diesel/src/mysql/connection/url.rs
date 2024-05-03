extern crate percent_encoding;
extern crate url;

use self::percent_encoding::percent_decode;
use self::url::{Host, Url};
use std::collections::HashMap;
use std::ffi::{CStr, CString};

use crate::result::{ConnectionError, ConnectionResult};

use mysqlclient_sys::mysql_ssl_mode;

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct CapabilityFlags: u32 {
        const CLIENT_LONG_PASSWORD = 0x00000001;
        const CLIENT_FOUND_ROWS = 0x00000002;
        const CLIENT_LONG_FLAG = 0x00000004;
        const CLIENT_CONNECT_WITH_DB = 0x00000008;
        const CLIENT_NO_SCHEMA = 0x00000010;
        const CLIENT_COMPRESS = 0x00000020;
        const CLIENT_ODBC = 0x00000040;
        const CLIENT_LOCAL_FILES = 0x00000080;
        const CLIENT_IGNORE_SPACE = 0x00000100;
        const CLIENT_PROTOCOL_41 = 0x00000200;
        const CLIENT_INTERACTIVE = 0x00000400;
        const CLIENT_SSL = 0x00000800;
        const CLIENT_IGNORE_SIGPIPE = 0x00001000;
        const CLIENT_TRANSACTIONS = 0x00002000;
        const CLIENT_RESERVED = 0x00004000;
        const CLIENT_SECURE_CONNECTION = 0x00008000;
        const CLIENT_MULTI_STATEMENTS = 0x00010000;
        const CLIENT_MULTI_RESULTS = 0x00020000;
        const CLIENT_PS_MULTI_RESULTS = 0x00040000;
        const CLIENT_PLUGIN_AUTH = 0x00080000;
        const CLIENT_CONNECT_ATTRS = 0x00100000;
        const CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA = 0x00200000;
        const CLIENT_CAN_HANDLE_EXPIRED_PASSWORDS = 0x00400000;
        const CLIENT_SESSION_TRACK = 0x00800000;
        const CLIENT_DEPRECATE_EOF = 0x01000000;
    }
}

pub(super) struct ConnectionOptions {
    host: Option<CString>,
    user: CString,
    password: Option<CString>,
    database: Option<CString>,
    port: Option<u16>,
    unix_socket: Option<CString>,
    client_flags: CapabilityFlags,
    ssl_mode: Option<mysql_ssl_mode>,
    ssl_ca: Option<CString>,
    ssl_cert: Option<CString>,
    ssl_key: Option<CString>,
}

impl ConnectionOptions {
    pub(super) fn parse(database_url: &str) -> ConnectionResult<Self> {
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
        if query_pairs.contains_key("database") {
            return Err(connection_url_error());
        }

        let unix_socket = match query_pairs.get("unix_socket") {
            Some(v) => Some(CString::new(v.as_bytes())?),
            _ => None,
        };

        let ssl_ca = match query_pairs.get("ssl_ca") {
            Some(v) => Some(CString::new(v.as_bytes())?),
            _ => None,
        };

        let ssl_cert = match query_pairs.get("ssl_cert") {
            Some(v) => Some(CString::new(v.as_bytes())?),
            _ => None,
        };

        let ssl_key = match query_pairs.get("ssl_key") {
            Some(v) => Some(CString::new(v.as_bytes())?),
            _ => None,
        };

        let ssl_mode = match query_pairs.get("ssl_mode") {
            Some(v) => {
                let ssl_mode = match v.to_lowercase().as_str() {
                    "disabled" => mysql_ssl_mode::SSL_MODE_DISABLED,
                    "preferred" => mysql_ssl_mode::SSL_MODE_PREFERRED,
                    "required" => mysql_ssl_mode::SSL_MODE_REQUIRED,
                    "verify_ca" => mysql_ssl_mode::SSL_MODE_VERIFY_CA,
                    "verify_identity" => mysql_ssl_mode::SSL_MODE_VERIFY_IDENTITY,
                    _ => {
                        let msg = "unknown ssl_mode";
                        return Err(ConnectionError::InvalidConnectionUrl(msg.into()));
                    }
                };
                Some(ssl_mode)
            }
            _ => None,
        };

        let host = match url.host() {
            Some(Host::Ipv6(host)) => Some(CString::new(host.to_string())?),
            Some(host) if host.to_string() == "localhost" && unix_socket.is_some() => None,
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

        // this is not present in the database_url, using a default value
        let client_flags = CapabilityFlags::CLIENT_FOUND_ROWS;

        Ok(ConnectionOptions {
            host,
            user,
            password,
            database,
            port: url.port(),
            client_flags,
            ssl_mode,
            unix_socket,
            ssl_ca,
            ssl_cert,
            ssl_key,
        })
    }

    pub(super) fn host(&self) -> Option<&CStr> {
        self.host.as_deref()
    }

    pub(super) fn user(&self) -> &CStr {
        &self.user
    }

    pub(super) fn password(&self) -> Option<&CStr> {
        self.password.as_deref()
    }

    pub(super) fn database(&self) -> Option<&CStr> {
        self.database.as_deref()
    }

    pub(super) fn port(&self) -> Option<u16> {
        self.port
    }

    pub(super) fn unix_socket(&self) -> Option<&CStr> {
        self.unix_socket.as_deref()
    }

    pub(super) fn ssl_ca(&self) -> Option<&CStr> {
        self.ssl_ca.as_deref()
    }

    pub(super) fn ssl_cert(&self) -> Option<&CStr> {
        self.ssl_cert.as_deref()
    }

    pub(super) fn ssl_key(&self) -> Option<&CStr> {
        self.ssl_key.as_deref()
    }

    pub(super) fn client_flags(&self) -> CapabilityFlags {
        self.client_flags
    }

    pub(super) fn ssl_mode(&self) -> Option<mysql_ssl_mode> {
        self.ssl_mode
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

    let db_url = format!("mysql://{encoded_username}:{encoded_password}@localhost/bar",);
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
    let db_url = format!("mysql://{username}:{password}@localhost?unix_socket={unix_socket}",);
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

#[test]
fn ssl_ca_tests() {
    let ssl_ca = "/etc/ssl/certs/ca-certificates.crt";
    let username = "foo";
    let password = "bar";
    let db_url = format!("mysql://{username}:{password}@localhost?ssl_ca={ssl_ca}",);
    let conn_opts = ConnectionOptions::parse(db_url.as_str()).unwrap();
    let cstring = |s| CString::new(s).unwrap();
    assert_eq!(Some(cstring("localhost")), conn_opts.host);
    assert_eq!(None, conn_opts.port);
    assert_eq!(cstring(username), conn_opts.user);
    assert_eq!(cstring(password), conn_opts.password.unwrap());
    assert_eq!(CString::new(ssl_ca).unwrap(), conn_opts.ssl_ca.unwrap());

    let url_with_unix_str_and_ssl_ca = format!(
        "mysql://{username}:{password}@localhost?unix_socket=/var/run/mysqld.sock&ssl_ca={ssl_ca}"
    );

    let conn_opts2 = ConnectionOptions::parse(url_with_unix_str_and_ssl_ca.as_str()).unwrap();
    assert_eq!(None, conn_opts2.host);
    assert_eq!(None, conn_opts2.port);
    assert_eq!(CString::new(ssl_ca).unwrap(), conn_opts2.ssl_ca.unwrap());
}

#[test]
fn ssl_cert_tests() {
    let ssl_cert = "/etc/ssl/certs/client-cert.crt";
    let username = "foo";
    let password = "bar";
    let db_url = format!("mysql://{username}:{password}@localhost?ssl_cert={ssl_cert}");
    let conn_opts = ConnectionOptions::parse(db_url.as_str()).unwrap();
    let cstring = |s| CString::new(s).unwrap();
    assert_eq!(Some(cstring("localhost")), conn_opts.host);
    assert_eq!(None, conn_opts.port);
    assert_eq!(cstring(username), conn_opts.user);
    assert_eq!(cstring(password), conn_opts.password.unwrap());
    assert_eq!(CString::new(ssl_cert).unwrap(), conn_opts.ssl_cert.unwrap());

    let url_with_unix_str_and_ssl_cert = format!(
        "mysql://{username}:{password}@localhost?unix_socket=/var/run/mysqld.sock&ssl_cert={ssl_cert}"
    );

    let conn_opts2 = ConnectionOptions::parse(url_with_unix_str_and_ssl_cert.as_str()).unwrap();
    assert_eq!(None, conn_opts2.host);
    assert_eq!(None, conn_opts2.port);
    assert_eq!(
        CString::new(ssl_cert).unwrap(),
        conn_opts2.ssl_cert.unwrap()
    );
}

#[test]
fn ssl_key_tests() {
    let ssl_key = "/etc/ssl/certs/client-key.crt";
    let username = "foo";
    let password = "bar";
    let db_url = format!("mysql://{username}:{password}@localhost?ssl_key={ssl_key}");
    let conn_opts = ConnectionOptions::parse(db_url.as_str()).unwrap();
    let cstring = |s| CString::new(s).unwrap();
    assert_eq!(Some(cstring("localhost")), conn_opts.host);
    assert_eq!(None, conn_opts.port);
    assert_eq!(cstring(username), conn_opts.user);
    assert_eq!(cstring(password), conn_opts.password.unwrap());
    assert_eq!(CString::new(ssl_key).unwrap(), conn_opts.ssl_key.unwrap());

    let url_with_unix_str_and_ssl_key = format!(
        "mysql://{username}:{password}@localhost?unix_socket=/var/run/mysqld.sock&ssl_key={ssl_key}"
    );

    let conn_opts2 = ConnectionOptions::parse(url_with_unix_str_and_ssl_key.as_str()).unwrap();
    assert_eq!(None, conn_opts2.host);
    assert_eq!(None, conn_opts2.port);
    assert_eq!(CString::new(ssl_key).unwrap(), conn_opts2.ssl_key.unwrap());
}

#[test]
fn ssl_mode() {
    let ssl_mode = |url| ConnectionOptions::parse(url).unwrap().ssl_mode();
    assert_eq!(ssl_mode("mysql://localhost"), None);
    assert_eq!(
        ssl_mode("mysql://localhost?ssl_mode=disabled"),
        Some(mysql_ssl_mode::SSL_MODE_DISABLED)
    );
    assert_eq!(
        ssl_mode("mysql://localhost?ssl_mode=PREFERRED"),
        Some(mysql_ssl_mode::SSL_MODE_PREFERRED)
    );
    assert_eq!(
        ssl_mode("mysql://localhost?ssl_mode=required"),
        Some(mysql_ssl_mode::SSL_MODE_REQUIRED)
    );
    assert_eq!(
        ssl_mode("mysql://localhost?ssl_mode=VERIFY_CA"),
        Some(mysql_ssl_mode::SSL_MODE_VERIFY_CA)
    );
    assert_eq!(
        ssl_mode("mysql://localhost?ssl_mode=verify_identity"),
        Some(mysql_ssl_mode::SSL_MODE_VERIFY_IDENTITY)
    );
}
