use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    MatchNestedRoutes, NavigateOptions, ParamSegment, StaticSegment, WildcardSegment,
    any_nested_route::IntoAnyNestedRoute,
    components::{Outlet, ParentRoute, ProtectedParentRoute, Redirect, Route, Router, Routes},
    hooks::use_location,
};

use crate::{auth, components};

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

    let session = Resource::new(|| {}, async |_| auth::session::get_session_data().await);

    let auth_check = move || {
        session
            .get()
            .and_then(|s| s.ok())
            .map(|session| match session {
                Some(session) => {
                    provide_context(session);
                    true
                }
                None => false,
            })
    };
    let redirect_path = || {
        let current = use_location();
        let path = current.pathname.read_untracked();
        let query = current.search.read_untracked();
        let hash = current.hash.read_untracked();

        let mut target = path.to_string();
        if !query.is_empty() {
            target.push('?');
            target.push_str(&query);
        }

        if !hash.is_empty() {
            if !hash.starts_with('#') {
                target.push('#');
            }
            target.push_str(&hash);
        }

        let target = urlencoding::encode(&target);
        let target = format!("/auth/login?next={target}");

        // this is hacky but it forces an actual reload
        // instead of leptos client-side routing
        // since redirect_path only gets called when
        // it is actually required to redirect
        let instant_target = target.clone();
        Effect::new(move || {
            let loc = location();
            let proto = loc.protocol().unwrap();
            let host = loc.host().unwrap();

            let href = format!("{proto}//{host}{instant_target}");
            loc.set_href(&href).unwrap();
        });

        target
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/mod-explorer.css"/>
        <ModSelectorData/>
        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <ProtectedParentRoute path=StaticSegment("i") view=ModSelectorWithOutlet condition=auth_check redirect_path=redirect_path>
                    <ExplorerRoutes/>
                    <Route path=StaticSegment("") view=RedirectToRoot/>
                </ProtectedParentRoute>
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
