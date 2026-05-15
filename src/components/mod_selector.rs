use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

#[component]
pub fn ModSelectorData() -> impl IntoView {
    let mods = Resource::new(|| (), async |()| get_available_mods().await);

    view! {
        <datalist id="mods">
            {move || {
                Suspend::new(async move {
                    let Ok(mods) = mods.await else {
                        return ().into_any();
                    };

                    mods.iter().map(|name| {
                        view! {
                            <option value={name.clone()}>{name.clone()}</option>
                        }
                    }).collect_view().into_any()
                })
            }}
        </datalist>
    }
}

#[component]
pub fn ModSelector() -> impl IntoView {
    let params = use_params_map();
    let name = move || params.read().get("name").unwrap_or_default();
    let version = move || params.read().get("version");

    let available_versions = Resource::new(name, async move |name| {
        if name.is_empty() {
            return None;
        }

        get_mod_versions(name).await.ok().map(|versions| {
            let latest = versions.first().cloned().unwrap_or_default();
            (latest, versions)
        })
    });

    let name_ref: NodeRef<html::Input> = NodeRef::new();
    let (selected_version, set_selected_version) = RwSignal::new(None).split();

    let trigger_nav = move || {
        let name = name_ref
            .get()
            .expect("name <input> should be mounted")
            .value();
        let version = selected_version.get().unwrap_or("latest".into());

        if !name.is_empty() {
            let navigate = use_navigate();
            navigate(&format!("/i/{name}/{version}"), Default::default());
        }
    };

    view! {
        <form on:submit=move |ev| {
            ev.prevent_default();
            trigger_nav();
        }>
            <input type="text" list="mods" node_ref=name_ref placeholder="Search for mod" value=move ||{name()}/>
            {move || {
                if let Some(version) = version() {
                    view! {
                        <select prop:value=version on:change:target=move |ev| {
                            set_selected_version.set(Some(ev.target().value()));
                            trigger_nav();
                        }>
                            <Suspense fallback=|| view! {
                                <option value="latest">"latest (?.?.?)"</option>
                            }>
                                {move || {

                                    match available_versions.get().flatten() {
                                        Some((latest, all_versions)) => {

                                            std::iter::once(view! {
                                                <option value="latest">"latest ("{latest.clone()}")"</option>
                                            }.into_any())
                                            .chain(all_versions.iter().map(|v| {
                                                view! {
                                                    <option value={v.clone()}>{v.clone()}</option>
                                                }.into_any()
                                            })).collect_view().into_any()
                                        }
                                        None => {
                                            view! {
                                                <option value="latest">"latest (?.?.?)"</option>
                                            }.into_any()
                                        }
                                    }
                                }}
                            </Suspense>
                        </select>
                    }.into_any()
                } else {
                    ().into_view().into_any()
                }
            }}
        </form>
    }
}

#[server]
async fn get_available_mods() -> Result<Box<[String]>, ServerFnError> {
    let Some(state) = use_context::<crate::state::AppState>() else {
        return Err(ServerFnError::ServerError("Missing state".into()));
    };

    let state = state.mods_state;

    let mods = state
        .all_mods
        .read()
        .await
        .iter()
        .map(|s| s.to_string())
        .collect::<Box<_>>();

    Ok(mods)
}

#[server]
async fn get_mod_versions(name: String) -> Result<Box<[String]>, ServerFnError> {
    let Some(state) = use_context::<crate::state::AppState>() else {
        return Err(ServerFnError::ServerError("Missing state".into()));
    };

    let state = state.mods_state;

    let versions = state.mod_versions.read().await;
    let Some(versions) = versions.get(name.as_str()) else {
        return Err(ServerFnError::ServerError(format!(
            "Unknown mod \"{name}\""
        )));
    };

    let versions = versions.iter().map(|s| s.to_string()).collect::<Box<_>>();
    Ok(versions)
}
