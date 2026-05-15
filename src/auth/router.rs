use tower_sessions_cookie_store::{
    CookieSessionConfig, CookieSessionManagerLayer, Key, SameSite, SignedCookie,
};

use super::config::RouteConfig;

use super::AuthConfig;
use super::provider::Provider;

mod endpoints;
mod middleware;

#[derive(Clone)]
pub struct AuthState {
    provider: Provider,

    route_config: RouteConfig,
    cookie_name: String,
}

impl AuthState {
    pub fn new(config: &AuthConfig) -> Self {
        let redirect_url = config.get_redirect_url();
        let provider = Provider::new(config.oauth2.clone(), redirect_url);

        Self {
            provider,
            route_config: config.route.clone(),
            cookie_name: config.session.cookie_name.clone(),
        }
    }
}

pub trait AuthRoutes {
    fn use_factorio_auth(self, config: AuthConfig) -> axum::Router;
}

impl AuthRoutes for axum::Router {
    fn use_factorio_auth(self, config: AuthConfig) -> axum::Router {
        let state = AuthState::new(&config);

        let cookies = create_cookie_layer(config.session.cookie_name, config.session.cookie_secret);

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

        Self::new()
            .merge(auth_router)
            .merge(extended)
            .layer(cookies)
    }
}

fn create_cookie_layer(name: String, secret: String) -> CookieSessionManagerLayer<SignedCookie> {
    let config = CookieSessionConfig::default()
        // .with_secure(!cfg!(debug_assertions))
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_name(name);
    let key = Key::from(secret.as_bytes());
    CookieSessionManagerLayer::signed(key).with_config(config)
}
