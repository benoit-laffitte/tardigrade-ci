/// API key validator used by bootstrap authentication flow.
pub struct ApiKeyAuth {
    /// Minimal bootstrap auth strategy for API key validation.
    expected_key: String,
}

impl ApiKeyAuth {
    /// Builds validator with one expected API key value.
    pub fn new(expected_key: impl Into<String>) -> Self {
        Self {
            expected_key: expected_key.into(),
        }
    }

    /// Verifies provided API key against configured expected value.
    pub fn verify(&self, provided: &str) -> bool {
        // Simple equality check; can be replaced by stronger auth schemes later.
        provided == self.expected_key
    }
}

#[cfg(test)]
mod tests;
