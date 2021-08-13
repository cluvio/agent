mod agentid;

use sealed_boxes::Data;
use minicbor::{Decode, Encode};
use minicbor::bytes::ByteSlice;
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use std::borrow::{Borrow, Cow};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use util::crypto;
use util::time::UnixTime;

pub use agentid::AgentId;

/// A generic message.
#[derive(Debug, Decode, Encode)]
#[non_exhaustive]
pub struct Message<D> {
    /// The identifier of this message.
    #[n(0)] pub id: Id,
    /// The payload data of this message.
    #[n(1)] pub data: D
}

impl<D> Message<D> {
    pub fn new(data: D) -> Self {
        Message { id: Id::fresh(), data }
    }

    pub fn new_with_id(id: Id, data: D) -> Self {
        Message { id, data }
    }
}


/// Payload of a server message.
#[derive(Debug, Decode, Encode)]
pub enum Server<'a> {
    /// Ask the client to answer with a `Pong`.
    #[n(0)] Ping,

    /// Answer a previously received ping message.
    #[n(1)] Pong {
        #[n(0)] re: Id
    },

    /// Tell the client to decrypt the given ciphertext.
    ///
    /// When clients authenticate, we send them a decrypt request
    /// so they prove to us that they posses the private key that
    /// corresponds to the public key that was used for encryption.
    #[n(2)] Challenge {
        #[n(0)] text: Box<CipherText>
    },

    /// Server answer to a newly opened data connection.
    ///
    /// Informs the client about the address the node will send
    /// traffic from.
    ///
    /// Control connections can not expect this message.
    #[n(3)] DataAddress {
        /// The message Id this answer corresponds to.
        #[n(0)] re: Id,
        /// Some opaque value to report back to the gateway.
        #[b(1)] data: Opaque<'a>
    },

    /// Tell the client to open a new data connection between `ext` and `int`.
    #[n(4)] Bridge {
        /// The external address.
        #[b(0)] ext: Address<'a>,
        /// The internal address.
        #[b(1)] int: Address<'a>,
        /// Authorization token.
        #[b(2)] auth: Authorization<'a>
    },

    /// Connect to the provided address and report back the result.
    #[n(5)] TestConnect {
        /// The internal address to test.
        #[b(0)] int: Address<'a>
    },

    /// Terminate the connection.
    #[n(6)] Terminate {
        #[n(0)] reason: Reason
    }
}

/// Payload of a client message.
#[derive(Debug, Decode, Encode)]
pub enum Client<'a> {
    /// Initial client message.
    #[n(0)] Hello {
        /// The client's public key.
        #[b(0)] pubkey: Cow<'a, ByteSlice>,
        /// What kind of connection this is.
        #[n(1)] connection: ConnectionType<'a>,
        /// The version of this agent.
        #[n(2)] agent_version: Version
    },

    /// Ask the server to answer with a `Pong`.
    #[n(1)] Ping,

    /// Answer to a previously received ping message.
    #[n(2)] Pong {
        #[n(0)] re: Id
    },

    /// Answer to a previously received decrypt message.
    ///
    /// Contains the decrypted plaintext value.
    #[n(3)] Response {
        /// The original message Id this answer corresponds to.
        #[n(0)] re: Id,
        /// The decrypted plaintext.
        #[b(1)] text: Cow<'a, ByteSlice>
    },

    /// Answer to a previously received `Server::Bridge` message.
    ///
    /// After the client successfully reached the requested
    /// external system, it informs the control server about
    /// the external peer address it is awaiting traffic on to
    /// relay to its internal endpoint.
    #[n(4)] Established  {
        /// The original message Id this answer corresponds to.
        #[n(0)] re: Id,
        /// Some opaque value to report back to the gateway.
        #[b(1)] data: Opaque<'a>
    },

    /// Some error happened.
    #[n(5)]
    #[cbor(map)]
    Error {
        /// The original message this error responds to.
        #[n(0)] re: Id,
        /// The optional error code.
        #[n(1)] code: Option<ErrorCode>,
        /// The optional error message.
        #[b(2)] msg: Option<Cow<'a, str>>
    },

    /// Client has now free capacity for more data connections.
    #[n(6)] Available,

    /// The result of a successful `TestConnect` command.
    #[n(7)] TestConnectSuccess {
        #[n(0)] re: Id
    }
}

/// The kind of connection the client has opened.
#[derive(Debug, Decode, Encode)]
pub enum ConnectionType<'a> {
    /// A control connection for receiving commands.
    #[n(0)] Control,
    /// A data connection for relaying data.
    #[n(1)] Data {
        #[n(0)] re: Id,
        #[b(1)] auth: Authorization<'a>
    }
}

/// An authorization token for opening new data connections.
#[derive(Debug, Decode, Encode)]
pub struct Authorization<'a> {
    /// Encoded [`AuthorizationToken`].
    #[b(0)] pub token: Cow<'a, ByteSlice>,
    /// Signature of authorization token.
    #[b(1)] pub sign: Cow<'a, ByteSlice>
}

