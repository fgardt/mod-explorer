use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, MetaTags, Title, provide_meta_context};
use leptos_router::components::{
    Outlet, ParentRoute, ProtectedParentRoute, Redirect, Route, Router, Routes,
};
use leptos_router::hooks::use_location;
use leptos_router::{
    MatchNestedRoutes, NavigateOptions, ParamSegment, StaticSegment, WildcardSegment,
    any_nested_route::IntoAnyNestedRoute,
};
use reactive_stores::Store;

use crate::{auth, components};

use components::{Attribution, EmptyFileViewer, FileTree, FileViewer};
use components::{Bookmarklet, GitHubCorner};
use components::{ModSelector, ModSelectorData};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="format-detection" content="telephone=no"/>
                <HashedStylesheet id="main" options=options.clone() />
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

#[derive(Clone, Default, Store)]
struct GlobalState {
    session: Option<auth::session::SessionPublic>,
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_context(Store::new(GlobalState::default()));

    let session = Resource::new(
        || {},
        async |_| {
            let session = auth::session::get_session_data().await.ok().flatten();

            let global =
                use_context::<Store<GlobalState>>().expect("GlobalState context not found");
            global.session().set(session.clone());

            session
        },
    );

    let auth_check = move || session.get().as_ref().map(Option::is_some);
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
        <ModSelectorData/>
        <Router>
            <Routes fallback=||().into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <ProtectedParentRoute path=StaticSegment("mod") view=ModSelectorWithOutlet condition=auth_check redirect_path=redirect_path>
                    <ExplorerRoutes/>
                    <Route path=StaticSegment("") view=RedirectToRoot/>
                </ProtectedParentRoute>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let global = use_context::<Store<GlobalState>>().expect("GlobalState context not found");
    let session = global.session();

    view! {
        <Title text="Factorio mod explorer"/>
        <GitHubCorner repo="fgardt/mod-explorer"/>
        <ModSelectorWithOutlet/>
        <div class="welcome">
            <h1>
                "Welcome to the Factorio mod explorer"
                {move || {
                    session.clone().read().as_ref().map(|s| format!(", {}", s.username))
                }}
                "!"
            </h1>
            <Bookmarklet/>
        </div>
        <h3 class="how">"^^ To get started just search for any mod here"</h3>
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
        <FileTree/>
        <Attribution/>
        <Outlet/>
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
