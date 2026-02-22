//! Plugin ecosystem validation: hooks, agents, commands, manifest,
//! and cross-component consistency.

pub mod agent;
pub mod command;
pub mod cross;
pub mod hooks;
pub mod manifest;

pub use agent::validate_agent;
pub use command::validate_command;
pub use cross::validate_cross_component;
pub use hooks::validate_hooks;
pub use manifest::{validate_manifest, PluginManifest};
