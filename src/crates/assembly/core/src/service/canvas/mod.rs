//! Canvas service compatibility facade.
//!
//! Concrete Canvas storage and compilation live in `alex-services-integrations`.
//! Keep this module as the legacy `alex_core::service::canvas` import path
//! while callers migrate to the provider owner.

pub use alex_services_integrations::canvas::{
    compile_canvas_component_js, compile_canvas_html, compile_canvas_source, CanvasMemoryStore,
    CanvasService,
};
