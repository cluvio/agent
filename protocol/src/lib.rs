mod agentid;

use sealed_boxes::Data;
use minicbor::{Decode, Encode};
use minicbor::bytes::ByteSlice;
use rand_core::{OsRng, TryRngCore};
use serde::Serialize;
use std::borrow::{Borrow, Cow};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

pub use agentid::AgentId;

/// A generic message.
#[derive(Debug, Decode, Encode)]
#[non_exhaustive]
pub struct Message<D> {
    /// The identifier of this message.
    #[n(0)] pub id: Id,
    /// The payload data of this message.
    #[n(1)] pub data: Option<D>
}

impl<D> Message<D> {
    pub fn new(data: D) -> Self {
        Message { id: Id::fresh(), data: Some(data) }
    }

    pub fn new_with_id(id: Id, data: D) -> Self {
        Message { id, data: Some(data) }
    }
}

/// Payload of a server control message.
#[derive(Decode, Encode)]
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

    /// Terminate the connection.
    #[n(3)] Terminate {
        #[n(0)] reason: Reason
    },

    /// Test reachability of upstream system.
    #[n(4)] Test {
        /// The upstream address.
        #[b(0)] addr: Address<'a>
    },

    /// Open a new connection and drain the existing one.
    #[n(5)] SwitchToNewConnection,

    /// A server error.
    #[n(6)] Error {
        /// Error message.
        #[b(0)] msg: Cow<'a, str>
    },

    /// The server has accepted the client.
    #[n(7)] Accepted
}

// Custom impl to skip over sensitive data.
impl fmt::Debug for Server<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Server::Ping =>
                f.debug_tuple("Ping").finish(),
            Server::Pong { re } =>
                f.debug_struct("Pong").field("re", re).finish(),
            Server::Challenge { text: _ } =>
                f.debug_struct("Challenge").finish(),
            Server::Terminate { reason } =>
                f.debug_struct("Terminate").field("reason", reason).finish(),
            Server::Test { addr } =>
                f.debug_struct("Test").field("addr", addr).finish(),
            Server::SwitchToNewConnection =>
                f.debug_struct("SwitchToNewConnection").finish(),
            Server::Error { msg } =>
                f.debug_struct("Error").field("msg", msg).finish(),
            Server::Accepted =>
                f.debug_tuple("Accepted").finish()
        }
    }
}

/// Payload of a client control message.
#[derive(Decode, Encode)]
pub enum Client<'a> {
    /// Initial client message.
    #[n(0)] Hello {
        /// The client's public key.
        #[b(0)] pubkey: Cow<'a, ByteSlice>,
        /// The version of this agent.
        #[n(1)] agent_version: Version
    },

    /// Ask the server to answer with a `Pong`.
    #[n(1)] Ping,

    /// Answer to a previously received ping message.
    #[n(2)] Pong {
        #[n(0)] re: Id
    },

    /// Answer to a previously received authentication challenge.
    ///
    /// Contains the decrypted plaintext value.
    #[n(3)] Response {
        /// The original message Id this answer corresponds to.
        #[n(0)] re: Id,
        /// The decrypted plaintext.
        #[b(1)] text: Cow<'a, ByteSlice>
    },

    /// Some error happened.
    #[cbor(n(4), map)]
    Error {
        /// The original message this error responds to.
        #[n(0)] re: Id,
        /// The optional error code.
        #[n(1)] code: Option<ErrorCode>,
        /// The optional error message.
        #[b(2)] msg: Option<Cow<'a, str>>
    },

    /// Test result.
    #[n(5)] Test {
        /// The original message this test result responds to.
        #[n(0)] re: Id,
        /// The optional error code.
        #[n(1)] code: Option<ErrorCode>
    },

    /// Opening a new connection and draining the existing one.
    #[n(6)] SwitchingConnection {
        #[n(0)] re: Id
    }
}

// Custom impl to skip over some data.
impl fmt::Debug for Client<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Client::Ping =>
                f.debug_tuple("Ping").finish(),
            Client::Pong { re } =>
                f.debug_struct("Pong").field("re", re).finish(),
            Client::Hello { agent_version, pubkey: _ } =>
                f.debug_struct("Hello").field("agent_version", agent_version).finish(),
            Client::Response { re, text: _ } =>
                f.debug_struct("Response").field("re", re).finish(),
            Client::Error { re, code, msg } =>
                f.debug_struct("Error")
                 .field("re", re)
                 .field("code", code)
                 .field("msg", msg)
                 .finish(),
            Client::Test { re, code } =>
                f.debug_struct("Test")
                 .field("re", re)
                 .field("code", code)
                 .finish(),
            Client::SwitchingConnection { re } =>
                f.debug_struct("SwitchingConnection")
                 .field("re", re)
                 .finish()
        }
    }
}

/// Establish connection to the given address and transfer data back and forth.
#[derive(Debug, Decode, Encode)]
#[cbor(map)]
pub struct Connect<'a> {
    /// The address to connect to.
    #[b(0)] pub addr: Address<'a>,
    /// The connection uses half-close (None = false).
    #[n(1)] pub use_half_close: Option<bool>
}

/// A network address.
#[derive(Debug, Clone, Decode, Encode, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Address<'a> {
    /// IP address and port number.
    #[n(0)] Addr(#[n(0)] SocketAddr),
    /// A domain name to be resolved with optional port number.
    #[n(1)] Name(#[b(0)] Cow<'a, str>, #[n(1)] u16)
}

impl<'a> Address<'a> {
    pub fn to_owned<'b>(&self) -> Address<'b> {
        match self {
            Address::Addr(a)    => Address::Addr(*a),
            Address::Name(n, p) => Address::Name(Cow::Owned(n.as_ref().to_owned()), *p)
        }
    }

    pub fn into_owned<'b>(self) -> Address<'b> {
        match self {
            Address::Addr(a)    => Address::Addr(a),
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
    /// The server challenge can not be decrypted.
    #[n(2)] DecryptionFailed
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::CouldNotConnect   => f.write_str("could not connect"),
            ErrorCode::AddressNotAllowed => f.write_str("address not allowed"),
            ErrorCode::DecryptionFailed  => f.write_str("decryption failed")
        }
    }
}

/// Possible reasons for connection termination.
#[derive(Copy, Clone, Debug, Decode, Encode, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    /// The agent failed to authenticate itself.
    ///
    /// The agent failed to prove ownership of the private key corresponding
    /// to the presented public key. Further connection attempts with the same
    /// public key are not permissible.
    #[n(0)] Unauthenticated,
    /// The agent is not authorized to connect.
    ///
    /// The agent's identity is not associated with a Cluvio organization.
    /// Further connection attempts with the same public key are not
    /// permissible.
    #[n(1)] Unauthorized,
    /// The agent version is not supported.
    ///
    /// Further connection attempts with the same version are not permissible.
    #[n(2)] UnsupportedVersion,
    /// The agent is disabled.
    ///
    /// This is usually a temporary condition and further connection attempts are warranted.
    #[n(3)] Disabled
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::Unauthenticated    => f.write_str("unauthenticated agent"),
            Reason::Unauthorized       => f.write_str("unauthorized agent"),
            Reason::UnsupportedVersion => f.write_str("unsupported agent version"),
            Reason::Disabled           => f.write_str("agent disabled")
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
        Id(OsRng.try_next_u64().expect("OS RNG not available or misconfigured"))
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
    #[n(0)] pub major: u64,
    #[n(1)] pub minor: u64,
    #[n(2)] pub patch: u64
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
