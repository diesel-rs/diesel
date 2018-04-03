extern crate ipnetwork;
extern crate libc;

use self::ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use std::io::prelude::*;
use std::net::{Ipv4Addr, Ipv6Addr};

use deserialize::{self, FromSql};
use pg::Pg;
use serialize::{self, IsNull, Output, ToSql};
use sql_types::{Cidr, Inet, MacAddr};

#[cfg(windows)]
const AF_INET: u8 = 2;
// Maybe not used, but defining to follow Rust's libstd/net/sys
#[cfg(redox)]
const AF_INET: u8 = 1;
#[cfg(not(any(windows, redox)))]
const AF_INET: u8 = libc::AF_INET as u8;

const PGSQL_AF_INET: u8 = AF_INET;
const PGSQL_AF_INET6: u8 = AF_INET + 1;

#[allow(dead_code)]
mod foreign_derives {
    use super::*;

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "MacAddr"]
    struct ByteArrayProxy([u8; 6]);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Inet"]
    #[sql_type = "Cidr"]
    struct IpNetworkProxy(IpNetwork);
}

macro_rules! err {
    () => (Err("invalid network address format".into()));
    ($msg: expr) => (Err(format!("invalid network address format. {}", $msg).into()));
}

macro_rules! assert_or_error {
    ($cond: expr) => {
        if ! $cond { return err!() }
    };

    ($cond: expr, $msg: expr) => {
        if ! $cond { return err!($msg) }
    };
}

impl FromSql<MacAddr, Pg> for [u8; 6] {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        assert_or_error!(6 == bytes.len(), "input isn't 6 bytes.");
        Ok([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]])
    }
}

impl ToSql<MacAddr, Pg> for [u8; 6] {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(&self[..])
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}
macro_rules! impl_Sql {
    ($ty: ty, $net_type: expr) => {
        impl FromSql<$ty, Pg> for IpNetwork {
            fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
                // https://github.com/postgres/postgres/blob/55c3391d1e6a201b5b891781d21fe682a8c64fe6/src/include/utils/inet.h#L23-L28
                let bytes = not_none!(bytes);
                assert_or_error!(4 <= bytes.len(), "input is too short.");
                let af = bytes[0];
                let prefix = bytes[1];
                let net_type = bytes[2];
                let len = bytes[3];
                assert_or_error!(net_type == $net_type, format!("returned type isn't a {}", stringify!($ty)));
                if af == PGSQL_AF_INET {
                    assert_or_error!(bytes.len() == 8);
                    assert_or_error!(len == 4, "the data isn't the size of ipv4");
                    let b = &bytes[4..];
                    let addr = Ipv4Addr::new(b[0], b[1], b[2], b[3]);
                    let inet = Ipv4Network::new(addr, prefix)?;
                    Ok(IpNetwork::V4(inet))
                } else if af == PGSQL_AF_INET6 {
                    assert_or_error!(bytes.len() == 20);
                    assert_or_error!(len == 16, "the data isn't the size of ipv6");
                    let b = &bytes[4..];
                    let addr = Ipv6Addr::from([b[0],  b[1],  b[2],  b[3],
                                               b[4],  b[5],  b[6],  b[7],
                                               b[8],  b[9],  b[10], b[11],
                                               b[12], b[13], b[14], b[15]]);
                    let inet = Ipv6Network::new(addr, prefix)?;
                    Ok(IpNetwork::V6(inet))
                } else {
                    err!()
                }
            }
        }

        impl ToSql<$ty, Pg> for IpNetwork {
            fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
                use self::ipnetwork::IpNetwork::*;
                let net_type = $net_type;
                match self {
                    V4(net) => {
                        let mut data = [0u8;8];
                        let af = PGSQL_AF_INET;
                        let prefix = net.prefix();
                        let len: u8 = 4;
                        let addr = net.ip().octets();
                        data[0] = af; data[1] = prefix; data[2] = net_type; data[3] = len;
                        data[4..].copy_from_slice(&addr);
                        out.write_all(&data)
                            .map(|_| IsNull::No)
                            .map_err(Into::into)
                    },
                    V6(net) => {
                        let mut data = [0u8;20];
                        let af = PGSQL_AF_INET6;
                        let prefix = net.prefix();
                        let len: u8 = 16;
                        let addr = net.ip().octets();
                        data[0] = af; data[1] = prefix; data[2] = net_type; data[3] = len;
                        data[4..].copy_from_slice(&addr);
                        out.write_all(&data)
                            .map(|_| IsNull::No)
                            .map_err(Into::into)

                    },
                }
            }
        }

    }
}
impl_Sql!(Inet, 0);
impl_Sql!(Cidr, 1);

