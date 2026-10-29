//! Compatibility facade for MiniApp JS worker processes.

pub use alex_services_integrations::miniapp::worker::{
    JsWorker, MiniAppWorkerEvent, MiniAppWorkerEventFuture, MiniAppWorkerEventSink,
    SharedMiniAppWorkerEventSink,
};
