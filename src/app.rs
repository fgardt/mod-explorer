use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    MatchNestedRoutes, NavigateOptions, ParamSegment, StaticSegment, WildcardSegment,
    any_nested_route::IntoAnyNestedRoute,
    components::{Outlet, ParentRoute, Redirect, Route, Router, Routes},
    hooks::use_params_map,
};

use crate::components::GitHubCorner;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="format-detection" content="telephone=no"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/mod-explorer.css"/>
        <Title text="Factorio mod explorer"/>
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <ParentRoute path=StaticSegment("i") view=ModSelector>
                        <ExplorerRoutes/>
                        <Route path=StaticSegment("") view=RedirectToRoot/>
                    </ParentRoute>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Welcome to the Factorio mod explorer!"</h1>
        <GitHubCorner repo="fgardt/mod-explorer"/>
    }
}

#[component(transparent)]
fn ExplorerRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=ParamSegment("name") view=Outlet>
            <Route path=StaticSegment("") view=RedirectToLatest/>
            <ParentRoute path=ParamSegment("version") view=FileTree>
                <Route path=StaticSegment("") view=||"select file to view".into_view()/>
                <Route path=WildcardSegment("file_path") view=FileViewer/>
            </ParentRoute>
        </ParentRoute>
    }
    .into_inner()
    .into_any_nested_route()
}

#[component]
fn ModSelector() -> impl IntoView {
    let params = use_params_map();
    let name = move || params.read().get("name").unwrap_or_default();
    let version = move || params.read().get("version");

    view! {
        <h2>"Mod selector (" {move || name()} " - " {move || version().unwrap_or("unknown".into())} ")"</h2>
        <Outlet/>
    }
}

#[component]
fn FileTree() -> impl IntoView {
    view! {
        <h2>"File tree"</h2>
    }
}

#[component]
fn FileViewer() -> impl IntoView {
    let params = use_params_map();
    let file_path = move || params.read().get("file_path").unwrap_or_default();

    view! {
        <h2>"File Viewer"</h2>
        <p>"Viewing file: " {move || file_path()}</p>
    }
}

#[component]
fn RedirectToRoot() -> impl IntoView {
    let opts = NavigateOptions {
        replace: true,
        ..Default::default()
    };

    view! {
        <Redirect path="/" options=opts />
    }
}

#[component]
fn RedirectToLatest() -> impl IntoView {
    let opts = NavigateOptions {
        replace: true,
        ..Default::default()
    };

    view! {
        <Redirect path="latest" options=opts />
    }
}
