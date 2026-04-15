/// Builds GraphQL endpoint URL for one API server base URL.
pub(crate) fn graphql_url(server_url: &str) -> String {
    format!("{server_url}/graphql")
}

#[cfg(test)]
mod tests;
