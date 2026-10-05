use crate::{
    models::{CandidateRequest, SearchResponse},
    search::{self, SearchError},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    candidate_limit: u64,
    requests: Arc<Semaphore>,
}

pub fn router(db: PgPool, candidate_limit: u64, service_token: String) -> Router {
    let authorization = Arc::new(format!("Bearer {service_token}"));
    Router::new()
        .route("/search", post(search_handler))
        .route_layer(middleware::from_fn_with_state(authorization, authenticate))
        // Liveness only: no search or data is exposed here.
        .route("/health", get(|| async { StatusCode::OK }))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .with_state(AppState {
            db,
            candidate_limit,
            requests: Arc::new(Semaphore::new(8)),
        })
}

async fn authenticate(
    State(expected): State<Arc<String>>,
    request: Request,
    next: Next,
) -> Response {
    let mut headers = request.headers().get_all(header::AUTHORIZATION).iter();
    let valid = headers
        .next()
        .is_some_and(|value| bool::from(value.as_bytes().ct_eq(expected.as_bytes())))
        && headers.next().is_none();

    if !valid {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }
    next.run(request).await
}

struct ApiError(StatusCode, &'static str);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

async fn search_handler(
    State(state): State<AppState>,
    Json(input): Json<CandidateRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    let permit = state.requests.clone().try_acquire_owned().map_err(|_| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "Search is busy; retry shortly",
        )
    })?;

    let outcome = tokio::time::timeout(
        Duration::from_secs(10),
        search::run(&state.db, input, state.candidate_limit, permit),
    )
    .await;

    match outcome {
        Ok(Ok(response)) => Ok(Json(response)),

        Ok(Err(SearchError::InvalidInput(message))) => {
            Err(ApiError(StatusCode::BAD_REQUEST, message))
        }

        Ok(Err(SearchError::Database(error))) => {
            eprintln!("Search database error: {error}");
            Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "Search temporarily unavailable",
            ))
        }

        Ok(Err(SearchError::Scoring(error))) => {
            eprintln!("Search scoring error: {error}");
            Err(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Search scoring failed",
            ))
        }

        Err(_) => Err(ApiError(StatusCode::GATEWAY_TIMEOUT, "Search timed out")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    fn app() -> Router {
        let db = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgresql://test:test@127.0.0.1/test")
            .unwrap();
        router(db, 10, "unit-test-token".to_owned())
    }

    #[tokio::test]
    async fn rejects_missing_wrong_and_duplicate_tokens_before_parsing_body() {
        for token in [
            None,
            Some("Bearer wrong-token"),
            Some("Basic unit-test-token"),
        ] {
            let mut request = Request::builder().method("POST").uri("/search");
            if let Some(token) = token {
                request = request.header(header::AUTHORIZATION, token);
            }
            let response = app()
                .oneshot(request.body(Body::from("invalid JSON")).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
        }
        let request = Request::builder()
            .method("POST")
            .uri("/search")
            .header(header::AUTHORIZATION, "Bearer unit-test-token")
            .header(header::AUTHORIZATION, "Bearer wrong-token")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app().oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn correct_token_runs_search() {
        let request = Request::builder()
            .method("POST")
            .uri("/search")
            .header(header::AUTHORIZATION, "Bearer unit-test-token")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"query":"","labels":[]}"#))
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["results"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn health_does_not_require_a_token() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
