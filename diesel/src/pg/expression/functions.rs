//! PostgreSQL specific functions

use sql_types::*;
use super::expression_methods::InetOrCidr;

sql_function! {
    /// Creates an abbreviated display format as text.
    fn abbrev<T: InetOrCidr>(addr: T) -> Text;
}
sql_function! {
    /// Computes the broadcast address for the address's network.
    fn broadcast<T: InetOrCidr>(addr: T) -> Inet;
}
sql_function! {
    /// Returns the address's family: 4 for IPv4, 6 for IPv6.
    fn family<T: InetOrCidr>(addr: T) -> Integer;
}
sql_function! {
    /// Returns the IP address as text, ignoring the netmask.
    fn host<T: InetOrCidr>(addr: T) -> Text;
}
sql_function! {
    /// Computes the host mask for the address's network.
    fn hostmask<T: InetOrCidr>(addr: T) -> Inet;
}
sql_function! {
    /// Computes the smallest network that includes both of the given networks.
    fn inet_merge<T: InetOrCidr, U: InetOrCidr>(a: T, b: U) -> Cidr;
}
sql_function! {
    /// Tests whether the addresses belong to the same IP family.
    fn inet_same_family<T: InetOrCidr, U: InetOrCidr>(a: T, b: U) -> Bool;
}
sql_function! {
    /// Returns the netmask length in bits.
    fn masklen<T: InetOrCidr>(addr: T) -> Integer;
}
sql_function! {
    /// Computes the network mask for the address's network.
    fn netmask<T: InetOrCidr>(addr: T) -> Inet;
}
sql_function! {
    /// Returns the network part of the address, zeroing out whatever is to the right of the
    /// netmask. (This is equivalent to casting the value to cidr.)
    fn network(addr: Inet) -> Cidr;
}
sql_function! {
    /// Sets the netmask length for an inet or cidr value.
    /// For inet, the address part does not changes. For cidr, address bits to the right of the new
    /// netmask are set to zero.
    fn set_masklen<T: InetOrCidr>(addr: T, len: Integer) -> T;
}
