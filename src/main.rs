#[cfg(feature = "ssr")]
use std::process::ExitCode;

#[cfg(feature = "ssr")]
use mod_explorer::state::AppState;

#[cfg(feature = "ssr")]
use mod_explorer::auth;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> ExitCode {
    use std::{net::SocketAddr, path::PathBuf};

    use auth::{AuthConfig, AuthRoutes as _};
    use axum::Router;
    use clap::Parser;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use mod_explorer::app::*;

    #[derive(Parser)]
    pub struct Cli {
        #[clap(long, default_value = "0.0.0.0:3000")]
        pub addr: SocketAddr,

        #[clap(long, default_value = "./mods")]
        pub mods: PathBuf,

        #[clap(long, default_value = "60")]
        pub scan_interval: u64,

        #[clap(long)]
        pub auth_config: Option<PathBuf>,
    }

    let cli = Cli::parse();

    let auth_config = if let Some(path) = cli.auth_config {
        let c = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Failed to read auth config file: {e}");
                return ExitCode::FAILURE;
            }
        };

        match toml::from_str::<AuthConfig>(&c) {
            Ok(config) => Some(config),
            Err(e) => {
                eprintln!("Failed to parse auth config file: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };

    let conf = get_configuration(None).unwrap();
    let mut leptos_options = conf.leptos_options;
    leptos_options.site_addr = cli.addr;

    let app_state = AppState::new(cli.mods);

    mod_folder_scanner(cli.scan_interval, app_state.clone());

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || provide_context(app_state.clone()),
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let app_service = match auth_config {
        None => app.into_make_service(),
        Some(mut config) => {
            // TODO: keeping this in sync by hand is annoying & error-prone, should be generated from route list
            let protected = vec!["/mod/".into(), "/api/sec/".into()].into_boxed_slice();
            config.route.protected_prefixes = protected;

            app.use_factorio_auth(config).into_make_service()
        }
    };

    log!("listening on http://{}", &cli.addr);
    let listener = tokio::net::TcpListener::bind(&cli.addr).await.unwrap();
    axum::serve(listener, app_service).await.unwrap();

    ExitCode::SUCCESS
}

#[cfg(feature = "ssr")]
fn mod_folder_scanner(scan_interval: u64, state: AppState) {
    let mods_folder = state.mods_folder;
    let state = state.mods_state;

    tokio::spawn(async move {
        use std::collections::{BTreeSet, HashMap, HashSet};
        use std::time::Duration;
        use tokio::fs::read_dir;

        let mut ticker = tokio::time::interval(Duration::from_secs(scan_interval));

        loop {
            ticker.tick().await;

            let mut dirs = match read_dir(&mods_folder).await {
                Ok(rd) => rd,
                Err(e) => {
                    eprintln!("Failed to read mods folder: {}", e);
                    continue;
                }
            };

            let all_mods = state.all_mods.read().await.clone();
            let all_versions = state
                .mod_versions
                .read()
                .await
                .values()
                .flatten()
                .cloned()
                .collect::<HashSet<_>>();

            let mut new_all_mods = BTreeSet::new();
            let mut new_mod_versions = HashMap::<_, HashSet<_>>::new();

            while let Ok(Some(entry)) = dirs.next_entry().await {
                let Ok(kind) = entry.file_type().await else {
                    eprintln!(
                        "Failed to get file type for entry: {}",
                        entry.path().display()
                    );
                    continue;
                };

                if !kind.is_dir() {
                    continue;
                }

                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy();
                let mut parts = name.split('_');
                let version = parts.next_back().unwrap_or_default();
                let name = parts.collect::<Box<[_]>>().join("_");

                let name = all_mods
                    .get(name.as_str())
                    .cloned()
                    .unwrap_or_else(|| name.into());

                let version = all_versions
                    .get(version)
                    .cloned()
                    .unwrap_or_else(|| version.into());

                new_all_mods.insert(name.clone());
                new_mod_versions.entry(name).or_default().insert(version);
            }

            let new_mod_versions: HashMap<_, Box<_>> = new_mod_versions
                .into_iter()
                .map(|(k, v)| {
                    let mut v = v.into_iter().collect::<Vec<_>>();

                    v.sort_by(|a, b| {
                        fn parts(s: &str) -> [&str; 3] {
                            let mut parts = s.split('.');

                            let major = parts.next().unwrap_or_default();
                            let minor = parts.next().unwrap_or_default();
                            let patch = parts.next().unwrap_or_default();

                            [major, minor, patch]
                        }

                        let a_parts = parts(a);
                        let b_parts = parts(b);

                        a_parts[0]
                            .cmp(b_parts[0])
                            .then_with(|| a_parts[1].cmp(b_parts[1]))
                            .then_with(|| a_parts[2].cmp(b_parts[2]))
                    });
                    v.reverse();

                    (k, v.into_boxed_slice())
                })
                .collect();

            *state.all_mods.write().await = new_all_mods;
            *state.mod_versions.write().await = new_mod_versions;
        }
    });
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
