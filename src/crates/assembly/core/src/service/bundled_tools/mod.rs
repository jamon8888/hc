//! Bundled tool packages that ship inside the OpenBitFun product.
//!
//! The implementation lives in the service owner crate; this module is the
//! stable path the product surfaces use.

pub use openbitfun_services_core::bundled_tools::{
    bundled_tool_host_tag, bundled_tool_package, BundledToolPackage,
};
