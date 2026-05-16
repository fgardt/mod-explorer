use std::path::PathBuf;

use serde::Deserialize;
use url::Url;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub oauth2: OAuth2Config,
    pub route: RouteConfig,
    pub session: SessionConfig,
}

#[derive(Clone, Deserialize)]
pub struct OAuth2Config {
    pub auth_url: Url,
    pub token_url: Url,

    pub scopes: Box<[String]>,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Clone, Deserialize)]
pub struct RouteConfig {
    pub base_url: Url,
    pub prefix: String,

    #[serde(skip)]
    pub protected_prefixes: Box<[String]>,
}

#[derive(Clone, Deserialize)]
pub struct SessionConfig {
    pub storage_path: Option<PathBuf>,
    pub cookie_name: String,
    #[serde(default)]
    pub cookie_insecure: bool,
    pub timeout_seconds: u64,
}

impl Config {
    pub fn get_redirect_url(&self) -> Url {
        let route = &self.route;
        route
            .base_url
            .join(&format!("{}/callback", route.prefix))
            .expect("Incorrectly configured auth routing")
    }
}
