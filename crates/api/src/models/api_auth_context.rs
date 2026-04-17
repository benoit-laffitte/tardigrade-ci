/// Per-request authentication context injected by server middleware.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiAuthContext {
    pub status: ApiAuthStatus,
}

/// Authentication verification outcomes used by API adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiAuthStatus {
    Disabled,
    Verified,
    Missing,
    Invalid,
}

impl Default for ApiAuthContext {
    /// Provides disabled-by-default auth context when no middleware data is injected.
    fn default() -> Self {
        Self {
            status: ApiAuthStatus::Disabled,
        }
    }
}
