use crate::{Reader, Writer, version};
use crate::config::Config;
use crate::error::Error;
use crate::stream::{self, streamer};
use crate::tls;
use futures::future;
use futures::stream::{BoxStream, FuturesUnordered, SelectAll, StreamExt};
use humantime::format_duration;
use protocol::{AgentId, Client, ErrorCode, Id, Message, Server};
use protocol::{Reason, Version};
use scopeguard::{ScopeGuard, guard};
use sealed_boxes::decrypt;
use std::borrow::Cow;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use tokio::net;
use tokio::{select, spawn};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::compat::TokioAsyncReadCompatExt;
use util::io::{send, recv};

/// The connection agent.
pub struct Agent {
    id: AgentId,
    version: Version,
    config: Arc<Config>,
    client: tls::Client,
    attempt: u8,
    ping_state: PingState,
    streams: FuturesUnordered<JoinHandle<Result<(), Error>>>,
    tests: FuturesUnordered<JoinHandle<(Id, Option<ErrorCode>)>>,
    drainage: SelectAll<BoxStream<'static, yamux::Stream>>,
    online: bool
}

/// Connection parts.
struct Connection {
    /// The task handling the TCP connection.
    task: JoinHandle<Result<(), yamux::ConnectionError>>,
    /// The control handle to eventually close the connection.
    ctrl: yamux::Control,
    /// The control stream reader.
    reader: Reader,
    /// The control stream writer.
    writer: Writer,
    /// New inbound streams opened from remote.
    inbound: mpsc::Receiver<yamux::Stream>
}

impl Drop for Agent {
    fn drop(&mut self) {
        for task in self.streams.iter() {
            task.abort()
        }
        for task in self.tests.iter() {
            task.abort()
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.inbound.close();
        self.task.abort();
    }
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
            id: AgentId::from(cfg.secret_key.public_key()),
            version: crate::version()?,
            config: Arc::new(cfg),
            client,
            attempt: 0,
            ping_state: PingState::Idle,
            streams: futures_unordered(),
            tests: futures_unordered(),
            drainage: {
                let mut s = SelectAll::new();
                s.push(futures::stream::pending().boxed());
                s
            },
            online: false
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
        let mut connection = self.connect().await;

        log::info! {
            agent   = %self.id,
            version = %version().expect("valid version"),
            "up and running"
        };

        // Event processing.
        loop {
            log::trace!("awaiting event ...");
            select! {
                // A new server message.
                message = recv(&mut connection.reader) => match message {
                    Err(e) => {
                        log::error!("error reading from server: {}", e);
                        connection = self.reconnect(connection).await
                    }
                    Ok(None) => {
                        log::warn!("control channel closed by server, reconnecting ...");
                        connection = self.reconnect(connection).await
                    }
                    Ok(Some(m)) => match self.on_message(&mut connection.writer, m).await {
                        Err(Error::Terminated(reason)) => return reason,
                        Err(e) => {
                            log::error!("failed to answer server message: {}", e);
                            connection = self.reconnect(connection).await
                        }
                        Ok(Some(mut conn)) => {
                            mem::swap(&mut connection, &mut conn);
                            let drain = futures::stream::unfold(conn, |mut conn| async move {
                                conn.inbound.recv().await.map(|s| (s, conn))
                            });
                            self.drainage.push(drain.boxed())
                        }
                        Ok(None) => {}
                    }
                },

                // A new inbound stream has been opened.
                stream = connection.inbound.recv(), if self.online => match stream {
                    None => {
                        log::debug!("connection to server lost");
                        self.online = false
                    }
                    Some(s) => {
                        log::debug!("new inbound stream");
                        let cfg = self.config.clone();
                        self.streams.push(spawn(streamer(cfg, s)))
                    }
                },

                // A new inbound stream has been opened.
                stream = self.drainage.next() => if let Some(s) = stream {
                    log::debug!("new inbound stream while draining");
                    let cfg = self.config.clone();
                    self.streams.push(spawn(streamer(cfg, s)))
                },

                // A connection test finished.
                Some(test) = self.tests.next() => match test {
                    Err(e) => {
                        if e.is_panic() {
                            log::error!("test task panic: {}", e)
                        } else {
                            log::warn!("test task error: {}", e)
                        }
                    }
                    Ok((re, code)) => {
                        let data = Client::Test { re, code };
                        if let Err(e) = send(&mut connection.writer, Message::new(data)).await {
                            log::warn!(id = %re, "error sending message to server: {}", e);
                            connection = self.reconnect(connection).await
                        }
                    }
                },

                // A stream completed.
                Some(result) = self.streams.next() => {
                    if let Err(e) = result {
                        if e.is_panic() {
                            log::error!("stream task panic: {}", e)
                        } else {
                            log::warn!("stream task error: {}", e)
                        }
                    }
                }

                // Awaiting pong or time to send the next ping.
                () = sleep(self.config.ping_frequency) => match self.ping_state {
                    PingState::Idle => {
                        let msg = Message::new(Client::Ping);
                        if let Err(e) = send(&mut connection.writer, &msg).await {
                            log::warn!("error sending message to server: {}", e);
                            connection = self.reconnect(connection).await
                        } else {
                            self.ping_state = PingState::Awaiting(msg.id)
                        }
                    }
                    PingState::Awaiting(id) => {
                        log::warn!(%id, "no pong from server");
                        connection = self.reconnect(connection).await
                    }
                }
            }
        }
    }

