use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn EmptyFileViewer() -> impl IntoView {
    view! {
        <div class="fileviewer empty">
            <p>"Select a file to view its contents"</p>
        </div>
    }
}

#[component]
pub fn FileViewer() -> impl IntoView {
    let params = use_params_map();
    let name = move || params.read().get("name").unwrap_or_default();
    let version = move || params.read().get("version").unwrap_or("latest".into());
    let file_path = move || params.read().get("file_path").unwrap_or_default();

    let file_content = Resource::new(
        move || (name(), version(), file_path()),
        async |(name, version, file_path)| fetch_file(name, version, file_path).await,
    );

    let (pending, set_pending) = RwSignal::new(false).split();

    view! {
        <div class="fileviewer">
            <Transition set_pending=set_pending fallback=move || view! {
                <p class="filename">{move || file_path()}</p>
                <textarea class="loading" readonly=true></textarea>
            }>
                <p class="filename">{move || file_path()}</p>
                <div class="content" class:pending=pending>
                {move || {
                    file_content.get().map(|res| match res {
                        Ok(content) => view! {
                            {content}
                        }.into_any(),
                        Err(ServerFnError::ServerError(msg)) => msg.into_view().into_any(),
                        Err(e) => format!("{e}").into_view().into_any(),
                    })
                }}
                </div>
            </Transition>
        </div>
    }
}

#[server(prefix = "/api/sec")]
pub async fn fetch_file(
    name: String,
    version: String,
    path: String,
) -> Result<String, ServerFnError> {
    use std::path::{Component, PathBuf};

    let Some(state) = use_context::<crate::state::AppState>() else {
        return Err(ServerFnError::ServerError("Missing state".into()));
    };

    let Some(version) = state.mods_state.get_version(&name, &version).await else {
        return Err(ServerFnError::ServerError(
            "Mod or version not found".into(),
        ));
    };

    let path = PathBuf::from(path);
    if path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(ServerFnError::ServerError("Invalid path".into()));
    }

    let path = state
        .mods_folder
        .join(format!("{name}_{version}"))
        .join(path);
    if !path.exists() || !path.is_file() {
        return Err(ServerFnError::ServerError("File not found".into()));
    }

    if let Some(guess) = mime_guess::from_path(&path).first() {
        println!("Guessed MIME type {guess} for file {}", path.display());

        match (guess.type_().as_str(), guess.subtype().as_str()) {
            ("text", _) | ("application", "json") => {} // text / json files are fine
            _ => {
                return Err(ServerFnError::ServerError(
                    "Only text files can be viewed".into(),
                ));
            }
        }
    };

    match tokio::fs::read_to_string(path).await {
        Ok(c) => Ok(c),
        Err(e) => Err(ServerFnError::ServerError(format!(
            "Failed to read file: {e}"
        ))),
    }
}
