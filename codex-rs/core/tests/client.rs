mod support;

// Client transport tests moved to `core-test-suites/client-transport`.
#[path = "suite/live_cli.rs"]
mod live_cli;
#[path = "suite/responses_api_proxy_headers.rs"]
mod responses_api_proxy_headers;
#[path = "suite/rmcp_client.rs"]
mod rmcp_client;