    /// Handle message from server.
    async fn on_message(&mut self, writer: &mut Writer, msg: Message<Server<'_>>) -> Result<Option<Connection>, Error> {
        log::trace!(id = %msg.id, online = %self.online, data = ?msg.data, "received message");

        match msg.data {
            Some(Server::Accepted) => {
                self.attempt = 0
            }
            Some(Server::Ping) => {
                if self.online {
                    send(writer, Message::new(Client::Pong { re: msg.id })).await?;
                }
            }
            Some(Server::Pong { re }) => {
                if let PingState::Awaiting(p) = self.ping_state {
                    if re == p {
                        self.ping_state = PingState::Idle
                    }
                }
            }
            Some(Server::Challenge { text }) =>
                if self.online {
                    match decrypt(&self.config.secret_key, text.0) {
                        Ok(plain) => {
                            let data = Client::Response {
                                re: msg.id,
                                text: Cow::Borrowed(plain.as_ref().into())
                            };
                            send(writer, Message::new(data)).await?;
                        }
                        Err(e) => {
                            log::warn!(id = %msg.id, "failed to decrypt challenge: {}", e);
                            let data = Client::Error {
                                re: msg.id,
                                code: Some(ErrorCode::DecryptionFailed),
                                msg: None
                            };
                            send(writer, Message::new(data)).await?;
                        }
                    }
                }
            Some(Server::Terminate { reason }) => {
                log::error!(id = %msg.id, ?reason, "connection terminated by gateway");
                return Err(Error::Terminated(reason))
            }
            Some(Server::Test { addr }) =>
                if self.online {
                    match stream::check_addr(addr, &self.config.allowed_addresses) {
                        Err(code) => {
                            let data = Client::Test { re: msg.id, code: Some(code) };
                            send(writer, Message::new(data)).await?;
                        }
                        Ok(addr) => {
                            let id = msg.id;
                            let cf = self.config.clone();
                            self.tests.push(spawn(async move {
                                if let Err(e) = stream::connect(id, &cf, &addr).await {
                                    log::warn!(%id, "test connection failed: {}", e);
                                    (id, Some(ErrorCode::CouldNotConnect))
                                } else {
                                    log::debug!(%id, "test connection suceeded");
                                    (id, None)
                                }
                            }))
                        }
                    }
                }
            Some(Server::SwitchToNewConnection) =>
                if self.online {
                    log::debug!(id = %msg.id, "switching to new connection and draining the existing one");
                    send(writer, Message::new(Client::SwitchingConnection { re: msg.id })).await?;
                    let c = self.connect().await;
                    return Ok(Some(c))
                }
            Some(Server::Error { msg }) => {
                log::error!(?msg, "server error")
            }
            None => {
                log::warn!(id = %msg.id, "ignoring unknown gateway message")
            }
        }
        Ok(None)
    }

