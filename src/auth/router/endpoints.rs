use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
};
use tower_sessions::Session;

use super::{
    super::{
        profile::{FetchProfile as _, MainProfile, ModPortalProfile},
        session::SessionData,
    },
    AuthState,
};

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    next: Option<SafeNext>,
}

#[derive(Clone, Debug)]
pub struct SafeNext(pub String);

impl<'de> serde::Deserialize<'de> for SafeNext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let path = String::deserialize(deserializer)?;
        let safe = path.starts_with('/') && !path.starts_with("//");

        if safe {
            Ok(Self(path))
        } else {
            Ok(Self("/".to_string()))
        }
    }
}

pub async fn login(State(state): State<AuthState>, Query(query): Query<LoginQuery>) -> Redirect {
    let next = query.next.map(|n| n.0);
    let redirect_url = state.provider.start_flow(next).await;

    Redirect::temporary(redirect_url.as_str())
}

#[derive(Clone, serde::Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn callback(
    State(state): State<AuthState>,
    Query(query): Query<CallbackQuery>,
    session: Session,
) -> Result<Redirect, (StatusCode, String)> {
    let exchange = state.provider.complete_flow(query.code, query.state);
    let (token, destination) = match exchange.await {
        Ok(res) => res,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

    let main_req = MainProfile::fetch(&token.access_token, &state.provider.http_client);
    let main_profile = match main_req.await {
        Ok(profile) => profile,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

    if !main_profile.game_ownership {
        return Err((
            StatusCode::FORBIDDEN,
            "You do not have ownership of the game and thus also no access to the mod portal!"
                .to_string(),
        ));
    }

    let portal_req = ModPortalProfile::fetch(&token.access_token, &state.provider.http_client);
    let portal_profile = match portal_req.await {
        Ok(profile) => profile,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

    let session_data = SessionData {
        token,
        public_data: portal_profile.into(),
    };

    if let Err(e) = session.insert(&state.cookie_name, session_data).await {
        eprintln!("Failed to store session data: {e}");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create session".to_string(),
        ));
    }

    let Some(target) = destination else {
        return Ok(Redirect::temporary("/"));
    };

    Ok(Redirect::temporary(&target))
}
