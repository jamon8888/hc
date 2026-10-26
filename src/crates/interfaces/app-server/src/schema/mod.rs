//! JSON-RPC wire contract for the Alex app-server surface.
//!
//! Schema types are grouped by product domain while this module re-exports the
//! complete contract. Existing consumers can continue importing types from
//! `alex_app_server::schema` without depending on the internal layout.

pub use alex_app_server_protocol::agent::*;
pub use alex_app_server_protocol::app::*;
pub use alex_app_server_protocol::config::*;
pub use alex_app_server_protocol::event::{
    AgentEventNotification as SessionEventNotification, ConfigEventNotification, ConfigUpdate,
    EventCursor, EventStream, EventStreamState, EventStreamStateNotification,
    FrontendEventNotification, PermissionEventNotification, ResyncDirective, SyncEventsRequest,
    SyncEventsResponse,
};
pub use alex_app_server_protocol::git::*;
pub use alex_app_server_protocol::i18n::*;
pub use alex_app_server_protocol::permission::*;
pub use alex_app_server_protocol::search::*;
pub use alex_app_server_protocol::session::*;
