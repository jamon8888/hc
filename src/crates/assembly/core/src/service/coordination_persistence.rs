use crate::util::errors::AlexResult;
use alex_services_core::coordination_persistence as storage;
use rusqlite::Connection;

pub(crate) fn initialize_coordination_schema(connection: &Connection) -> AlexResult<()> {
    storage::initialize_coordination_schema(connection).map_err(Into::into)
}
pub(crate) fn validate_coordination_agent_id(agent_id: &str) -> AlexResult<()> {
    storage::validate_coordination_agent_id(agent_id).map_err(Into::into)
}
