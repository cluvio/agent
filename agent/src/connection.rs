use crate::{Error, tls, Reader, Writer};
use crate::address::CheckedAddr;
use crate::config::Config;
use either::Either;
use protocol::{Address, Authorization, Client, ConnectionType, Id, Message, Opaque, Server, Version};
use sealed_boxes::decrypt;
use socket2::{Socket, TcpKeepalive};
use std::borrow::Cow;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{self, TcpStream};
use tokio::io::{self, AsyncWriteExt};
use tokio::time::timeout;
use tokio_rustls::webpki::DNSNameRef;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use util::io::{send, recv};

// TCP keepalive settings used for data transfer connections.
#[cfg(unix)]
const KEEPALIVE_SETTINGS: TcpKeepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(30))
        .with_interval(Duration::from_secs(10))
        .with_retries(3);

// TCP keepalive settings used for data transfer connections.
#[cfg(windows)]
const KEEPALIVE_SETTINGS: TcpKeepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(30))
        .with_interval(Duration::from_secs(10));

/// An outbound data connection.
#[derive(Debug)]
pub struct Connection {
    /// The peer's socket address.
    pub(crate) peer: SocketAddr,
    /// The read-half of this connection.
    pub(crate) reader: io::ReadHalf<tls::Stream<TcpStream>>,
    /// The write-half of this connection.
    pub(crate) writer: io::WriteHalf<tls::Stream<TcpStream>>,
    /// Some data provided by the external node to relay back to the control node.
    pub(crate) witness: Opaque<'static>
}

/// Connect to the given external address for eventual data transfer.
pub async fn establish(re: Id, cfg: &Config, ver: &Version, client: &tls::Client, ext: &CheckedAddr<'_>, auth: Authorization<'_>) -> Result<Connection, Error> {
    let stream = {
        log::debug!(%re, "connecting to external address {}", ext.addr());
        let name = match ext.addr() {
            Address::Name(h, _) => DNSNameRef::try_from_ascii_str(h)?,
            Address::Addr(_)    => return Err(Error::Unexpected("socket address", "domain name"))
        };
        let iter = resolve(ext).await?;
        client.connect_any(iter, name).await?
    };

    let peer   = stream.get_ref().0.peer_addr()?;
    let (r, w) = io::split(stream);
    let mut r  = Reader::new(r.compat());
    let mut w  = Writer::new(w.compat_write());

    // say hello ...
    let pubkey = cfg.private_key.public_key();
    let hello  = Client::Hello {
        pubkey: Cow::Borrowed(pubkey.as_bytes()[..].into()),
        connection: ConnectionType::Data { re, auth },
        agent_version: *ver
    };
    send(&mut w, Message::new(hello)).await?;

    // await authentication challenge ...
    match recv(&mut r).await? {
        Some(Message { id, data, .. }) => match data {
            Server::Challenge { text } => {
                log::trace!(msg = %id, "received challenge");
                let plain = decrypt(&cfg.private_key, text.0)?;
                let data  = Client::Response { re: id, text: Cow::Borrowed(plain.as_ref().into()) };
                send(&mut w, Message::new(data)).await?;
                w.flush().await?
            }
            Server::Terminate { reason } => {
                log::warn!(msg = %id, ?reason, "connection rejected");
                return Err(Error::Unexpected("terminate message", "challenge"))
            }
            other => {
                log::debug!(msg = %id, "unexpected server message: {:?}", other);
                return Err(Error::Unexpected("server message", "challenge"))
            }
        }
        None => return Err(Error::Io(io::ErrorKind::UnexpectedEof.into()))
    }

    // await server answer telling us the address to use ...
    match recv(&mut r).await? {
        Some(Message { id, data, .. }) => match data {
            Server::DataAddress { re: re2, data } => {
                if re != re2 {
                    let err = Error::Mismatch { expected: re2, actual: re };
                    log::debug!(msg = %id, ours = %re, theirs = %re2, "unexpected server data address: {}", err);
                    return Err(err)
                }
                let witness = data.into_owned();
                Ok(Connection {
                    peer,
                    reader: r.into_parts().0.into_inner(),
                    writer: w.into_parts().0.into_inner(),
                    witness
                })
            }
            Server::Terminate { reason } => {
                log::warn!(msg = %id, ?reason, "connection rejected");
                Err(Error::Unexpected("terminate message", "data address"))
            }
            other => {
                log::debug!(msg = %id, "unexpected server message: {:?}", other);
                Err(Error::Unexpected("server message", "data address"))
            }
        }
        None => Err(Error::Io(io::ErrorKind::UnexpectedEof.into()))
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub(crate) from: SocketAddr,
    pub(crate) to: SocketAddr,
    pub(crate) ext_to_int: io::Result<u64>,
    pub(crate) int_to_ext: io::Result<u64>
}

/// Connect to an internal address and return the open TCP socket.
pub async fn connect(re: Id, cfg: &Config, int: &CheckedAddr<'_>) -> Result<TcpStream, Error> {
    log::debug!(%re, "connecting to internal address {}", int.addr());
    let iter = resolve(int).await?;
    let sock = timeout(cfg.connect_timeout, connect_any(iter, int)).await??;
    let sock = Socket::from(sock.into_std()?);
    sock.set_tcp_keepalive(&KEEPALIVE_SETTINGS)?;
    Ok(TcpStream::from_std(sock.into())?)
}

/// Connect to an internal address and relay data to the given connection.
pub async fn bridge(re: Id, cfg: &Config, conn: Connection, int: &CheckedAddr<'_>) -> Result<Outcome, Error> {
    let tcp  = connect(re, cfg, int).await?;
    let peer = tcp.peer_addr()?;
    log::debug!(%re, "connected to internal host {}: {}", int.addr(), peer);

    let (mut r, mut w) = io::split(tcp);
    let mut reader = conn.reader;
    let mut writer = conn.writer;

    let result = tokio::join! {
        async move {
            let result = io::copy(&mut reader, &mut w).await;
            w.shutdown().await?;
            result
        },
        async move {
            let result = io::copy(&mut r, &mut writer).await;
            writer.shutdown().await?;
            result
        }
    };

    log::debug!(%re, "bridge to {} at {} terminated", int.addr(), peer);

    Ok(Outcome {
        from: conn.peer,
        to: peer,
        ext_to_int: result.0,
        int_to_ext: result.1
    })
}

/// Resolve an address.
async fn resolve<'a>(addr: &'a CheckedAddr<'_>) -> Result<impl Iterator<Item = SocketAddr> + 'a, Error> {
    match addr.addr() {
        Address::Addr(socketaddr) => Ok(Either::Left(std::iter::once(*socketaddr))),
        Address::Name(host, port) => {
            let mut iter = net::lookup_host((host.as_ref(), *port)).await?.peekable();
            if iter.peek().is_none() {
                return Err(Error::Unreachable(host.as_ref().into()))
            }
            Ok(Either::Right(iter))
        }
    }
}

/// Connect to any of the given IP addresses.
async fn connect_any<I>(iter: I, dest: &Address<'_>) -> io::Result<TcpStream>
where
    I: Iterator<Item = SocketAddr>
{
    for addr in iter {
        match TcpStream::connect(addr).await {
            Ok(s)  => return Ok(s),
            Err(e) => log::debug!("failed to connect to {} ({}): {}", addr, dest, e)
        }
    }

    let msg = format!("could not connect to any address of {}", dest);
    Err(io::Error::new(io::ErrorKind::AddrNotAvailable, msg))
}

