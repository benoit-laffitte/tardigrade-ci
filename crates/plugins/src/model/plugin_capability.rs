use serde::Deserialize;

/// Capability families used by plugin permissions and policy checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Network,
    Filesystem,
    Secrets,
    RuntimeHooks,
}
