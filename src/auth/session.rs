use leptos::{prelude::ServerFnError, server};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use super::provider::Token;

#[cfg(feature = "ssr")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub token: Token,

    pub public_data: SessionPublic,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub downloads_count: u32,
    pub id: String,
    pub name: String,
    pub title: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionPublic {
    pub username: String,
    pub avatar_url: String,

    pub mods: Box<[ModInfo]>,
    pub mods_collaborated: Box<[ModInfo]>,
}

#[cfg(feature = "ssr")]
impl<S> axum::extract::FromRequestParts<S> for SessionPublic
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or(())
    }
}

#[server(prefix = "/auth", endpoint = "session")]
pub async fn get_session_data() -> Result<Option<SessionPublic>, ServerFnError> {
    leptos_axum::extract::<SessionPublic>()
        .await
        .map(Some)
        .or(Ok(None))
}
