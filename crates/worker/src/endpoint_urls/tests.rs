use super::graphql_url;

/// Confirms worker GraphQL endpoint URL is rendered consistently.
#[test]
fn worker_graphql_url_is_built_consistently() {
    let server_url = "http://127.0.0.1:8080";

    assert_eq!(graphql_url(server_url), "http://127.0.0.1:8080/graphql");
}
