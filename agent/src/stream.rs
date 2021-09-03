use crate::{Error, Reader, Writer};
use crate::address::CheckedAddr;
use crate::config::{Config, Network};
use either::Either;
use protocol::{Address, ErrorCode, Id, Message, Connect};
use socket2::{Socket, TcpKeepalive};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{self, TcpStream};
use tokio::io::{self, AsyncWriteExt};
use tokio::time::timeout;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};
use util::io::{send, recv};

/// Handles a single Yamux stream.
pub async fn streamer(config: Arc<Config>, stream: yamux::Stream) -> Result<(), Error> {
    let (r, w)     = futures::io::AsyncReadExt::split(stream);
    let mut reader = Reader::new(r);
    let mut writer = Writer::new(w);

    let (id, addr) = match recv(&mut reader).await? {
        Some(Message { id, data: Connect { addr }, .. }) => {
            match check_addr(addr, &config.allowed_addresses) {
                Ok(addr)  => (id, addr),
                Err(code) => {
                    send(&mut writer, Message::new(Err::<(), _>(code))).await?;
                    return Ok(())
                }
            }
        }
        None => return Err(Error::Io(io::ErrorKind::UnexpectedEof.into()))
    };

    let socket =
        match connect(id, &config, &addr).await {
            Ok(socket) => {
                log::debug!(%id, "connected to {}", addr.addr());
                socket
            }
            Err(error) => {
                log::warn!(%id, "failed to connect to {}: {}", addr.addr(), error);
                send(&mut writer, Message::new(Err::<(), _>(ErrorCode::CouldNotConnect))).await?;
                return Err(error)
            }
        };

    send(&mut writer, Message::new(Ok::<_, ErrorCode>(()))).await?;
    let mut reader = reader.into_parts().0.compat();
    let mut writer = writer.into_parts().0.compat_write();
    let (mut r, mut w) = io::split(socket);

    let (received, sent) = tokio::join! {
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

    log::debug!(%id, ?received, ?sent, "connection to {} terminated", addr.addr());

    Ok(())
}

/// Check that an address is whitelisted.
pub fn check_addr<'a>(addr: Address<'_>, whitelist: &[Network]) -> Result<CheckedAddr<'a>, ErrorCode> {
    match CheckedAddr::check(addr.into_owned(), whitelist) {
        Ok(addr)  => Ok(addr),
        Err(addr) => {
            log::error!(address = %addr, "address not allowed");
            Err(ErrorCode::AddressNotAllowed)
        }
    }
}

/// Connect to an internal address and return the open TCP socket.
pub async fn connect(re: Id, cfg: &Config, addr: &CheckedAddr<'_>) -> Result<TcpStream, Error> {
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

    log::debug!(%re, "connecting to internal address {}", addr.addr());
    let iter = resolve(addr).await?;
    let sock = timeout(cfg.connect_timeout, connect_any(iter, addr)).await??;
    let sock = Socket::from(sock.into_std()?);
    sock.set_tcp_keepalive(&KEEPALIVE_SETTINGS)?;
    Ok(TcpStream::from_std(sock.into())?)
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

