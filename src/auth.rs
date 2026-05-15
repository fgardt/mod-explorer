#[cfg(feature = "ssr")]
mod config;

#[cfg(feature = "ssr")]
mod profile;

#[cfg(feature = "ssr")]
mod provider;

#[cfg(feature = "ssr")]
mod router;

pub mod session;

#[cfg(feature = "ssr")]
pub use config::Config as AuthConfig;
#[cfg(feature = "ssr")]
pub use router::AuthRoutes;
