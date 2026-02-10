#![no_main]

use libfuzzer_sys::fuzz_target;

use core::num::NonZeroU32;
use diesel::deserialize::FromSql;
use diesel::pg::{Pg, PgValue};
use diesel::sql_types::{Cidr, Inet};
use ipnetwork::IpNetwork;

// Fuzz diesel's PostgreSQL network address (inet/cidr) wire protocol parser.
//
// IpNetwork::from_sql parses the PG binary inet/cidr format:
//   [af: u8] [prefix: u8] [net_type: u8] [len: u8]
//   IPv4: 4 address bytes (total 8 bytes)
//   IPv6: 16 address bytes (total 20 bytes)
//
// Targets: diesel/src/pg/types/network_address.rs (FromSql<Inet/Cidr, Pg>)

fuzz_target!(|data: &[u8]| {
    // Test INET type
    let oid = NonZeroU32::new(869).unwrap(); // INETOID
    let value = PgValue::new(data, &oid);
    let _ = <IpNetwork as FromSql<Inet, Pg>>::from_sql(value);

    // Test CIDR type
    let oid = NonZeroU32::new(650).unwrap(); // CIDROID
    let value = PgValue::new(data, &oid);
    let _ = <IpNetwork as FromSql<Cidr, Pg>>::from_sql(value);
});
