//! Plugin ecosystem validation: hooks, agents, commands, manifest,
//! and cross-component consistency.

pub mod manifest;

pub use manifest::{validate_manifest, PluginManifest};
