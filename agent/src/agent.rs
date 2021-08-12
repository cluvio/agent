use crate::{Reader, Writer};
use crate::address::CheckedAddr;
use crate::config::{Config, Network};
use crate::connection::{self, Connection, Outcome};
use crate::error::Error;
use crate::tls;
use futures::future;
use futures::stream::{FuturesUnordered, StreamExt};
use humantime::format_duration;
use minicbor_io::Error as CborError;
use protocol::{Address, AgentId, Client, ConnectionType, ErrorCode, Id, Message, Opaque, Server};
use protocol::{Reason, Version};
use sealed_boxes::decrypt;
use std::borrow::Cow;
use std::convert::identity;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io;
use tokio::net;
use tokio::{select, spawn};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use util::io::{send, recv};

/// The connection agent.
///
/// It receives commands from the control server and attempts
/// to bridge external systems with internal ones.
#[derive(Debug)]
pub struct Agent {
    version: Version,
    id: AgentId,
    config: Arc<Config>,
    client: tls::Client,
    failures: u32,
    ping_state: PingState,
    connected_timestamp: Option<Instant>,
    connect_tasks: FuturesUnordered<JoinHandle<ConnectResult>>,
    test_tasks: FuturesUnordered<JoinHandle<TestResult>>,
    transfer_tasks: FuturesUnordered<JoinHandle<TransferResult>>,
    notify_cap: bool
}

/// The result of a connection attempt.
#[derive(Debug)]
struct ConnectResult {
    /// Request ID of this connection attempt.
    re: Id,
    /// The internal address.
    addr: CheckedAddr<'static>,
    /// The result of this connection attempt.
    conn: Result<Connection, Error>
}

#[derive(Debug)]
struct TransferResult {
    /// Request ID of this connection attempt.
    re: Id,
    /// The internal address.
    addr: CheckedAddr<'static>,
    /// The result of this transfer.
    result: Result<Outcome, Error>
}

/// The result of a connection test attempt.
#[derive(Debug)]
struct TestResult {
    /// Request ID of this connection test attempt.
    re: Id,
    /// The test result.
    result: Result<(), Error>
}

/// Ping/Pong state.
#[derive(Debug)]
enum PingState {
    /// Normal processing.
    Idle,
    /// Awaiting pong with the given Id.
    Awaiting(Id)
}

