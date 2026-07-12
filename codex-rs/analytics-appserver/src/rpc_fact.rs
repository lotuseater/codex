//! The rich, app-server-protocol-typed fact enum.
//!
//! Facts are produced by [`crate::client_ext::AppServerAnalyticsExt`], serialized
//! to a `serde_json::Value`, and crossed over the protocol-free seam as
//! [`codex_analytics::AnalyticsFact::AppServer`]. The reducer in
//! [`crate::reducer`] deserializes them back into this enum.

use codex_analytics::AnalyticsJsonRpcError;
use codex_analytics::AppServerRpcTransport;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ClientResponsePayload;
use codex_app_server_protocol::InitializeParams;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ServerResponse;
use codex_protocol::request_permissions::RequestPermissionsResponse;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub(crate) enum AppServerFact {
    Initialize {
        connection_id: u64,
        params: InitializeParams,
        product_client_id: String,
        rpc_transport: AppServerRpcTransport,
    },
    ClientRequest {
        connection_id: u64,
        request_id: RequestId,
        request: Box<ClientRequest>,
    },
    ClientResponse {
        connection_id: u64,
        request_id: RequestId,
        #[serde(with = "client_response_payload_serde")]
        response: Box<ClientResponsePayload>,
        thread_originator: Option<String>,
    },
    ErrorResponse {
        connection_id: u64,
        request_id: RequestId,
        error: JSONRPCErrorError,
        error_type: Option<AnalyticsJsonRpcError>,
    },
    ServerRequest {
        connection_id: u64,
        request: Box<ServerRequest>,
    },
    ServerResponse {
        completed_at_ms: u64,
        response: Box<ServerResponse>,
    },
    EffectivePermissionsApprovalResponse {
        completed_at_ms: u64,
        request_id: RequestId,
        response: Box<RequestPermissionsResponse>,
    },
    ServerRequestAborted {
        completed_at_ms: u64,
        request_id: RequestId,
    },
    Notification(Box<ServerNotification>),
}

mod client_response_payload_serde {
    use super::ClientResponsePayload;
    use codex_app_server_protocol::ThreadForkResponse;
    use codex_app_server_protocol::ThreadResumeResponse;
    use codex_app_server_protocol::ThreadStartResponse;
    use codex_app_server_protocol::TurnStartResponse;
    use codex_app_server_protocol::TurnSteerResponse;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;

    #[derive(Serialize)]
    enum BorrowedClientResponsePayload<'a> {
        ThreadStart(&'a ThreadStartResponse),
        ThreadResume(&'a ThreadResumeResponse),
        ThreadFork(&'a ThreadForkResponse),
        TurnStart(&'a TurnStartResponse),
        TurnSteer(&'a TurnSteerResponse),
    }

    #[derive(Deserialize)]
    enum OwnedClientResponsePayload {
        ThreadStart(ThreadStartResponse),
        ThreadResume(ThreadResumeResponse),
        ThreadFork(ThreadForkResponse),
        TurnStart(TurnStartResponse),
        TurnSteer(TurnSteerResponse),
    }

    pub fn serialize<T, S>(response: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<ClientResponsePayload> + ?Sized,
        S: Serializer,
    {
        let serializable = match response.as_ref() {
            ClientResponsePayload::ThreadStart(response) => {
                BorrowedClientResponsePayload::ThreadStart(response)
            }
            ClientResponsePayload::ThreadResume(response) => {
                BorrowedClientResponsePayload::ThreadResume(response)
            }
            ClientResponsePayload::ThreadFork(response) => {
                BorrowedClientResponsePayload::ThreadFork(response)
            }
            ClientResponsePayload::TurnStart(response) => {
                BorrowedClientResponsePayload::TurnStart(response)
            }
            ClientResponsePayload::TurnSteer(response) => {
                BorrowedClientResponsePayload::TurnSteer(response)
            }
            _ => {
                return Err(<S::Error as serde::ser::Error>::custom(
                    "analytics appserver records only lifecycle client responses",
                ));
            }
        };

        serializable.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Box<ClientResponsePayload>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let response = OwnedClientResponsePayload::deserialize(deserializer)?;
        let response = match response {
            OwnedClientResponsePayload::ThreadStart(response) => {
                ClientResponsePayload::ThreadStart(response)
            }
            OwnedClientResponsePayload::ThreadResume(response) => {
                ClientResponsePayload::ThreadResume(response)
            }
            OwnedClientResponsePayload::ThreadFork(response) => {
                ClientResponsePayload::ThreadFork(response)
            }
            OwnedClientResponsePayload::TurnStart(response) => {
                ClientResponsePayload::TurnStart(response)
            }
            OwnedClientResponsePayload::TurnSteer(response) => {
                ClientResponsePayload::TurnSteer(response)
            }
        };

        Ok(Box::new(response))
    }
}
