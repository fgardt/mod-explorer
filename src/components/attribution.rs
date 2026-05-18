use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};

#[component]
pub fn Attribution() -> impl IntoView {
    let params = use_params_map();
    let name = move || params.read().get("name");

    let attribution = Resource::new(name, async move |name| {
        if let Some(name) = name {
            get_attribution(name).await.ok()
        } else {
            None
        }
    });

    view! {
        <Suspense>
            {move || {
                let name = name().unwrap_or_default();
                attribution.get().flatten().map(|res| {
                    view! {
                        <div class="attribution">
                            <PortalLink path=format!("/mod/{name}")>
                                {res.title}
                            </PortalLink>
                            <p>"made by"</p>
                            <PortalLink path=format!("/user/{}", res.owner)>
                                {res.owner}
                            </PortalLink>
                            <p>"licensed under"</p>
                            <a href=res.license_url.clone() target="_blank">
                                {res.license_name}
                            </a>
                        </div>
                    }
                })
            }}
        </Suspense>
    }
}

#[component]
fn PortalLink<Chil>(path: String, children: TypedChildren<Chil>) -> impl IntoView
where
    Chil: IntoView + Send + 'static,
{
    const PORTAL_URL: &str = "https://mods.factorio.com";
    let children = children.into_inner()();

    view! {
        <a href=format!("{PORTAL_URL}{path}") target="_blank">{children}</a>
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct AttributionData {
    title: String,
    owner: String,
    license_name: String,
    license_url: String,
}

#[server]
async fn get_attribution(name: String) -> Result<AttributionData, ServerFnError> {
    let Some(state) = use_context::<crate::state::AppState>() else {
        return Err(ServerFnError::ServerError("Missing state".into()));
    };

    let Some(info) = state.mods_state.attribution_cache.get(&name).await else {
        return Err(ServerFnError::ServerError(
            "Failed to fetch attribution data".into(),
        ));
    };

    Ok(AttributionData {
        title: info.title,
        owner: info.owner.to_string(),
        license_name: info.license.title.to_string(),
        license_url: info.license.url.to_string(),
    })
}