    /// Connect to server (with exponential backoff between failures).
    async fn connect(&mut self) -> Connection {
        async fn try_connect(client: &tls::Client, version: &Version, cfg: &Config) -> Result<Connection, Error> {
            let hostname = &cfg.server.host;
            let host_str = hostname.as_str();
            let port = cfg.server.port;
            log::debug!("connecting to {}:{} ...", host_str, port);
            let iter     = net::lookup_host((host_str, port)).await?;
            let future   = client.connect_any(iter, hostname);
            let stream   = timeout(cfg.connect_timeout, future).await??;
            let mut conn = {
                let cfg = yamux::Config::default();
                yamux::Connection::new(stream.compat(), cfg, yamux::Mode::Client)
            };
            let mut ctrl = conn.control();
            let (tx, rx) = mpsc::channel(2048); // channel to announce new inbound streams
            let task     = spawn(async move {
                while let Some(s) = conn.next_stream().await? {
                    match tx.try_send(s) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Closed(_)) => break,
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            log::warn!("dropping inbound stream")
                        }
                    }
                }
                Ok(())
            });
            let task   = guard(task, |t| t.abort()); // in case of error abort the task
            let stream = ctrl.open_stream().await?;
            let (r, w) = futures::io::AsyncReadExt::split(stream);
            let mut w  = Writer::new(w);
            let pubkey = cfg.secret_key.public_key();
            let hello  = Client::Hello {
                pubkey: Cow::Borrowed(pubkey.as_bytes()[..].into()),
                agent_version: *version
            };
            send(&mut w, Message::new(hello)).await?;
            Ok(Connection {
                ctrl,
                reader: Reader::new(r),
                writer: w,
                task: ScopeGuard::into_inner(task),
                inbound: rx
            })
        }
        let host = &self.config.server.host;
        let port = self.config.server.port;
        loop {
            if self.attempt > 0 {
                let d = Duration::from_secs(2u64.pow(self.attempt.into()));
                log::info!("waiting {} before connecting ...", format_duration(d));
                sleep(d).await
            }
            if self.attempt < 6 {
                self.attempt += 1
            }
            match try_connect(&self.client, &self.version, &self.config).await {
                Ok(conn) => {
                    log::info!("connected to server: {}:{}", host.as_str(), port);
                    self.ping_state = PingState::Idle;
                    self.online = true;
                    return conn
                }
                Err(e) => {
                    log::warn!(err = %e, "failed to connect to {}:{}", host.as_str(), port)
                }
            }
        }
    }

    /// Reconnect to server (with exponential backoff between failures).
    ///
    /// We consume the existing reader and writer to trigger an immediate
    /// close of the current connection.
    async fn reconnect(&mut self, mut conn: Connection) -> Connection {
        if let Err(e) = timeout(Duration::from_secs(5), conn.ctrl.close()).await {
            log::warn!("error closing connection: {}", e)
        }
        drop(conn);
        self.online = false;
        self.connect().await
    }
}

/// Create a new `FuturesUnordered` value with a sentinel task.
///
/// The sentinel will never finish and ensures that awaiting on an otherwise
/// empty `FuturesUnordered` will not immediately produce a `Poll::Ready(None)`.
fn futures_unordered<T: Send + 'static>() -> FuturesUnordered<JoinHandle<T>> {
    let f = FuturesUnordered::new();
    f.push(spawn(future::pending()));
    f
}
