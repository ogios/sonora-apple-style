//! Browser requests owned by a connected GUI; cookies return only through acknowledged calls.

use serde::{Deserialize, Serialize};

/// A throwaway provider sign-in or proof-of-origin page that needs a native browser engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Target {
    pub url: String,
    pub landing: String,
    pub domain: String,
    pub proof: Vec<String>,
    /// Provider name for a sign-in window; the GUI localizes its title.
    pub title: String,
    pub agent: Option<String>,
    pub script: Option<String>,
}

/// The latest instruction for one page. Script sequence numbers prevent repeated execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page {
    pub id: u64,
    pub client: String,
    pub target: Target,
    pub script: Option<(u64, String)>,
}

/// A page result carries credentials directly to the owning server operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Reply {
    Closed,
    Cookies(String),
}
