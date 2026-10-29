//! Bundled tool packages that ship inside the Alex product.
//!
//! The implementation lives in the service owner crate; this module is the
//! stable path the product surfaces use.

pub use alex_services_core::bundled_tools::{
    bundled_tool_host_tag, bundled_tool_package, BundledToolPackage,
};