impl Agent {
    pub fn new(cfg: Config) -> Result<Self, Error> {
        let client = tls::Client::new(&cfg)?;
        Ok(Agent {
            version: crate::version()?,
            id: AgentId::from(cfg.private_key.public_key()),
            config: Arc::new(cfg),
            client,
            failures: 0,
            ping_state: PingState::Idle,
            connected_timestamp: None,
            transfer_tasks: futures_unordered(),
            connect_tasks: futures_unordered(),
            test_tasks: futures_unordered(),
            notify_cap: false
        })
    }

    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// Run this agent.
    ///
    /// This method will only return if the gateway terminates the agent with
    /// a reason (which is returned to the caller).
    pub async fn go(mut self) -> Reason {
        let (mut reader, mut writer) = self.connect().await;

        log::info!(agent = %self.id, "up and running");

        // Event processing.
        loop {
            log::trace!("awaiting event ...");
            select! {
                // A new message from our control server.
                message = recv(&mut reader) => match message {
                    Err(e) => {
                        log::debug!("error reading from control server: {}", e);
                        let (r, w) = self.reconnect(reader, writer).await;
                        reader = r;
                        writer = w
                    }
                    Ok(None) => {
                        log::debug!("connection to control server lost, reconnecting ...");
                        let (r, w) = self.reconnect(reader, writer).await;
                        reader = r;
                        writer = w
                    }
                    Ok(Some(m)) => match self.on_message(&mut writer, m).await {
                        Err(Error::Terminated(reason)) => return reason,
                        Err(e) => {
                            log::debug!("failed to answer control server message: {}", e);
                            let (r, w) = self.reconnect(reader, writer).await;
                            reader = r;
                            writer = w
                        }
                        Ok(()) => {}
                    }
                },

                // A connection attempt completed.
                Some(result) = self.connect_tasks.next() => {
                    match result {
                        Ok(res) =>
                            if let Err(e) = self.on_established(&mut writer, res).await {
                                log::debug!("failed to write control server message: {}", e);
                                let (r, w) = self.reconnect(reader, writer).await;
                                reader = r;
                                writer = w
                            }
                        Err(er) =>
                            if er.is_panic() {
                                log::error!("connect task panic: {}", er)
                            } else {
                                log::debug!("connect task error: {}", er)
                            }
                    }
                    if let Err(e) = self.notify_capacity(&mut writer).await {
                        log::debug!("error sending message to control server: {}", e);
                        let (r, w) = self.reconnect(reader, writer).await;
                        reader = r;
                        writer = w
                    }
                },

                // A connection test attempt completed.
                Some(result) = self.test_tasks.next() => {
                    match result {
                        Ok(res) => if let Err(e) = self.on_connect_test(&mut writer, res).await {
                            log::debug!("failed to write control server message: {}", e);
                            let (r, w) = self.reconnect(reader, writer).await;
                            reader = r;
                            writer = w
                        }
                        Err(er) =>
                            if er.is_panic() {
                                log::error!("connect test task panic: {}", er)
                            } else {
                                log::debug!("connect test task error: {}", er)
                            }
                    }
                    if let Err(e) = self.notify_capacity(&mut writer).await {
                        log::debug!("error sending message to control server: {}", e);
                        let (r, w) = self.reconnect(reader, writer).await;
                        reader = r;
                        writer = w
                    }
                },

                // A data transfer completed.
                Some(result) = self.transfer_tasks.next() => {
                    match result {
                        Ok(out) => self.on_finished(out),
                        Err(er) =>
                            if er.is_panic() {
                                log::error!("transfer task panic: {}", er)
                            } else {
                                log::debug!("transfer task error: {}", er)
                            }
                    }
                    if let Err(e) = self.notify_capacity(&mut writer).await {
                        log::debug!("error sending message to control server: {}", e);
                        let (r, w) = self.reconnect(reader, writer).await;
                        reader = r;
                        writer = w
                    }
                }

                // Awaiting pong or time to send the next ping.
                () = sleep(self.config.ping_frequency) => match self.ping_state {
                    PingState::Idle => {
                        let msg = Message::new(Client::Ping);
                        if let Err(e) = send(&mut writer, &msg).await {
                            log::debug!("error sending message to control server: {}", e);
                            let (r, w) = self.reconnect(reader, writer).await;
                            reader = r;
                            writer = w
                        } else {
                            self.ping_state = PingState::Awaiting(msg.id)
                        }
                    }
                    PingState::Awaiting(id) => {
                        log::info!(msg = %id, "no pong from control server");
                        let (r, w) = self.reconnect(reader, writer).await;
                        reader = r;
                        writer = w
                    }
                }
            }
        }
    }

