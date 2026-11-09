//! Serialization of authoritative state for ordered IPC subscriptions.

use crate::runtime::App;

/// A service model's serializable presentation state.
pub(crate) trait Model: 'static {
    fn remote_snapshot(&self, cx: &App) -> serde_json::Value;
}

/// This runtime owns provider state and never mirrors another server.
pub(crate) fn is_remote(_: &App) -> bool {
    false
}
/// Preferences are loaded from the authoritative persistent stores.
pub(crate) fn take_settings(_: &mut App) -> Option<crate::AppSettings> {
    None
}
/// A GUI subscription handle is only present in the presentation build.
pub(crate) struct Lease;
/// The service manages audio subscriptions directly through its signal entity.
pub(crate) fn lease(_: &str, _: &mut App) -> Option<Lease> {
    None
}

/// Serializes presentation fields while keeping provider and task handles private.
macro_rules! remote_model {
    ($type:ty, $kind:literal, $dynamic:literal, {$($field:ident),+ $(,)?} $(, events $event:ty)?) => {
        impl crate::client::Model for $type {
            fn remote_snapshot(&self, _cx: &crate::runtime::App) -> serde_json::Value {
                serde_json::json!({$(stringify!($field): &self.$field),+})
            }
        }
    };
}
