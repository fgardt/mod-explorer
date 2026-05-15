use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use oauth2::{
    AccessToken, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet,
    EndpointSet, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope,
    TokenResponse, TokenUrl, basic::BasicClient, reqwest,
};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, time::interval};
use url::Url;

use super::config::OAuth2Config;

type Client = BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(Clone)]
pub struct Provider {
    client: Arc<Client>,
    scopes: Box<[Scope]>,

    pub(super) http_client: reqwest::Client,

    pending_flows: Arc<Mutex<HashMap<String, PendingFlow>>>,
}

struct PendingFlow {
    verifier: String,
    created_at: Instant,
    next: Option<String>,
}

#[derive(Clone, Debug)]
pub enum OauthError {
    InvalidState,
    TokenExchangeFailed,
    MissingRefreshToken,
    TokenRefreshFailed,
}

impl std::error::Error for OauthError {}

impl std::fmt::Display for OauthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidState => write!(f, "Invalid state parameter"),
            Self::TokenExchangeFailed => {
                write!(f, "Failed to exchange authorization code for access token")
            }
            Self::MissingRefreshToken => write!(f, "No refresh token available"),
            Self::TokenRefreshFailed => write!(f, "Failed to refresh access token"),
        }
    }
}

impl Provider {
    pub fn new(config: OAuth2Config, redirect_url: Url) -> Self {
        let client = BasicClient::new(ClientId::new(config.client_id))
            .set_client_secret(ClientSecret::new(config.client_secret))
            .set_auth_uri(AuthUrl::from_url(config.auth_url))
            .set_token_uri(TokenUrl::from_url(config.token_url))
            .set_redirect_uri(RedirectUrl::from_url(redirect_url));
        let client = Arc::new(client);

        let scopes = config.scopes.into_iter().map(Scope::new).collect();
        let http_client = oauth2::reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("working TLS backend for OAuth2 client is required");

        let pending_flows = Arc::new(Mutex::new(HashMap::<_, PendingFlow>::new()));
        let pf_cleanup = pending_flows.clone();

        // cleanup task to remove expired pending callbacks
        tokio::spawn(async move {
            const CALLBACK_EXPIRATION: Duration = Duration::from_mins(15);
            let mut ticker = interval(Duration::from_secs(60));

            loop {
                ticker.tick().await;
                let mut pending = pf_cleanup.lock().await;
                let now = Instant::now();
                pending.retain(|_, p| {
                    let elapsed = now.duration_since(p.created_at);

                    elapsed < CALLBACK_EXPIRATION && elapsed != Duration::ZERO
                });
            }
        });

        Self {
            client,
            scopes,
            http_client,
            pending_flows,
        }
    }

    /// - `destination`: Optional URL to redirect the user to after successful authentication
    pub async fn start_flow(&self, next: Option<String>) -> Url {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(self.scopes.clone())
            .set_pkce_challenge(challenge)
            .url();

        let mut pending = self.pending_flows.lock().await;
        pending.insert(
            csrf_token.secret().clone(),
            PendingFlow {
                verifier: verifier.secret().clone(),
                created_at: Instant::now(),
                next,
            },
        );

        auth_url
    }

    pub async fn complete_flow(
        &self,
        code: String,
        state: String,
    ) -> Result<(Token, Option<String>), OauthError> {
        let exchange = {
            let mut pending = self.pending_flows.lock().await;
            let Some(exchange) = pending.remove(&state) else {
                return Err(OauthError::InvalidState);
            };
            exchange
        };

        let token_req = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(PkceCodeVerifier::new(exchange.verifier))
            .request_async(&self.http_client);

        let token = match token_req.await {
            Ok(token) => token,
            Err(e) => {
                eprintln!("Failed to exchange code for token: {e}");
                return Err(OauthError::TokenExchangeFailed);
            }
        };

        let token = Token {
            access_token: token.access_token().clone(),
            refresh_token: token.refresh_token().cloned(),
            expires_in: token.expires_in(),
            fetched_at: SystemTime::now(),
        };

        Ok((token, exchange.next))
    }

    pub async fn refresh_token(&self, token: &mut Token) -> Result<(), OauthError> {
        let Some(refresh_token) = &token.refresh_token else {
            return Err(OauthError::MissingRefreshToken);
        };

        let refresh_req = self
            .client
            .exchange_refresh_token(refresh_token)
            .request_async(&self.http_client);

        let refreshed = match refresh_req.await {
            Ok(token) => token,
            Err(e) => {
                eprintln!("Failed to refresh access token: {e}");
                return Err(OauthError::TokenRefreshFailed);
            }
        };

        token.fetched_at = SystemTime::now();
        token.expires_in = refreshed.expires_in();

        token.access_token = refreshed.access_token().clone();
        if let Some(new_refresh_token) = refreshed.refresh_token() {
            token.refresh_token = Some(new_refresh_token.clone());
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Token {
    pub access_token: AccessToken,
    pub refresh_token: Option<RefreshToken>,
    pub expires_in: Option<Duration>,
    pub fetched_at: SystemTime,
}

impl Token {
    fn age(&self) -> Duration {
        self.fetched_at.elapsed().unwrap_or(Duration::MAX)
    }

    pub fn should_refresh(&self, refresh_window: Duration) -> bool {
        let Some(expires_in) = self.expires_in else {
            return false;
        };

        let elapsed = self.age().saturating_add(refresh_window);
        elapsed >= expires_in
    }

    pub fn is_expired(&self) -> bool {
        let Some(expires_in) = self.expires_in else {
            return false;
        };

        self.age() >= expires_in
    }
}
