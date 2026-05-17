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
            <ThemeSelector />
            <Transition set_pending=set_pending fallback=move || view! {
                <p class="filename">{move || file_path()}</p>
                <div class="content pending"></div>
            }>
                <p class="filename">{move || file_path()}</p>
                {move || {
                    file_content.get().map(|res| match res {
                        Ok(content) => view! {
                            <div class="content" class:pending=pending inner_html=content />
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

pub const DEFAULT_THEME: &str = "base16-eighties.dark";

#[component]
fn ThemeSelector() -> impl IntoView {
    let (selected_theme, set_selected_theme) = RwSignal::new(DEFAULT_THEME.to_string()).split();
    let themes = Resource::new(|| (), async |_| theme_list().await);
    let theme_css = Resource::new(
        move || selected_theme.get(),
        async |theme| get_theme(theme).await,
    );

    view! {
        <div class="theme-selector">
            <select prop:value=selected_theme on:change:target=move |ev| {
                set_selected_theme.set(ev.target().value());
            }>
                <Suspense fallback=view! {
                    <option value=DEFAULT_THEME>{DEFAULT_THEME}</option>
                }>
                    {move || {
                        themes.get().map(|res| match res {
                            Err(_) => view! {
                                <option value=DEFAULT_THEME>{DEFAULT_THEME}</option>
                            }.into_any(),
                            Ok(t) => t.iter().map(|t| {
                                view! {
                                    <option value={t.clone()}>{t.clone()}</option>
                                }
                            }).collect_view().into_any(),
                        })
                    }}
                </Suspense>
            </select>
            <Transition>
                {move || {
                    theme_css.get().and_then(Result::ok).map(|css| {
                        view! {
                            <style inner_html=css />
                        }
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
) -> Result<String, ServerFnError> {
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

    let mut generator =
        ClassedHTMLGenerator::new_with_class_style(syntax, syntax_set, ClassStyle::Spaced);

    for line in LinesWithEndings::from(&content) {
        if let Err(e) = generator.parse_html_for_line_which_includes_newline(line) {
            return Err(ServerFnError::ServerError(format!(
                "Failed to parse file for syntax highlighting: {e}"
            )));
        }
    }

    Ok(generator.finalize())
}

#[server]
pub async fn theme_list() -> Result<Vec<String>, ServerFnError> {
    let Some(state) = use_context::<crate::state::AppState>() else {
        return Err(ServerFnError::ServerError("Missing state".into()));
    };

    Ok(state.highlighter.theme_set.themes.keys().cloned().collect())
}

#[server]
pub async fn get_theme(name: String) -> Result<String, ServerFnError> {
    use syntect::html::{ClassStyle, css_for_theme_with_class_style};

    let Some(state) = use_context::<crate::state::AppState>() else {
        return Err(ServerFnError::ServerError("Missing state".into()));
    };

    let all_themes = &state.highlighter.theme_set.themes;
    let theme = all_themes
        .get(&name)
        .unwrap_or_else(|| &all_themes[DEFAULT_THEME]);

    css_for_theme_with_class_style(theme, ClassStyle::Spaced)
        .map_err(|e| ServerFnError::ServerError(format!("Failed to generate CSS for theme: {e}")))
}
