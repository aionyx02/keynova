pub mod action_registry;
pub mod agent_observation;
pub mod agent_runtime;
pub mod automation_engine;
pub mod builtin_command_registry;
pub mod command_router;
pub mod config_manager;
pub mod control_plane;
pub mod event_bus;
pub mod ipc_error;
pub mod knowledge_store;
pub mod observability;
pub mod plugin_runtime;
pub mod search_registry;
pub mod workflow_pipeline;

pub use action_registry::{ActionArena, ActionRegistry};
pub use agent_observation::{prepare_observation, AgentObservationPolicy, PreparedObservation};
pub use agent_runtime::AgentRuntime;
pub use builtin_command_registry::BuiltinCommandRegistry;
pub use command_router::{CommandHandler, CommandResult, CommandRouter};
pub use event_bus::{AppEvent, EventBus};
pub use ipc_error::IpcError;
pub use knowledge_store::{
    ActionLogEntry, AgentAuditEntry, AgentMemoryEntry, KnowledgeStoreHandle,
};
