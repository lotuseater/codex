//! App-server-facing analytics tracking, layered on top of the protocol-free
//! [`codex_analytics::AnalyticsEventsClient`].
//!
//! These methods used to live as inherent methods on `AnalyticsEventsClient` in
//! `codex-analytics`, but they consume `codex_app_server_protocol` types, so
//! they moved here. `codex-app-server` brings them into scope with
//! `use codex_analytics_appserver::AppServerAnalyticsExt;` and the existing call
//! sites compile unchanged.

use crate::rpc_fact::AppServerFact;
use codex_analytics::AnalyticsEventsClient;
use codex_analytics::AnalyticsFact;
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

fn record(client: &AnalyticsEventsClient, fact: AppServerFact) {
    let value = serde_json::to_value(&fact).unwrap_or(serde_json::Value::Null);
    client.record_fact(AnalyticsFact::AppServer(value));
}

/// App-server RPC analytics tracking for [`AnalyticsEventsClient`].
pub trait AppServerAnalyticsExt {
    fn track_initialize(
        &self,
        connection_id: u64,
        params: InitializeParams,
        product_client_id: String,
        rpc_transport: AppServerRpcTransport,
    );
    fn track_request(&self, connection_id: u64, request_id: RequestId, request: &ClientRequest);
    fn track_response(
        &self,
        connection_id: u64,
        request_id: RequestId,
        response: ClientResponsePayload,
    );
    fn track_error_response(
        &self,
        connection_id: u64,
        request_id: RequestId,
        error: JSONRPCErrorError,
        error_type: Option<AnalyticsJsonRpcError>,
    );
    fn track_server_request(&self, connection_id: u64, request: ServerRequest);
    fn track_server_response(&self, completed_at_ms: u64, response: ServerResponse);
    fn track_effective_permissions_approval_response(
        &self,
        completed_at_ms: u64,
        request_id: RequestId,
        response: RequestPermissionsResponse,
    );
    fn track_server_request_aborted(&self, completed_at_ms: u64, request_id: RequestId);
    fn track_notification(&self, notification: ServerNotification);
}

impl AppServerAnalyticsExt for AnalyticsEventsClient {
    fn track_initialize(
        &self,
        connection_id: u64,
        params: InitializeParams,
        product_client_id: String,
        rpc_transport: AppServerRpcTransport,
    ) {
        record(
            self,
            AppServerFact::Initialize {
                connection_id,
                params,
                product_client_id,
                rpc_transport,
            },
        );
    }

    fn track_request(&self, connection_id: u64, request_id: RequestId, request: &ClientRequest) {
        if !matches!(
            request,
            ClientRequest::TurnStart { .. } | ClientRequest::TurnSteer { .. }
        ) {
            return;
        }
        record(
            self,
            AppServerFact::ClientRequest {
                connection_id,
                request_id,
                request: Box::new(request.clone()),
            },
        );
    }

    fn track_response(
        &self,
        connection_id: u64,
        request_id: RequestId,
        response: ClientResponsePayload,
    ) {
        if !matches!(
            response,
            ClientResponsePayload::ThreadStart(_)
                | ClientResponsePayload::ThreadResume(_)
                | ClientResponsePayload::ThreadFork(_)
                | ClientResponsePayload::TurnStart(_)
                | ClientResponsePayload::TurnSteer(_)
        ) {
            return;
        }
        record(
            self,
            AppServerFact::ClientResponse {
                connection_id,
                request_id,
                response: Box::new(response),
            },
        );
    }

    fn track_error_response(
        &self,
        connection_id: u64,
        request_id: RequestId,
        error: JSONRPCErrorError,
        error_type: Option<AnalyticsJsonRpcError>,
    ) {
        record(
            self,
            AppServerFact::ErrorResponse {
                connection_id,
                request_id,
                error,
                error_type,
            },
        );
    }

    fn track_server_request(&self, connection_id: u64, request: ServerRequest) {
        record(
            self,
            AppServerFact::ServerRequest {
                connection_id,
                request: Box::new(request),
            },
        );
    }

    fn track_server_response(&self, completed_at_ms: u64, response: ServerResponse) {
        record(
            self,
            AppServerFact::ServerResponse {
                completed_at_ms,
                response: Box::new(response),
            },
        );
    }

    fn track_effective_permissions_approval_response(
        &self,
        completed_at_ms: u64,
        request_id: RequestId,
        response: RequestPermissionsResponse,
    ) {
        record(
            self,
            AppServerFact::EffectivePermissionsApprovalResponse {
                completed_at_ms,
                request_id,
                response: Box::new(response),
            },
        );
    }

    fn track_server_request_aborted(&self, completed_at_ms: u64, request_id: RequestId) {
        record(
            self,
            AppServerFact::ServerRequestAborted {
                completed_at_ms,
                request_id,
            },
        );
    }

    fn track_notification(&self, notification: ServerNotification) {
        if !matches!(
            notification,
            ServerNotification::TurnStarted(_)
                | ServerNotification::TurnCompleted(_)
                | ServerNotification::TurnDiffUpdated(_)
                | ServerNotification::ItemStarted(_)
                | ServerNotification::ItemCompleted(_)
                | ServerNotification::ItemGuardianApprovalReviewStarted(_)
                | ServerNotification::ItemGuardianApprovalReviewCompleted(_)
        ) {
            return;
        }
        record(self, AppServerFact::Notification(Box::new(notification)));
    }
}
