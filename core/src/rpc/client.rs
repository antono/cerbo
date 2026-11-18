//! A blocking client for the daemon's socket: connect, handshake, call.
//!
//! Used by the CLI and the MCP shim, and by the daemon's own tests. The
//! desktop app wraps the same protocol in its own async client.

use super::{
    check_protocol, AdminAction, AdminInfo, AdminReply, AdminRequest, Call, ClientHello, ClientKind, Event, Hello, Request, RpcError, ServerMessage, CODE_CLIENT_TIMEOUT,
    PROTOCOL_VERSION,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// How long [`Client::call`] waits for an answer before giving up. A call
/// that may wait for a person (an agent write at `approve`) uses
/// [`Client::call_with_timeout`] with the approval deadline instead.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// What [`Client::read_message`] reports when its time ran out.
const TIMED_OUT: &str = "the daemon did not answer in time";

/// Why a connection could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectError {
    /// No daemon answers on the socket (not running, or a stale socket).
    Unavailable(String),
    /// The daemon runs another protocol version; restart it. Nothing but the
    /// handshake was sent.
    VersionMismatch(RpcError),
    /// The daemon refused the connection, for example an agent's vault is not
    /// one it serves.
    Rejected(RpcError),
    /// The daemon garbled the handshake.
    Refused(String),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectError::Unavailable(m) | ConnectError::Refused(m) => write!(f, "{m}"),
            ConnectError::VersionMismatch(e) | ConnectError::Rejected(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConnectError {}

/// What the daemon said in its welcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Welcome {
    pub protocol: u32,
    pub daemon_version: String,
    pub pid: u32,
}

/// Why an administration line got no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminError {
    /// Nobody listens on the socket.
    Unavailable(String),
    /// Something listens but does not know administration lines: a daemon
    /// older than they are.
    Unsupported,
    /// The connection failed after it was made.
    Failed(String),
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminError::Unavailable(m) | AdminError::Failed(m) => write!(f, "{m}"),
            AdminError::Unsupported => write!(f, "the daemon is too old to answer administration lines"),
        }
    }
}

impl std::error::Error for AdminError {}

/// Ask the daemon at `socket` for its identity, or to stop, with the
/// version-independent administration line: it works whatever protocol
/// version the daemon speaks, so a stale daemon can still be inspected and
/// stopped after an upgrade.
pub fn admin(socket: &Path, action: AdminAction) -> Result<AdminInfo, AdminError> {
    let mut stream = UnixStream::connect(socket)
        .map_err(|e| AdminError::Unavailable(format!("cannot connect to {}: {e}", socket.display())))?;
    stream.set_read_timeout(Some(CALL_TIMEOUT)).ok();
    let mut line =
        serde_json::to_vec(&AdminRequest { admin: action }).map_err(|e| AdminError::Failed(e.to_string()))?;
    line.push(b'\n');
    stream.write_all(&line).and_then(|()| stream.flush()).map_err(|e| AdminError::Failed(format!("send: {e}")))?;
    let mut answer = String::new();
    match BufReader::new(stream).read_line(&mut answer) {
        // Closed without an answer: not understood.
        Ok(0) => Err(AdminError::Unsupported),
        Ok(_) => serde_json::from_str::<AdminReply>(answer.trim_end())
            .map(|reply| reply.admin)
            .map_err(|_| AdminError::Unsupported),
        Err(e) => Err(AdminError::Failed(e.to_string())),
    }
}

/// One connection to the daemon.
pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next_id: u64,
    events: VecDeque<Event>,
    responses: HashMap<u64, Result<Value, RpcError>>,
    partial: String,
    welcome: Welcome,
    /// How long the current wait for an answer lasts.
    call_timeout: Duration,
    /// How long [`Client::call`] waits; [`CALL_TIMEOUT`] unless changed.
    default_timeout: Duration,
}

impl Client {
    /// Connect to the socket and complete the handshake. On a protocol
    /// mismatch no request is ever sent.
    pub fn connect(socket: &Path, kind: ClientKind, name: Option<&str>) -> Result<Self, ConnectError> {
        Self::connect_full(socket, kind, name, None, PROTOCOL_VERSION)
    }

    /// [`Self::connect`] for a connection bound to one vault, as an agent's
    /// must be: the daemon then refuses any call about another vault.
    pub fn connect_bound(
        socket: &Path,
        kind: ClientKind,
        name: Option<&str>,
        vault: &str,
    ) -> Result<Self, ConnectError> {
        Self::connect_full(socket, kind, name, Some(vault), PROTOCOL_VERSION)
    }

    /// [`Self::connect`] announcing `protocol` instead of the built-in one.
    /// For tests of the mismatch path.
    pub fn connect_with_protocol(
        socket: &Path,
        kind: ClientKind,
        name: Option<&str>,
        protocol: u32,
    ) -> Result<Self, ConnectError> {
        Self::connect_full(socket, kind, name, None, protocol)
    }

