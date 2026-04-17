use std::collections::HashMap;

/// Transport-neutral webhook command consumed by CI service orchestration.
#[derive(Debug, Clone)]
pub(crate) struct ScmWebhookRequest {
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl ScmWebhookRequest {
    /// Builds webhook command from raw header pairs and body bytes.
    pub(crate) fn from_parts<I>(headers: I, body: Vec<u8>) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut normalized_headers = HashMap::new();

        // Keep the first value for each header key to match lookup behavior used by adapters.
        for (key, value) in headers {
            let normalized_key = key.trim().to_ascii_lowercase();
            let normalized_value = value.trim().to_string();
            if normalized_key.is_empty() || normalized_value.is_empty() {
                continue;
            }

            normalized_headers
                .entry(normalized_key)
                .or_insert(normalized_value);
        }

        Self {
            headers: normalized_headers,
            body,
        }
    }

    /// Returns one optional normalized header value when present and non-empty.
    pub(crate) fn header_value(&self, key: &str) -> Option<&str> {
        self.headers
            .get(&key.to_ascii_lowercase())
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// Returns webhook payload body bytes.
    pub(crate) fn body(&self) -> &[u8] {
        &self.body
    }
}
