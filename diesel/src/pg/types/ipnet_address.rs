extern crate libc;

use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use std::io::prelude::*;
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::deserialize::{self, FromSql, FromSqlRow};
use crate::pg::{Pg, PgValue};
#[cfg(test)]
use crate::query_builder::bind_collector::ByteWrapper;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{Cidr, Inet};

#[cfg(windows)]
const AF_INET: u8 = 2;
// Maybe not used, but defining to follow Rust's libstd/net/sys
#[cfg(target_os = "redox")]
const AF_INET: u8 = 1;
#[cfg(not(any(windows, target_os = "redox")))]
const AF_INET: u8 = libc::AF_INET as u8;

const PGSQL_AF_INET: u8 = AF_INET;
const PGSQL_AF_INET6: u8 = AF_INET + 1;

#[allow(dead_code)]
mod foreign_derives {
    use super::*;
    use crate::expression::AsExpression;

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Inet)]
    #[diesel(sql_type = Cidr)]
    struct IpNetworkProxy(IpNet);
}

macro_rules! err {
    () => {
        Err("invalid network address format".into())
    };
    ($msg:expr) => {
        Err(format!("invalid network address format. {}", $msg).into())
    };
}

macro_rules! assert_or_error {
    ($cond:expr) => {
        if !$cond {
            return err!();
        }
    };

    ($cond:expr, $msg:expr) => {
        if !$cond {
            return err!($msg);
        }
    };
}

macro_rules! impl_Sql {
    ($ty: ty, $net_type: expr) => {
        #[cfg(all(feature = "postgres_backend", feature = "ipnet-address"))]
        impl FromSql<$ty, Pg> for IpNet {
            fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
                // https://github.com/postgres/postgres/blob/55c3391d1e6a201b5b891781d21fe682a8c64fe6/src/include/utils/inet.h#L23-L28
                let bytes = value.as_bytes();
                assert_or_error!(4 <= bytes.len(), "input is too short.");
                let af = bytes[0];
                let prefix = bytes[1];
                let net_type = bytes[2];
                let len = bytes[3];
                assert_or_error!(
                    net_type == $net_type,
                    format!("returned type isn't a {}", stringify!($ty))
                );
                if af == PGSQL_AF_INET {
                    assert_or_error!(bytes.len() == 8);
                    assert_or_error!(len == 4, "the data isn't the size of ipv4");
                    let b = &bytes[4..];
                    let addr = Ipv4Addr::new(b[0], b[1], b[2], b[3]);
                    let inet = Ipv4Net::new(addr, prefix)?;
                    Ok(IpNet::V4(inet))
                } else if af == PGSQL_AF_INET6 {
                    assert_or_error!(bytes.len() == 20);
                    assert_or_error!(len == 16, "the data isn't the size of ipv6");
                    let b = &bytes[4..];
                    let addr = Ipv6Addr::from([
                        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11],
                        b[12], b[13], b[14], b[15],
                    ]);
                    let inet = Ipv6Net::new(addr, prefix)?;
                    Ok(IpNet::V6(inet))
                } else {
                    err!()
                }
            }
        }

        #[cfg(all(feature = "postgres_backend", feature = "ipnet-address"))]
        impl ToSql<$ty, Pg> for IpNet {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                let net_type = $net_type;
                match *self {
                    IpNet::V4(ref net) => {
                        let mut data = [0u8; 8];
                        let af = PGSQL_AF_INET;
                        let prefix = net.prefix_len();
                        let len: u8 = 4;
                        let addr = net.network().octets();
                        data[0] = af;
                        data[1] = prefix;
                        data[2] = net_type;
                        data[3] = len;
                        data[4..].copy_from_slice(&addr);
                        out.write_all(&data).map(|_| IsNull::No).map_err(Into::into)
                    }
                    IpNet::V6(ref net) => {
                        let mut data = [0u8; 20];
                        let af = PGSQL_AF_INET6;
                        let prefix = net.prefix_len();
                        let len: u8 = 16;
                        let addr = net.network().octets();
                        data[0] = af;
                        data[1] = prefix;
                        data[2] = net_type;
                        data[3] = len;
                        data[4..].copy_from_slice(&addr);
                        out.write_all(&data).map(|_| IsNull::No).map_err(Into::into)
                    }
                }
            }
        }
    };
}

