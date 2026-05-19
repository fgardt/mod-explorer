use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

mod line_numbers;
mod theme;

pub use theme::DEFAULT_THEME;

use line_numbers::LineNumbers;
use theme::ThemeSelector;

#[component]
pub fn EmptyFileViewer() -> impl IntoView {
    view! {
        <div class="fileviewer empty">
            <EmptyFileViewerInner />
        </div>
    }
}

#[component]
fn EmptyFileViewerInner() -> impl IntoView {
    view! {
        <div class="empty">
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
            <ThemeSelector />
            <Transition set_pending=set_pending fallback=move || view! {
                <p class="filename">{move || file_path()}</p>
                <div class="content pending"></div>
            }>
                <p class="filename">{move || file_path()}</p>
                {move || {
                    file_content.get().map(|res| match res {
                        Ok(Some((content, line_count))) => view! {
                            <div class="content" class:pending=pending>
                                <LineNumbers count=line_count />
                                <div class="file" inner_html=content />
                            </div>
                        }.into_any(),
                        Ok(None) => view! {
                            <EmptyFileViewerInner />
                        }.into_any(),
                        Err(e) => view! {
                            <div class="content" class:pending=pending>
                                {format!("Error fetching file: {e}")}
                            </div>
                        }.into_any(),
                    })
                }}
            </Transition>
        </div>
    }
}

#[server(prefix = "/api/sec")]
pub async fn fetch_file(
    name: String,
    version: String,
    path: String,
) -> Result<Option<(String, usize)>, ServerFnError> {
    use std::path::{Component, PathBuf};
    use syntect::html::{ClassStyle, ClassedHTMLGenerator};
    use syntect::util::LinesWithEndings;

    let Some(state) = use_context::<crate::state::AppState>() else {
        return Err(ServerFnError::ServerError("Missing state".into()));
    };

    let Some(version) = state.mods_state.get_version(&name, &version).await else {
        return Err(ServerFnError::ServerError(
            "Mod or version not found".into(),
        ));
    };

    if !state.mods_state.attribution_cache.can_access(&name).await {
        return Err(ServerFnError::ServerError(
            "Mod license does not allow exploring via this tool".into(),
        ));
    }

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
    if !path.exists() {
        return Err(ServerFnError::ServerError("File not found".into()));
    }

    if !path.is_file() {
        return Ok(None);
    }

    if let Some(guess) = mime_guess::from_path(&path).first() {
        // println!("Guessed MIME type: {guess} for file: {}", path.display());

        match (guess.type_().as_str(), guess.subtype().as_str()) {
            ("text", _) | ("application", _) => {} // text / json / script files are fine
            _ => {
                return Err(ServerFnError::ServerError(
                    "Only text files can be viewed".into(),
                ));
            }
        }
    };

    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(e) => {
            return Err(ServerFnError::ServerError(format!(
                "Failed to read file: {e}"
            )));
        }
    };

    let line_count = content.lines().count();

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    let syntax_set = &state.highlighter.syntax_set;

    let syntax = syntax_set
        .find_syntax_by_extension(extension)
        .or_else(|| {
            syntax_set.find_syntax_by_first_line(content.lines().next().unwrap_or_default())
        })
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut g = ClassedHTMLGenerator::new_with_class_style(syntax, syntax_set, ClassStyle::Spaced);

    for line in LinesWithEndings::from(&content) {
        if let Err(e) = g.parse_html_for_line_which_includes_newline(line) {
            return Err(ServerFnError::ServerError(format!(
                "Failed to parse file for syntax highlighting: {e}"
            )));
        }
    }

    Ok(Some((g.finalize(), line_count)))
}
