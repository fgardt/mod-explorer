use std::time::Duration;

use super::AuthConfig;
use super::config::RouteConfig;
use super::provider::Provider;

mod endpoints;
mod middleware;

const SESSION_KEY: &str = "auth";

#[derive(Clone)]
pub struct AuthState {
    provider: Provider,

    route_config: RouteConfig,
}

impl AuthState {
    pub fn new(config: &AuthConfig) -> Self {
        let redirect_url = config.get_redirect_url();
        let provider = Provider::new(config.oauth2.clone(), redirect_url);

        Self {
            provider,
            route_config: config.route.clone(),
        }
    }
}

pub trait AuthRoutes {
    fn use_factorio_auth(self, config: AuthConfig) -> axum::Router;
}

impl AuthRoutes for axum::Router {
    fn use_factorio_auth(self, config: AuthConfig) -> axum::Router {
        use tower_sessions::cookie::{SameSite, time::Duration as CookieTimer};
        use tower_sessions::{ExpiredDeletion, Expiry};
        use tower_sessions_file_store::FileSessionStorage;

        let state = AuthState::new(&config);

        let auth_router = axum::Router::new()
            .nest(
                &config.route.prefix,
                axum::Router::new()
                    .route("/login", axum::routing::get(endpoints::login))
                    .route("/callback", axum::routing::get(endpoints::callback)),
            )
            .with_state(state.clone());

        let extended = self.layer(axum::middleware::from_fn_with_state(
            state,
            middleware::authentication_check,
        ));

        let session_store = if let Some(path) = config.session.storage_path {
            FileSessionStorage::new_in_folder(path)
        } else {
            FileSessionStorage::new()
        }
        .set_minimum_expiry_date(Duration::from_secs(config.session.timeout_seconds));

        tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(Duration::from_secs(60)),
        );

        let age = config.session.timeout_seconds + (config.session.timeout_seconds >> 2);
        let cookie_timer = CookieTimer::seconds(age as i64);
        let session_manager = tower_sessions::SessionManagerLayer::new(session_store)
            .with_name(config.session.cookie_name)
            .with_secure(!config.session.cookie_insecure)
            .with_same_site(SameSite::Lax)
            .with_expiry(Expiry::OnInactivity(cookie_timer));

        Self::new()
            .merge(auth_router)
            .merge(extended)
            .layer(session_manager)
    }
}