impl_Sql!(Inet, 0);
impl_Sql!(Cidr, 1);

#[test]
fn v4address_to_sql() {
    macro_rules! test_to_sql {
        ($ty:ty, $net_type:expr) => {
            let mut buffer = Vec::new();
            {
                let mut bytes = Output::test(ByteWrapper(&mut buffer));
                let test_address =
                    IpNet::V4(Ipv4Net::new(Ipv4Addr::new(127, 0, 0, 1), 32).unwrap());
                ToSql::<$ty, Pg>::to_sql(&test_address, &mut bytes).unwrap();
            }
            assert_eq!(buffer, vec![PGSQL_AF_INET, 32, $net_type, 4, 127, 0, 0, 1]);
        };
    }

    test_to_sql!(Inet, 0);
    test_to_sql!(Cidr, 1);
}

#[test]
fn some_v4address_from_sql() {
    macro_rules! test_some_address_from_sql {
        ($ty:tt) => {
            let input_address = IpNet::V4(Ipv4Net::new(Ipv4Addr::new(127, 0, 0, 1), 32).unwrap());
            let mut buffer = Vec::new();
            {
                let mut bytes = Output::test(ByteWrapper(&mut buffer));
                ToSql::<$ty, Pg>::to_sql(&input_address, &mut bytes).unwrap();
            }
            let output_address = FromSql::<$ty, Pg>::from_sql(PgValue::for_test(&buffer)).unwrap();
            assert_eq!(input_address, output_address);
        };
    }

    test_some_address_from_sql!(Cidr);
    test_some_address_from_sql!(Inet);
}

#[test]
fn v6address_to_sql() {
    macro_rules! test_to_sql {
        ($ty:ty, $net_type:expr) => {
            let mut buffer = Vec::new();
            {
                let mut bytes = Output::test(ByteWrapper(&mut buffer));
                let test_address =
                    IpNet::V6(Ipv6Net::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 64).unwrap());
                ToSql::<$ty, Pg>::to_sql(&test_address, &mut bytes).unwrap();
            }
            assert_eq!(
                buffer,
                vec![
                    PGSQL_AF_INET6,
                    64,
                    $net_type,
                    16,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    1,
                ]
            );
        };
    }

    test_to_sql!(Inet, 0);
    test_to_sql!(Cidr, 1);
}

#[test]
fn some_v6address_from_sql() {
    macro_rules! test_some_address_from_sql {
        ($ty:tt) => {
            let input_address =
                IpNet::V6(Ipv6Net::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 64).unwrap());
            let mut buffer = Vec::new();
            {
                let mut bytes = Output::test(ByteWrapper(&mut buffer));
                ToSql::<$ty, Pg>::to_sql(&input_address, &mut bytes).unwrap();
            }
            let output_address = FromSql::<$ty, Pg>::from_sql(PgValue::for_test(&buffer)).unwrap();
            assert_eq!(input_address, output_address);
        };
    }

    test_some_address_from_sql!(Inet);
    test_some_address_from_sql!(Cidr);
}

#[test]
fn bad_address_from_sql() {
    macro_rules! bad_address_from_sql {
        ($ty:tt) => {
            let address: Result<IpNet, _> =
                FromSql::<$ty, Pg>::from_sql(PgValue::for_test(&[7, PGSQL_AF_INET, 0]));
            assert_eq!(
                address.unwrap_err().to_string(),
                "invalid network address format. input is too short."
            );
        };
    }

    bad_address_from_sql!(Inet);
    bad_address_from_sql!(Cidr);
}

#[test]
fn no_address_from_sql() {
    macro_rules! test_no_address_from_sql {
        ($ty:ty) => {
            let address: Result<IpNet, _> = FromSql::<$ty, Pg>::from_nullable_sql(None);
            assert_eq!(
                address.unwrap_err().to_string(),
                "Unexpected null for non-null column"
            );
        };
    }

    test_no_address_from_sql!(Inet);
    test_no_address_from_sql!(Cidr);
}
