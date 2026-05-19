use leptos::prelude::*;

pub const DEFAULT_THEME: &str = "base16-eighties.dark";

#[component]
pub fn ThemeSelector() -> impl IntoView {
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
                        let is_selected = |theme: String| theme == selected_theme.get();

                        themes.get().map(|res| match res {
                            Err(_) => view! {
                                <option value=DEFAULT_THEME>{DEFAULT_THEME}</option>
                            }.into_any(),
                            Ok(t) => t.iter().map(|t| {
                                view! {
                                    <option value={t.clone()} selected=is_selected(t.clone())>{t.clone()}</option>
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
