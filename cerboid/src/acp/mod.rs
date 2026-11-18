//! A hand-written ACP (Agent Client Protocol) client over stdio, version 1.
//!
//! [`AcpClient::spawn`] starts an agent process, speaks newline-delimited
//! JSON-RPC 2.0 with it and reports what it says as [`AcpEvent`]s. The client
//! advertises no `fs` and no `terminal` capability and answers any such
//! request with -32601; that only limits what the agent may ask of the
//! client. Confinement of the agent process itself is the sandbox's job
//! (the command passed to `spawn` is the sandbox wrapper's).
//!
//! ```ignore
//! let (client, mut events) = AcpClient::spawn_argv(&argv, &env, Some(&cwd), ClientOptions::default())?;
//! client.initialize().await?;
//! let session = client.new_session(&vault, &[mcp_server]).await?;
//! let stop = client.prompt(&session.session_id, &[ContentBlock::text("hello")]).await?;
//! while let Ok(event) = events.try_recv() { /* every update of the turn is here */ }
//! ```
//!
//! The consumer must drain the event stream (an unbounded channel: chunks are
//! small and the agent must never be stalled by a slow reader).
//!
//! The scripted fake agent for tests is the `fake-acp-agent` binary of this
//! crate; [`replay`] reads the JSONL logs the client records.

mod client;
pub mod replay;
mod types;
#[cfg(test)]
mod types_tests;

pub use client::{
    AcpClient, AcpError, AcpEvent, AcpEvents, ClientOptions, ExitInfo, PermissionRequest, METHOD_NOT_FOUND,
};
pub use types::*;
