use axum::extract::{Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::Json;

use crate::handlers::err;
use crate::AppState;

/// GET /api/events?source=core&case_id=...&limit=100
pub async fn query_events(
    State(state): State<AppState>,
    Query(filter): Query<dm_core::events::EventFilter>,
) -> impl IntoResponse {
    match state.events.query(&filter) {
        Ok(events) => Json(events).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/events/count?source=core&case_id=...
pub async fn count_events(
    State(state): State<AppState>,
    Query(filter): Query<dm_core::events::EventFilter>,
) -> impl IntoResponse {
    match state.events.count(&filter) {
        Ok(c) => Json(serde_json::json!({ "count": c })).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/events
pub async fn ingest_event(
    State(state): State<AppState>,
    Json(event): Json<dm_core::events::Event>,
) -> impl IntoResponse {
    match state.events.emit(&event) {
        Ok(id) => Json(serde_json::json!({ "id": id })).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/events/export?source=dataflow&format=xes
pub async fn export_events(
    State(state): State<AppState>,
    Query(filter): Query<dm_core::events::EventFilter>,
) -> impl IntoResponse {
    match state.events.export_xes(&filter) {
        Ok(xes) => ([(CONTENT_TYPE, "application/xml")], xes).into_response(),
        Err(e) => err(e).into_response(),
    }
}
