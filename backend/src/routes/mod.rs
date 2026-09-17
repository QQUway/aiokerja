use axum::Router;

use crate::state::AppState;

pub mod chat;
pub mod dashboard;
pub mod documents;
pub mod events;
pub mod health;
pub mod products;
pub mod projects;
pub mod search;
pub mod tags;
pub mod tasks;

/// Root router: `/health` at the top level (for Docker healthchecks) and every
/// other route under `/api`.
pub fn api_router() -> Router<AppState> {
    let api = Router::new()
        .merge(health::router())
        .merge(tasks::router())
        .merge(projects::router())
        .merge(tags::router())
        .merge(events::router())
        .merge(documents::router())
        .merge(chat::router())
        .merge(search::router())
        .merge(dashboard::router())
        .merge(products::router());

    Router::new().merge(health::router()).nest("/api", api)
}
