mod support;

#[path = "suite/cli_stream.rs"]
mod cli_stream;
#[path = "suite/client.rs"]
mod client;
#[path = "suite/client_websockets.rs"]
mod client_websockets;
#[path = "suite/live_cli.rs"]
mod live_cli;
#[path = "suite/realtime_conversation.rs"]
mod realtime_conversation;
#[path = "suite/responses_api_proxy_headers.rs"]
mod responses_api_proxy_headers;
#[path = "suite/rmcp_client.rs"]
mod rmcp_client;
#[path = "suite/stream_error_allows_next_turn.rs"]
mod stream_error_allows_next_turn;
#[path = "suite/stream_no_completed.rs"]
mod stream_no_completed;
#[path = "suite/websocket_fallback.rs"]
mod websocket_fallback;
