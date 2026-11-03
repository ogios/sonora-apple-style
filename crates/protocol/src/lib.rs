//! Versioned commands and subscriptions between Sonora's local server and its clients.

mod client;
mod endpoint;
mod framing;

pub use client::{Client, ConnectionState};
pub use endpoint::{connect, listen};
pub use framing::{read, write};

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The wire contract required by both ends of a connection.
pub const VERSION: u32 = 2;

/// An operation on a named service object. Arguments retain their declared positional order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Call {
    pub object: String,
    pub method: String,
    pub arguments: Vec<Value>,
    /// The revision of the object the user acted on, required for index-based mutations.
    pub revision: Option<u64>,
}

impl Call {
    pub fn new(
        object: impl Into<String>,
        method: impl Into<String>,
        arguments: Vec<Value>,
    ) -> Self {
        Self {
            object: object.into(),
            method: method.into(),
            arguments,
            revision: None,
        }
    }
}

/// A client request. Request IDs are scoped to one connection and are never replayed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub body: RequestBody,
}

/// Handshake, state subscription and command operations supported by the local service.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RequestBody {
    Hello { version: u32 },
    Snapshot,
    Subscribe,
    Call(Call),
}

/// One coherent snapshot, taken on the service thread before subsequent events are published.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub instance: String,
    pub revision: u64,
    pub objects: BTreeMap<String, Value>,
}

/// A changed service object. The connection preserves publication order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub instance: String,
    pub revision: u64,
    pub objects: BTreeMap<String, Value>,
    #[serde(default)]
    pub removed: Vec<String>,
}

/// A typed model event and the state it describes, delivered together in publication order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    pub instance: String,
    pub revision: u64,
    pub object: String,
    pub value: Value,
    pub objects: BTreeMap<String, Value>,
}

/// Ordered notifications retained for a GUI until its event loop processes them.
pub enum Notice {
    Changed(Event),
    Signal(Signal),
}

/// Replies and subscription updates sent by the server.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Message {
    Welcome {
        version: u32,
        instance: String,
        pid: u32,
    },
    Reply {
        id: u64,
        result: Result<Value, String>,
    },
    Changed(Event),
    Signal(Signal),
}