    /// Handle message from control server.
    async fn on_message(&mut self, writer: &mut Writer, msg: Message<Server<'_>>) -> Result<(), Error> {
        log::trace!(msg = %msg.id, "received message data: {:?}", msg.data);

        match msg.data {
            Server::Ping => {
                send(writer, Message::new(Client::Pong { re: msg.id })).await?;
            }
            Server::Pong { re } => {
                if let PingState::Awaiting(p) = self.ping_state {
                    if re == p {
                        self.ping_state = PingState::Idle
                    }
                }
            }
            Server::Challenge { text } =>
                match decrypt(&self.config.private_key, text.0) {
                    Ok(plain) => {
                        let data = Client::Response {
                            re: msg.id,
                            text: Cow::Borrowed(plain.as_ref().into())
                        };
                        send(writer, Message::new(data)).await?;
                    }
                    Err(e) => {
                        log::debug!(msg = %msg.id, "failed to decrypt challenge: {}", e);
                        let data = Client::Error {
                            re: msg.id,
                            code: Some(ErrorCode::DecryptionFailed),
                            msg: None
                        };
                        send(writer, Message::new(data)).await?;
                    }
                }
            Server::Bridge { ext, int, auth } =>
                if self.has_capacity() {
                    let ext = match check_addr("external", msg.id, ext, &self.config.external) {
                        Ok(addr) => addr,
                        Err(msg) => {
                            send(writer, msg).await?;
                            return Ok(())
                        }
                    };
                    let int = match check_addr("internal", msg.id, int, &self.config.internal) {
                        Ok(addr) => addr,
                        Err(msg) => {
                            send(writer, msg).await?;
                            return Ok(())
                        }
                    };
                    let config = self.config.clone();
                    let client = self.client.clone();
                    let auth = auth.into_owned();
                    let id = msg.id;
                    let version = self.version;
                    self.connect_tasks.push(spawn(async move {
                        let c = connection::establish(id, &config, &version, &client, &ext, auth);
                        let r = timeout(config.connect_timeout, c).await
                            .map_err(From::from)
                            .and_then(identity);
                        ConnectResult { re: id, addr: int, conn: r }
                    }))
                } else {
                    let data = Client::Error {
                        re: msg.id,
                        code: Some(ErrorCode::AtCapacity),
                        msg: None
                    };
                    send(writer, Message::new(data)).await?;
                    self.notify_cap = true
                }
            Server::TestConnect { int } =>
                if self.has_capacity() {
                    let id  = msg.id;
                    let int = match check_addr("internal", msg.id, int, &self.config.internal) {
                        Ok(addr) => addr,
                        Err(msg) => {
                            send(writer, msg).await?;
                            return Ok(())
                        }
                    };
                    let config = self.config.clone();
                    self.test_tasks.push(spawn(async move {
                        TestResult {
                            re: id,
                            result: connection::connect(id, &config, &int).await.map(|_| ())
                        }
                    }))
                } else {
                    let data = Client::Error {
                        re: msg.id,
                        code: Some(ErrorCode::AtCapacity),
                        msg: None
                    };
                    send(writer, Message::new(data)).await?;
                    self.notify_cap = true
                }
            Server::DataAddress { .. } => {
                log::error!(msg = %msg.id, "unexpected data address on control connection")
            }
            Server::Terminate { reason } => {
                log::error!(msg = %msg.id, ?reason, "connection terminated by gateway");
                return Err(Error::Terminated(reason))
            }
        }
        Ok(())
    }

    /// Handle connection attempt result.
    async fn on_established(&mut self, writer: &mut Writer, c: ConnectResult) -> Result<(), CborError> {
        let ConnectResult { re, addr, conn } = c;
        match conn {
            Err(e) => {
                log::debug!(%re, "could not connect to external host: {}", e);
                let data = Client::Error {
                    re,
                    code: Some(ErrorCode::CouldNotConnect),
                    msg: Some(Cow::Owned(e.to_string()))
                };
                send(writer, Message::new(data)).await?;
            }
            Ok(conn) => {
                log::debug!(%re, "connected to external host");
                let data = Client::Established {
                    re,
                    data: Opaque {
                        key_id: conn.witness.key_id,
                        nonce: conn.witness.nonce,
                        value: Cow::Borrowed(&conn.witness.value)
                    }
                };
                send(writer, Message::new(data)).await?;
                let config = self.config.clone();
                self.transfer_tasks.push(spawn(async move {
                    let r = connection::bridge(re, &config, conn, &addr).await;
                    TransferResult { re, addr, result: r }
                }))
            }
        }
        Ok(())
    }

    /// Handle connection test result.
    async fn on_connect_test(&mut self, writer: &mut Writer, r: TestResult) -> Result<(), CborError> {
        let TestResult { re, result } = r;
        let data = match result {
            Ok(()) => {
                log::debug!(%re, "connected to internal host");
                Client::TestConnectSuccess { re }
            }
            Err(e) => {
                log::debug!(%re, "could not connect to internal host: {}", e);
                Client::Error {
                    re,
                    code: Some(ErrorCode::CouldNotConnect),
                    msg: Some(Cow::Owned(e.to_string()))
                }
            }
        };
        send(writer, Message::new(data)).await?;
        Ok(())
    }

    /// Handle data transfer result.
    fn on_finished(&mut self, result: TransferResult) {
        match result.result {
            Ok(out) =>
                log::debug! {
                    re = %result.re,
                    from = %out.from,
                    to = %out.to,
                    rx = ?out.ext_to_int,
                    tx = ?out.int_to_ext
                },
            Err(e) => {
                log::warn!(re = %result.re, addr = %result.addr.addr(), "connection error: {}", e);
            }
        }
    }

