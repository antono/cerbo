//! A blocking client for the daemon's socket: connect, handshake, call.
//!
//! Used by the CLI and the MCP shim, and by the daemon's own tests. The
//! desktop app wraps the same protocol in its own async client.

use super::{
    check_protocol, Call, ClientHello, ClientKind, Event, Hello, Request, RpcError, ServerMessage, PROTOCOL_VERSION,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// How long [`Client::call`] waits for an answer before giving up.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Why a connection could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectError {
    /// No daemon answers on the socket (not running, or a stale socket).
    Unavailable(String),
    /// The daemon runs another protocol version; restart it. Nothing but the
    /// handshake was sent.
    VersionMismatch(RpcError),
    /// The daemon refused or garbled the handshake.
    Refused(String),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectError::Unavailable(m) | ConnectError::Refused(m) => write!(f, "{m}"),
            ConnectError::VersionMismatch(e) => write!(f, "{e}"),
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

/// One connection to the daemon.
pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next_id: u64,
    events: VecDeque<Event>,
    responses: HashMap<u64, Result<Value, RpcError>>,
    partial: String,
    welcome: Welcome,
}

impl Client {
    /// Connect to the socket and complete the handshake. On a protocol
    /// mismatch no request is ever sent.
    pub fn connect(socket: &Path, kind: ClientKind, name: Option<&str>) -> Result<Self, ConnectError> {
        Self::connect_with_protocol(socket, kind, name, PROTOCOL_VERSION)
    }

    /// [`Self::connect`] announcing `protocol` instead of the built-in one.
    /// For tests of the mismatch path.
    pub fn connect_with_protocol(
        socket: &Path,
        kind: ClientKind,
        name: Option<&str>,
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
        };

        let hello = ClientHello { hello: Hello { protocol, client: kind, name: name.map(str::to_string) } };
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
            Some(ServerMessage::Error { error }) => Err(ConnectError::Refused(error.message)),
            Some(other) => Err(ConnectError::Refused(format!("unexpected answer to the handshake: {other:?}"))),
            None => Err(ConnectError::Refused("the daemon closed the connection".to_string())),
        }
    }

    /// What the daemon said about itself when connecting.
    pub fn welcome(&self) -> &Welcome {
        &self.welcome
    }

    /// Send `call` and wait for its answer. Events that arrive meanwhile are
    /// kept for [`Self::next_event`].
    pub fn call(&mut self, call: Call) -> Result<Value, RpcError> {
        let id = self.send(call)?;
        self.wait(id)
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
            match self.read_message(None).map_err(RpcError::failed)? {
                Some(message) => self.stash(message),
                None => return Err(RpcError::failed("the daemon closed the connection")),
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

    /// One line from the daemon. `timeout` of `None` uses the call timeout;
    /// `Ok(None)` is end of stream or, with a timeout, nothing arrived.
    fn read_message(&mut self, timeout: Option<Duration>) -> Result<Option<ServerMessage>, String> {
        self.reader
            .get_ref()
            .set_read_timeout(Some(timeout.unwrap_or(CALL_TIMEOUT)))
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
                    return if timeout.is_some() { Ok(None) } else { Err("the daemon did not answer in time".into()) };
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}
