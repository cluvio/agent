use crate::config::Network;
use protocol::Address;
use std::ops::Deref;

/// An address checked against some whitelist.
#[derive(Debug)]
pub struct CheckedAddr<'a>(Address<'a>);

impl<'a> CheckedAddr<'a> {
    /// Create a checked address if the given address is part of the whitelist.
    pub fn check(addr: Address<'a>, whitelist: &[Network]) -> Result<Self, Address<'a>> {
        let is_allowed = match &addr {
            Address::Addr(addr) => whitelist.iter().any(|net| {
                if let Network::Ip(net) = net {
                    net.contains(&addr.ip())
                } else {
                    false
                }
            }),
            Address::Name(addr, _) => whitelist.iter().any(|net| {
                match net {
                    Network::Ip(_)  => false,
                    Network::Dns(n) => <&str>::from(n.as_ref()) == addr,
                    Network::Pat(p) => p.matches(addr)
                }
            })
        };
        if is_allowed {
            Ok(CheckedAddr(addr))
        } else {
            Err(addr)
        }
    }

    pub fn addr(&self) -> &Address<'a> {
        &self.0
    }
}

impl<'a> Deref for CheckedAddr<'a> {
    type Target = Address<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<CheckedAddr<'a>> for Address<'a> {
    fn from(c: CheckedAddr<'a>) -> Self {
        c.0
    }
}