#[test]
fn macaddr_roundtrip() {
    let mut bytes = Output::test();
    let input_address = [0x52, 0x54, 0x00, 0xfb, 0xc6, 0x16];
    ToSql::<MacAddr, Pg>::to_sql(&input_address, &mut bytes).unwrap();
    let output_address: [u8; 6] = FromSql::from_sql(Some(bytes.as_ref())).unwrap();
    assert_eq!(input_address, output_address);
}

#[test]
fn v4address_to_sql() {
    macro_rules! test_to_sql {
        ($ty: ty, $net_type: expr) => {
            let mut bytes = Output::test();
            let test_address = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(127, 0, 0, 1), 32).unwrap());
            ToSql::<$ty, Pg>::to_sql(&test_address, &mut bytes).unwrap();
            assert_eq!(bytes, vec![PGSQL_AF_INET, 32, $net_type, 4, 127, 0, 0, 1]);
        }
    }

    test_to_sql!(Inet, 0);
    test_to_sql!(Cidr, 1);
}

#[test]
fn some_v4address_from_sql() {
    macro_rules! test_some_address_from_sql {
        ($ty: ty) => {
            let input_address = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(127, 0, 0, 1), 32).unwrap());
            let mut bytes = Output::test();
            ToSql::<$ty, Pg>::to_sql(&input_address, &mut bytes).unwrap();
            let output_address = FromSql::<$ty, Pg>::from_sql(Some(bytes.as_ref())).unwrap();
            assert_eq!(input_address, output_address);
        }
    }

    test_some_address_from_sql!(Cidr);
    test_some_address_from_sql!(Inet);
}

#[test]
fn v6address_to_sql() {
    macro_rules! test_to_sql {
        ($ty: ty, $net_type: expr) => {
            let mut bytes = Output::test();
            let test_address = IpNetwork::V6(Ipv6Network::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 64).unwrap());
            ToSql::<$ty, Pg>::to_sql(&test_address, &mut bytes).unwrap();
            assert_eq!(bytes, vec![PGSQL_AF_INET6, 64, $net_type, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        }
    }

    test_to_sql!(Inet, 0);
    test_to_sql!(Cidr, 1);
}

#[test]
fn some_v6address_from_sql() {
    macro_rules! test_some_address_from_sql {
        ($ty: ty) => {
            let input_address = IpNetwork::V6(Ipv6Network::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 64).unwrap());
            let mut bytes = Output::test();
            ToSql::<$ty, Pg>::to_sql(&input_address, &mut bytes).unwrap();
            let output_address = FromSql::<$ty, Pg>::from_sql(Some(bytes.as_ref())).unwrap();
            assert_eq!(input_address, output_address);
        }
    }

    test_some_address_from_sql!(Inet);
    test_some_address_from_sql!(Cidr);
}

#[test]
fn bad_address_from_sql() {
    macro_rules! bad_address_from_sql {
        ($ty: ty) => {
            let address: Result<IpNetwork, _> =
                FromSql::<$ty, Pg>::from_sql(Some(&[7, PGSQL_AF_INET, 0]));
            assert_eq!(address.unwrap_err().description(), "invalid network address format. input is too short.");
        }
    }

    bad_address_from_sql!(Inet);
    bad_address_from_sql!(Cidr);
}

#[test]
fn no_address_from_sql() {
    macro_rules! test_no_address_from_sql {
        ($ty: ty) => {
            let address: Result<IpNetwork, _> =
                FromSql::<$ty, Pg>::from_sql(None);
            assert_eq!(address.unwrap_err().description(),
                       "Unexpected null for non-null column");

        }
    }

    test_no_address_from_sql!(Inet);
    test_no_address_from_sql!(Cidr);
}
