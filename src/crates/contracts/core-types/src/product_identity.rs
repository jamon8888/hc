//! Immutable identity facts compiled into first-party product artifacts.
//!
//! Product build adapters derive these values from the selected product
//! definition. They are compile-time facts rather than runtime configuration:
//! a running process cannot switch product identity or storage namespaces.

const DEFAULT_PRODUCT_ID: &str = "alex";
const DEFAULT_DATA_NAMESPACE: &str = "alex";
const DEFAULT_HIDDEN_DATA_DIRECTORY: &str = ".openbitfun";

/// Stable identifier for the product family represented by this artifact.
pub const fn product_id() -> &'static str {
    match option_env!("ALEX_PRODUCT_ID") {
        Some(value) => value,
        None => DEFAULT_PRODUCT_ID,
    }
}

/// Storage namespace paired with [`product_id`].
pub const fn data_namespace() -> &'static str {
    match option_env!("ALEX_DATA_NAMESPACE") {
        Some(value) => value,
        None => DEFAULT_DATA_NAMESPACE,
    }
}

/// Hidden directory name derived from [`data_namespace`] by the build adapter.
///
/// The same name is used for the product home and project-local product data.
pub const fn hidden_data_directory() -> &'static str {
    match option_env!("ALEX_HIDDEN_DATA_DIRECTORY") {
        Some(value) => value,
        None => DEFAULT_HIDDEN_DATA_DIRECTORY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_identity_is_alex() {
        assert_eq!(product_id(), "alex");
        assert_eq!(data_namespace(), "alex");
        assert_eq!(hidden_data_directory(), ".openbitfun");
    }
}
