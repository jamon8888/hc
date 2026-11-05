//! Process-wide model egress policy published by the host composition root.

use alex_runtime_ports::{
    ModelEgressContext, ModelEgressError, ModelEgressErrorKind, ModelEgressPolicy,
};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock, RwLock,
};

static GLOBAL_MODEL_EGRESS_POLICY: OnceLock<RwLock<Option<Arc<dyn ModelEgressPolicy>>>> =
    OnceLock::new();
static GLOBAL_MODEL_EGRESS_GENERATION: AtomicU64 = AtomicU64::new(0);

pub struct RefusingModelEgressPolicy {
    message: String,
}

impl RefusingModelEgressPolicy {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[async_trait]
impl ModelEgressPolicy for RefusingModelEgressPolicy {
    async fn protect_messages(
        &self,
        _messages: Vec<alex_core_types::Message>,
        _context: ModelEgressContext,
    ) -> Result<Vec<alex_core_types::Message>, ModelEgressError> {
        Err(ModelEgressError::new(
            ModelEgressErrorKind::Unavailable,
            self.message.clone(),
        ))
    }
}

#[cfg(test)]
struct TestModelEgressPolicy;

#[cfg(test)]
pub(crate) fn test_passthrough_model_egress_policy() -> Arc<dyn ModelEgressPolicy> {
    Arc::new(TestModelEgressPolicy)
}

#[cfg(test)]
#[async_trait]
impl ModelEgressPolicy for TestModelEgressPolicy {
    async fn protect_messages(
        &self,
        messages: Vec<alex_core_types::Message>,
        _context: ModelEgressContext,
    ) -> Result<Vec<alex_core_types::Message>, ModelEgressError> {
        Ok(messages)
    }
}

pub fn set_global_model_egress_policy(policy: Option<Arc<dyn ModelEgressPolicy>>) {
    let mut slot = GLOBAL_MODEL_EGRESS_POLICY
        .get_or_init(|| RwLock::new(None))
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *slot = policy;
    GLOBAL_MODEL_EGRESS_GENERATION.fetch_add(1, Ordering::Relaxed);
}

pub fn install_refusing_model_egress_policy(message: impl Into<String>) {
    set_global_model_egress_policy(Some(Arc::new(RefusingModelEgressPolicy::new(message))));
}

pub fn global_model_egress_policy() -> Option<Arc<dyn ModelEgressPolicy>> {
    let policy = GLOBAL_MODEL_EGRESS_POLICY
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone();

    if policy.is_some() {
        return policy;
    }

    // Core unit tests construct clients directly, without a product host
    // composition root. Keep those tests focused on their own behavior while
    // production remains fail-closed until a host publishes a real policy.
    #[cfg(test)]
    {
        return Some(Arc::new(TestModelEgressPolicy));
    }

    #[cfg(not(test))]
    {
        None
    }
}

/// Read the policy and its generation while holding the policy read lock.
/// Publication updates both under the matching write lock, so callers never
/// pair a policy from one workspace with the generation of another.
pub fn global_model_egress_snapshot() -> (Option<Arc<dyn ModelEgressPolicy>>, u64) {
    let policy = GLOBAL_MODEL_EGRESS_POLICY
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let generation = GLOBAL_MODEL_EGRESS_GENERATION.load(Ordering::Relaxed);
    (policy, generation)
}

pub fn global_model_egress_generation() -> u64 {
    GLOBAL_MODEL_EGRESS_GENERATION.load(Ordering::Relaxed)
}
