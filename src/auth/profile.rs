use oauth2::{AccessToken, reqwest};
use serde::{Deserialize, Serialize};
use url::Url;

use super::session::{ModInfo, SessionPublic};

#[derive(Debug)]
pub enum ProfileFetchError {
    RequestFailed,
    ParseFailed,
}

impl std::error::Error for ProfileFetchError {}

impl std::fmt::Display for ProfileFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestFailed => write!(f, "Failed to fetch profile: HTTP request failed"),
            Self::ParseFailed => write!(f, "Failed to parse profile response"),
        }
    }
}

pub trait FetchProfile: Sized + for<'de> Deserialize<'de> {
    const ENDPOINT: &'static str;

    async fn fetch(
        token: &AccessToken,
        client: &reqwest::Client,
    ) -> Result<Self, ProfileFetchError> {
        let res = client
            .get(Self::ENDPOINT)
            .bearer_auth(token.secret())
            .send();

        let res = match res.await {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Failed to send profile request: {e}");
                return Err(ProfileFetchError::RequestFailed);
            }
        };

        if !res.status().is_success() {
            eprintln!(
                "Failed to fetch profile: HTTP {} - {}",
                res.status(),
                res.text().await.unwrap_or_default()
            );
            return Err(ProfileFetchError::RequestFailed);
        }

        let bytes = match res.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Failed to read profile response: {e}");
                return Err(ProfileFetchError::RequestFailed);
            }
        };

        serde_json::from_slice(&bytes).map_err(|e| {
            eprintln!("Failed to parse profile response: {e}");
            ProfileFetchError::ParseFailed
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MainProfile {
    pub account_created_at: String,
    pub account_upgraded_at: Option<String>,
    pub game_ownership: bool,
    pub id: String,
    pub memberships: Box<[String]>,
    pub steam_id_connected: bool,
    pub username: String,
}

impl FetchProfile for MainProfile {
    const ENDPOINT: &'static str = "https://factorio.com/api/profile";
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModPortalProfile {
    pub avatar_url: Url,
    pub id: String,
    pub mods: Box<[ModInfo]>,
    pub mods_collaborated: Box<[ModInfo]>,
    pub username: String,
}

impl FetchProfile for ModPortalProfile {
    const ENDPOINT: &'static str = "https://mods.factorio.com/api/profile";
}

impl From<ModPortalProfile> for SessionPublic {
    fn from(value: ModPortalProfile) -> Self {
        Self {
            username: value.username,
            avatar_url: value.avatar_url.to_string(),
            mods: value.mods,
            mods_collaborated: value.mods_collaborated,
        }
    }
}
