use std::time::Duration;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, Uri, uri::PathAndQuery},
    response::{IntoResponse as _, Redirect, Response},
};
use tower_sessions::Session;

use super::SESSION_KEY;

use super::{super::session::SessionData, AuthState};

pub async fn authentication_check(
    State(state): State<AuthState>,
    session: Session,
    mut request: Request,
    next: axum::middleware::Next,
) -> Response {
    let session_data = session.get::<SessionData>(SESSION_KEY).await.ok().flatten();
    if let Some(session_data) = &session_data {
        request
            .extensions_mut()
            .insert(session_data.public_data.clone());
    }

    let protected_prefixes = &state.route_config.protected_prefixes;
    let uri = request.uri();
    let path = uri.path();

    // check if uri is unprotected
    if !protected_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return next.run(request).await;
    }

    let Some(session_data) = session_data else {
        // no session, but protected

        // don't redirect server_fns
        // TODO: get prefix from leptos_options
        if path.starts_with("/api/") {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Unauthorized"))
                .unwrap();
        }

        return redirect_to_login(&state.route_config.prefix, uri);
    };

    if session_data.token.is_expired() {
        session.remove::<SessionData>(SESSION_KEY).await.ok();
        return redirect_to_login(&state.route_config.prefix, uri);
    }

    if session_data.token.should_refresh(Duration::from_mins(30)) {
        let mut token = session_data.token.clone();
        match state.provider.refresh_token(&mut token).await {
            Ok(()) => {
                let mut new_data = session_data;
                new_data.token = token;

                session.insert(SESSION_KEY, new_data).await.ok();
            }
            Err(_) => {
                session.remove::<SessionData>(SESSION_KEY).await.ok();
                return redirect_to_login(&state.route_config.prefix, uri);
            }
        }
    }

    next.run(request).await
}

fn redirect_to_login(prefix: &str, current_uri: &Uri) -> Response {
    let next_param = current_uri
        .path_and_query()
        .map_or("/", PathAndQuery::as_str);
    let target = format!("{prefix}/login?next={}", urlencoding::encode(next_param));

    Redirect::temporary(&target).into_response()
}
