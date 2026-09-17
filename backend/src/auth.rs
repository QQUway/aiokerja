use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::AppError;

/// Fixed single user id (v1: no auth; see docs/decisions.md D6).
pub const LOCAL_USER_ID: uuid::Uuid =
    uuid::Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0001);

/// No-op auth placeholder so handlers already take a `CurrentUser`
/// and real multi-user auth can be added later without restructuring handlers.
#[derive(Debug, Clone, Copy)]
pub struct CurrentUser {
    /// Reserved for future multi-user auth (docs/decisions.md D6); handlers
    /// currently accept the user but do not yet partition data by it.
    #[allow(dead_code)]
    pub id: uuid::Uuid,
}

/// Implemented for any router state so the extractor works next to
/// `State<AppState>` in handlers.
#[async_trait::async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(CurrentUser { id: LOCAL_USER_ID })
    }
}
