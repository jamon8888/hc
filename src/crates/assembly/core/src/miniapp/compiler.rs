//! MiniApp compiler compatibility facade.

pub use alex_product_domains::miniapp::compiler::{
    MiniAppCompileError, MiniAppCompileRequest, MiniAppCompileResult,
};

use crate::miniapp::types::{MiniAppPermissions, MiniAppSource};
use crate::util::errors::{AlexError, AlexResult};

/// Compile MiniApp source into full HTML with Import Map, Runtime Adapter, and CSP injected.
pub fn compile(
    source: &MiniAppSource,
    permissions: &MiniAppPermissions,
    app_id: &str,
    app_data_dir: &str,
    workspace_dir: &str,
    appearance_mode: &str,
) -> AlexResult<String> {
    alex_product_domains::miniapp::compiler::compile(
        source,
        permissions,
        app_id,
        app_data_dir,
        workspace_dir,
        appearance_mode,
    )
    .map_err(|e| AlexError::validation(e.to_string()))
}

pub fn compile_with_request(
    source: &MiniAppSource,
    permissions: &MiniAppPermissions,
    request: &MiniAppCompileRequest,
) -> AlexResult<String> {
    alex_product_domains::miniapp::compiler::compile_with_request(
        source,
        permissions,
        request,
    )
    .map_err(|e| AlexError::validation(e.to_string()))
}

pub fn compile_market_with_request(
    source: &MiniAppSource,
    permissions: &MiniAppPermissions,
    request: &MiniAppCompileRequest,
) -> AlexResult<String> {
    alex_product_domains::miniapp::compiler::compile_market_with_request(
        source,
        permissions,
        request,
    )
    .map_err(|e| AlexError::validation(e.to_string()))
}