/// The authorization token.
#[derive(Debug, Decode, Encode)]
pub struct AuthorizationToken<'a> {
    /// Signing key identifier.
    #[n(0)] pub key_id: u64,
    /// Validity timeout.
    #[n(1)] pub until: UnixTime,
    /// The opaque authorization bytes.
    #[b(2)] pub value: Cow<'a, ByteSlice>
}

impl<'a> Authorization<'a> {
    pub fn into_owned<'b>(self) -> Authorization<'b> {
        Authorization {
            token: Cow::Owned(self.token.into_owned()),
            sign: Cow::Owned(self.sign.into_owned())
        }
    }
}

/// A network address.
#[derive(Debug, Decode, Encode, PartialEq, Eq)]
pub enum Address<'a> {
    /// IP address and port number.
    #[n(0)] Addr(#[n(0)] SocketAddr),
    /// A domain name to be resolved with optional port number.
    #[n(1)] Name(#[b(0)] Cow<'a, str>, #[n(1)] u16)
}

impl<'a> Address<'a> {
    pub fn to_owned<'b>(&self) -> Address<'b> {
        match self {
            Address::Addr(a) => Address::Addr(*a),
            Address::Name(n, p) => Address::Name(Cow::Owned(n.as_ref().to_owned()), *p)
        }
    }

    pub fn into_owned<'b>(self) -> Address<'b> {
        match self {
            Address::Addr(a) => Address::Addr(a),
            Address::Name(n, p) => Address::Name(Cow::Owned(n.into_owned()), p)
        }
    }

    pub fn borrow(&self) -> Address<'_> {
        match self {
            Address::Addr(a)    => Address::Addr(*a),
            Address::Name(n, p) => Address::Name(Cow::Borrowed(n.borrow()), *p)
        }
    }

    pub fn read_owned<'b>(addr: String, port: u16) -> Address<'b> {
        if let Ok(ip) = IpAddr::from_str(&addr) {
            Address::Addr(SocketAddr::from((ip, port)))
        } else {
            Address::Name(Cow::Owned(addr), port)
        }
    }

    pub fn read_borrowed(addr: &'a str, port: u16) -> Address<'a> {
        if let Ok(ip) = IpAddr::from_str(addr) {
            Address::Addr(SocketAddr::from((ip, port)))
        } else {
            Address::Name(Cow::Borrowed(addr), port)
        }
    }
}

impl fmt::Display for Address<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Addr(a)    => a.fmt(f),
            Address::Name(n, p) => write!(f, "{}:{}", n, p)
        }
    }
}

#[derive(Debug, Decode, Encode)]
pub struct Opaque<'a> {
    /// The encryption key identifier.
    #[n(0)] pub key_id: u64,
    /// A message nonce.
    #[n(1)] pub nonce: crypto::Nonce,
    /// The encrypted payload.
    #[b(2)] pub value: Cow<'a, ByteSlice>
}

impl<'a> Opaque<'a> {
    pub fn into_owned<'b>(self) -> Opaque<'b> {
        Opaque {
            key_id: self.key_id,
            nonce: self.nonce,
            value: Cow::Owned(self.value.into_owned())
        }
    }
}

/// The challenge-response ciphertext used when authenticating clients.
#[derive(Debug, Clone, Decode, Encode)]
#[cbor(transparent)]
pub struct CipherText(#[n(0)] pub Data<32>);

impl From<Data<32>> for CipherText {
    fn from(d: Data<32>) -> Self {
        CipherText(d)
    }
}

/// Possible error codes.
#[derive(Copy, Clone, Debug, Decode, Encode, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cbor(index_only)]
pub enum ErrorCode {
    /// An address was not reachable.
    #[n(0)] CouldNotConnect,
    /// The requested address is blocked by the client configuration.
    #[n(1)] AddressNotAllowed,
    /// Max. number of connections are in use.
    #[n(2)] AtCapacity,
    /// The server challenge can not be decrypted.
    #[n(3)] DecryptionFailed
}

/// Possible reasons for termination.
#[derive(Copy, Clone, Debug, Decode, Encode, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cbor(index_only)]
pub enum Reason {
    /// The agent is not authentic.
    ///
    /// I.e. it could not prove ownership of the claimed private key.
    #[n(0)] Unauthenticated,
    /// The agent is not authorized to connect.
    #[n(1)] Unauthorized,
    /// The agent version is not supported.
    #[n(2)] UnsupportedVersion
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::Unauthenticated    => f.write_str("unauthenticated agent"),
            Reason::Unauthorized       => f.write_str("unauthorized agent"),
            Reason::UnsupportedVersion => f.write_str("unsupported agent version")
        }
    }
}

/// A generic identifier.
#[derive(Copy, Clone, Decode, Encode, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cbor(transparent)]
pub struct Id(#[n(0)] u64);

impl Id {
    /// Get a random Id.
    pub fn fresh() -> Self {
        Id(OsRng.next_u64())
    }

    /// Get the numeric value of this ID.
    pub fn numeric(self) -> u64 {
        self.0
    }
}

impl From<u64> for Id {
    fn from(n: u64) -> Self {
        Id(n)
    }
}

impl nohash_hasher::IsEnabled for Id {}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Version information.
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    #[n(0)] major: u64,
    #[n(1)] minor: u64,
    #[n(2)] patch: u64
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Version { major, minor, patch }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
