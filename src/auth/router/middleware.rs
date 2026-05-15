use std::time::Duration;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, Uri, uri::PathAndQuery},
    response::{IntoResponse as _, Redirect, Response},
};
use tower_sessions::Session;

use super::{super::session::SessionData, AuthState};

pub async fn authentication_check(
    State(state): State<AuthState>,
    session: Session,
    mut request: Request,
    next: axum::middleware::Next,
) -> Response {
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

    let cookie_name = &state.cookie_name;
    let Some(session_data) = session.get::<SessionData>(cookie_name).await.ok().flatten() else {
        // no session, but protected

        // don't redirect server_fns
        // TODO: get prefix from leptos_options
        if path.starts_with("/api") {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Unauthorized"))
                .unwrap();
        }

        return redirect_to_login(&state.route_config.prefix, uri);
    };

    if session_data.token.is_expired() {
        session.remove::<SessionData>(cookie_name).await.ok();
        return redirect_to_login(&state.route_config.prefix, uri);
    }

    let public = session_data.public_data.clone();

    if session_data.token.should_refresh(Duration::from_mins(30)) {
        let mut token = session_data.token.clone();
        match state.provider.refresh_token(&mut token).await {
            Ok(()) => {
                let mut new_data = session_data;
                new_data.token = token;

                session.insert(cookie_name, new_data).await.ok();
            }
            Err(e) => {
                eprintln!("Failed to refresh token: {e}");
                session.remove::<SessionData>(cookie_name).await.ok();
                return redirect_to_login(&state.route_config.prefix, uri);
            }
        }
    }

    request.extensions_mut().insert(public);
    next.run(request).await
}

fn redirect_to_login(prefix: &str, current_uri: &Uri) -> Response {
    let next_param = current_uri
        .path_and_query()
        .map_or("/", PathAndQuery::as_str);
    let target = format!("{prefix}/login?next={}", urlencoding::encode(next_param));

    Redirect::temporary(&target).into_response()
}