    pub(super) fn connect_full(
        socket: &Path,
        kind: ClientKind,
        name: Option<&str>,
        vault: Option<&str>,
        protocol: u32,
    ) -> Result<Self, ConnectError> {
        let stream = UnixStream::connect(socket)
            .map_err(|e| ConnectError::Unavailable(format!("cannot connect to {}: {e}", socket.display())))?;
        stream.set_read_timeout(Some(CALL_TIMEOUT)).ok();
        let writer = stream.try_clone().map_err(|e| ConnectError::Unavailable(e.to_string()))?;
        let mut client = Self {
            reader: BufReader::new(stream),
            writer,
            next_id: 1,
            events: VecDeque::new(),
            responses: HashMap::new(),
            partial: String::new(),
            welcome: Welcome { protocol: 0, daemon_version: String::new(), pid: 0 },
            call_timeout: CALL_TIMEOUT,
            default_timeout: CALL_TIMEOUT,
        };

        let hello = ClientHello {
            hello: Hello {
                protocol,
                client: kind,
                name: name.map(str::to_string),
                vault: vault.map(str::to_string),
            },
        };
        client.send_line(&hello).map_err(|e| ConnectError::Unavailable(format!("handshake: {e}")))?;

        match client.read_message(None).map_err(ConnectError::Refused)? {
            Some(ServerMessage::Welcome { protocol, daemon_version, pid }) => {
                // The daemon vouches for its own version; check it as well, so
                // an old daemon that never checked is refused here.
                check_protocol(protocol).map_err(ConnectError::VersionMismatch)?;
                client.welcome = Welcome { protocol, daemon_version, pid };
                Ok(client)
            }
            Some(ServerMessage::Error { error }) if error.code == super::CODE_PROTOCOL_MISMATCH => {
                Err(ConnectError::VersionMismatch(error))
            }
            Some(ServerMessage::Error { error }) => Err(ConnectError::Rejected(error)),
            Some(other) => Err(ConnectError::Refused(format!("unexpected answer to the handshake: {other:?}"))),
            None => Err(ConnectError::Refused("the daemon closed the connection".to_string())),
        }
    }

    /// What the daemon said about itself when connecting.
    pub fn welcome(&self) -> &Welcome {
        &self.welcome
    }

    /// Change how long [`Self::call`] waits for an answer (60 s by default).
    /// For tests, and for callers that know their daemon is slow.
    pub fn set_default_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
        self.call_timeout = timeout;
    }

    /// Send `call` and wait for its answer. Events that arrive meanwhile are
    /// kept for [`Self::next_event`].
    pub fn call(&mut self, call: Call) -> Result<Value, RpcError> {
        self.call_with_timeout(call, self.default_timeout)
    }

    /// [`Self::call`] waiting up to `timeout` for the answer. On a timeout the
    /// error has code [`CODE_CLIENT_TIMEOUT`]; the request may still be
    /// pending in the daemon, so the caller should drop this connection (the
    /// daemon then rejects what the connection left pending).
    pub fn call_with_timeout(&mut self, call: Call, timeout: Duration) -> Result<Value, RpcError> {
        let id = self.send(call)?;
        self.call_timeout = timeout;
        let answer = self.wait(id);
        self.call_timeout = self.default_timeout;
        answer
    }

    /// Send `call` without waiting; collect the answer with [`Self::wait`].
    /// Requests sent one after another are queued by the daemon in that order.
    pub fn send(&mut self, call: Call) -> Result<u64, RpcError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_line(&Request { id, call }).map_err(|e| RpcError::failed(format!("send: {e}")))?;
        Ok(id)
    }

    /// The answer to request `id` from [`Self::send`].
    pub fn wait(&mut self, id: u64) -> Result<Value, RpcError> {
        loop {
            if let Some(answer) = self.responses.remove(&id) {
                return answer;
            }
            match self.read_message(None) {
                Ok(Some(message)) => self.stash(message),
                Ok(None) => return Err(RpcError::failed("the daemon closed the connection")),
                Err(e) if e == TIMED_OUT => return Err(RpcError::new(CODE_CLIENT_TIMEOUT, e)),
                Err(e) => return Err(RpcError::failed(e)),
            }
        }
    }

    /// The next event, waiting up to `timeout`. `Ok(None)` when none came.
    pub fn next_event(&mut self, timeout: Duration) -> Result<Option<Event>, String> {
        if let Some(event) = self.events.pop_front() {
            return Ok(Some(event));
        }
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return Ok(None);
            }
            match self.read_message(Some(left))? {
                Some(message) => {
                    self.stash(message);
                    if let Some(event) = self.events.pop_front() {
                        return Ok(Some(event));
                    }
                }
                None => return Ok(None),
            }
        }
    }

    fn stash(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Event { event } => self.events.push_back(event),
            ServerMessage::Response { id, result, error } => {
                self.responses.insert(id, error.map_or(Ok(result), Err));
            }
            ServerMessage::Welcome { .. } | ServerMessage::Error { .. } => {}
        }
    }

    fn send_line<T: serde::Serialize>(&mut self, message: &T) -> std::io::Result<()> {
        let mut line = serde_json::to_vec(message).map_err(std::io::Error::other)?;
        line.push(b'\n');
        self.writer.write_all(&line)?;
        self.writer.flush()
    }

    /// One line from the daemon. `timeout` of `None` uses the current call
    /// timeout (the total wait is one timeout per line read);
    /// `Ok(None)` is end of stream or, with a timeout, nothing arrived.
    fn read_message(&mut self, timeout: Option<Duration>) -> Result<Option<ServerMessage>, String> {
        self.reader
            .get_ref()
            .set_read_timeout(Some(timeout.unwrap_or(self.call_timeout)))
            .map_err(|e| e.to_string())?;
        loop {
            match self.reader.read_line(&mut self.partial) {
                Ok(0) => return Ok(None),
                Ok(_) if self.partial.ends_with('\n') => {
                    let line = std::mem::take(&mut self.partial);
                    return serde_json::from_str(line.trim_end())
                        .map(Some)
                        .map_err(|e| format!("unreadable message from the daemon: {e}"));
                }
                Ok(_) => continue,
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    return if timeout.is_some() { Ok(None) } else { Err(TIMED_OUT.into()) };
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}
