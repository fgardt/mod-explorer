use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    MatchNestedRoutes, NavigateOptions, ParamSegment, StaticSegment, WildcardSegment,
    any_nested_route::IntoAnyNestedRoute,
    components::{Outlet, ParentRoute, Redirect, Route, Router, Routes},
};

use crate::components;

use components::GitHubCorner;
use components::{EmptyFileViewer, FileTree, FileViewer};
use components::{ModSelector, ModSelectorData};

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
        <ModSelectorData/>
        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <ParentRoute path=StaticSegment("i") view=ModSelectorWithOutlet>
                    <ExplorerRoutes/>
                    <Route path=StaticSegment("") view=RedirectToRoot/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <Title text="Factorio mod explorer"/>
        <h1>"Welcome to the Factorio mod explorer!"</h1>
        <GitHubCorner repo="fgardt/mod-explorer"/>
        <ModSelector/>
    }
}

#[component(transparent)]
fn ExplorerRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=ParamSegment("name") view=Outlet>
            <Route path=StaticSegment("") view=RedirectToLatest/>
            <ParentRoute path=ParamSegment("version") view=FileTreeWithOutlet>
                <Route path=StaticSegment("") view=EmptyFileViewer/>
                <Route path=WildcardSegment("file_path") view=FileViewer/>
            </ParentRoute>
        </ParentRoute>
    }
    .into_inner()
    .into_any_nested_route()
}

#[component]
fn ModSelectorWithOutlet() -> impl IntoView {
    view! {
        <div class="top-bar">
            <ModSelector/>
        </div>
        <Outlet/>
    }
}

#[component]
fn FileTreeWithOutlet() -> impl IntoView {
    view! {
        <div class="explorer">
            <FileTree/>
            <Outlet/>
        </div>
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