    /// Connect to control server (with exponential backoff between failures).
    async fn connect(&mut self) -> (Reader, Writer) {
        async fn try_connect(client: &tls::Client, version: &Version, cfg: &Config) -> Result<(Reader, Writer), Error> {
            let hostname = cfg.control.host.as_ref();
            let host_str: &str = hostname.into();
            let port = cfg.control.port;
            log::debug!("connecting to {}:{} ...", host_str, port);
            let iter = net::lookup_host((host_str, port)).await?;
            let future = client.connect_any(iter, hostname);
            let stream = timeout(cfg.connect_timeout, future).await??;
            let (r, w) = io::split(stream);
            let mut w  = Writer::new(w.compat_write());
            let pubkey = cfg.private_key.public_key();
            let hello  = Client::Hello {
                pubkey: Cow::Borrowed(pubkey.as_bytes()[..].into()),
                connection: ConnectionType::Control,
                agent_version: *version
            };
            send(&mut w, Message::new(hello)).await?;
            Ok((Reader::new(r.compat()), w))
        }
        let host = self.config.control.host.as_ref();
        let port = self.config.control.port;
        loop {
            if let Some(ts) = self.connected_timestamp {
                let delta = Instant::now() - ts;
                if let Some(d) = Duration::from_secs(5).checked_sub(delta) {
                    log::debug!("waiting {}s ...", d.as_secs());
                    sleep(d).await
                }
            }
            match try_connect(&self.client, &self.version, &self.config).await {
                Ok((r, w)) => {
                    log::debug!("connected to control server: {}:{}", <&str>::from(host), port);
                    self.failures = 0;
                    self.ping_state = PingState::Idle;
                    self.connected_timestamp = Some(Instant::now());
                    return (r, w)
                }
                Err(e) => {
                    self.connected_timestamp = None;
                    let duration = Duration::from_secs(2u64.pow(self.failures));
                    log::warn! {
                        "failed to connect to {}:{}: {}; trying again in {} ...",
                        <&str>::from(host),
                        port,
                        e,
                        format_duration(duration)
                    };
                    sleep(duration).await;
                    if self.failures < 6 { self.failures += 1 }
                }
            }
        }
    }

    /// Reconnect to control server (with exponential backoff between failures).
    ///
    /// We consume the existing reader and writer to trigger an immediate
    /// close of the current connection.
    async fn reconnect(&mut self, _r: Reader, _w: Writer) -> (Reader, Writer) {
        let rw = self.connect().await;
        if !self.connect_tasks.is_empty() {
            // Cancel all ongoing connect attempts, as we can not report back
            // to the original control server. Data transfers should not be
            // affected.
            for j in self.connect_tasks.iter() {
                j.abort()
            }
            self.connect_tasks = futures_unordered()
        }
        rw
    }

    /// If necessary, tell control server that we can handle more connections.
    async fn notify_capacity(&mut self, writer: &mut Writer) -> Result<(), CborError> {
        if self.notify_cap && self.has_capacity() {
            send(writer, Message::new(Client::Available)).await?;
            self.notify_cap = false
        }
        Ok(())
    }

    /// Do we still have capacity for more connections?
    fn has_capacity(&self) -> bool {
        let n = self.connect_tasks.len()
            .saturating_add(self.transfer_tasks.len())
            .saturating_sub(2); // do not count sentinel tasks
        n < usize::from(self.config.max_connections)
    }
}

/// Create a new `FuturesUnordered` value with a sentinel task.
///
/// The sentinel will never finish and ensures that awaiting on an otherwise
/// empty FuturesUnordered will not immediately produce a `Poll::Ready(None)`.
fn futures_unordered<T: Send + 'static>() -> FuturesUnordered<JoinHandle<T>> {
    let f = FuturesUnordered::new();
    f.push(spawn(future::pending()));
    f
}

/// Check that an address is whitelisted.
fn check_addr<'a, 'b>
    ( ctx: &str
    , id: Id
    , addr: Address<'_>
    , whitelist: &[Network]
    ) -> Result<CheckedAddr<'a>, Message<Client<'b>>>
{
    match CheckedAddr::check(addr.into_owned(), whitelist) {
        Ok(addr)  => Ok(addr),
        Err(addr) => {
            log::error!(address = %addr, "{} address not allowed", ctx);
            let msg = Message::new(Client::Error {
                re: id,
                code: Some(ErrorCode::AddressNotAllowed),
                msg: None
            });
            Err(msg)
        }
    }
}
