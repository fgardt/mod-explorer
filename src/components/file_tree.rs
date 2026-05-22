use leptos::{ev::MouseEvent, prelude::*};
use leptos_meta::Title;
use leptos_router::{
    components::A,
    hooks::{use_location, use_params_map},
};
use serde::{Deserialize, Serialize};

#[component]
pub fn FileTree() -> impl IntoView {
    let params = use_params_map();
    let name = move || params.read().get("name").unwrap_or_default();
    let version = move || params.read().get("version").unwrap_or("latest".into());
    let path = move || params.read().get("file_path").unwrap_or_default();

    let title = move || {
        let path = path();
        if path.is_empty() {
            return name();
        }

        format!("{}/{path}", name())
    };

    let name_untracked = move || params.read_untracked().get("name").unwrap_or_default();
    let version_untracked = move || {
        params
            .read_untracked()
            .get("version")
            .unwrap_or("latest".into())
    };

    let (pending, set_pending) = RwSignal::new(false).split();

    let tree = Resource::new(
        move || (name(), version()),
        async |(name, version)| get_file_tree(name, version, "".into()).await,
    );

    view! {
        <Title text=title />
        <div class="filetree" class:pending=pending>
            <Transition
                fallback=|| "Loading...".into_view()
                set_pending
            >
                {move || {
                    tree.get().map(|res| match res {
                        Ok(tree) => tree.view(name_untracked(), version_untracked(), "".into()).into_any(),
                        Err(e) => view! {
                            <div class="error">
                                { match e {
                                    ServerFnError::ServerError(msg) => msg.into_view().into_any(),
                                    _ => "Failed to load".into_view().into_any(),
                                }}
                            </div>
                        }.into_any()
                    })
                }}
            </Transition>
        </div>
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum FileTreeNode {
    File(String),
    Dir(String, DirData),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum DirData {
    Loaded(Vec<FileTreeNode>),
    Empty,
    /// Indicates that the node limit during filetree generation was reached
    /// so this directory's children need to be loaded lazily when the user tries to open it
    Lazy,
}

impl DirData {
    fn should_be_link(&self) -> bool {
        matches!(self, Self::Lazy)
    }
}

fn join_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        return segment.to_string();
    }

    [base, segment].join("/")
}

impl FileTreeNode {
    fn view(&self, name: String, version: String, path: String) -> impl IntoView {
        match self {
            Self::File(filename) => {
                let path = join_path(&path, filename);
                let filename = filename.clone();

                view! {
                    <A href=path>{filename}</A>
                }
                .into_any()
            }
            Self::Dir(dirname, data) => {
                let path = join_path(&path, dirname);
                let dirname = dirname.clone();

                let current_path = use_location().pathname.read_untracked();
                let node_path = format!("/mod/{name}/{version}/{path}",);
                let node_path = node_path.strip_suffix('/').unwrap_or(&node_path);
                let is_open = current_path.starts_with(node_path);

                let (open, set_open) = RwSignal::new(is_open).split();
                let (loaded, set_loaded) = RwSignal::new(false).split();

                // TODO: turn this into isomorphic, requires some further investigation to resolve hydration issues
                //       with lazy directories that are open on page load due to the path matching
                Effect::new(move |_| {
                    if open.get() && !loaded.get() {
                        set_loaded.set(true);
                    }
                });

                let name = name.clone();
                let version = version.clone();
                let data = data.clone();
                let link = data.should_be_link() && !is_open;

                let click = move |ev: MouseEvent| {
                    ev.prevent_default();
                    set_open.update(|s| *s = !*s);
                };

                view! {
                    <details open=open>
                        <summary>
                            {if link {
                                view! {
                                    <A href={path.clone()} on:click=click>{dirname}</A>
                                }.into_any()
                            } else {
                                view! {
                                    <FakeA href={path.clone()} on:click=click>{dirname}</FakeA>
                                }.into_any()
                            }}
                        </summary>
                        {move || match &data {
                            DirData::Loaded(nodes) => nodes.iter().map(|node| node.view(name.clone(), version.clone(), path.clone())).collect_view().into_any(),
                            DirData::Lazy => if loaded.get() {
                                view! {
                                    <LazyDirInner name=name.clone() version=version.clone() path=path.clone()/>
                                }.into_any()
                            } else {
                                ().into_view().into_any()
                            },
                            DirData::Empty => view!{ <EmptyDirInner /> }.into_any(),
                        }}
                    </details>
                }
                .into_any()
            }
        }
    }
}

#[component]
fn FakeA<Chil>(href: String, children: TypedChildren<Chil>) -> impl IntoView
where
    Chil: IntoView + Send + 'static,
{
    let path = use_location().pathname;
    let path = move || {
        let p = path.read();
        let parts = p
            .strip_prefix("/mod/")?
            .split('/')
            .skip(2)
            .collect::<Box<[_]>>();
        Some(parts.join("/"))
    };

    let is_active = move || path().unwrap_or_default().starts_with(&href);
    let children = children.into_inner()();

    view! {
        <span aria-current=move || if is_active() { Some("page") } else { None }>{children}</span>
    }
}

#[component]
fn LazyDirInner(name: String, version: String, path: String) -> impl IntoView {
    let n = name.clone();
    let v = version.clone();
    let p = path.clone();
    let tree = Resource::new(
        || (),
        move |_| get_file_tree(n.clone(), v.clone(), p.clone()),
    );

    Suspend::new(async move {
        match tree.await {
            Ok(FileTreeNode::Dir(_, DirData::Loaded(nodes))) => nodes
                .iter()
                .map(|node| node.view(name.clone(), version.clone(), path.clone()))
                .collect_view()
                .into_any(),
            Ok(FileTreeNode::Dir(_, DirData::Empty)) => view! { <EmptyDirInner /> }.into_any(),
            _ => "Failed to load".into_view().into_any(),
        }
    })
}

#[component]
fn EmptyDirInner() -> impl IntoView {
    view! {
        <div class="empty">
            "empty"
        </div>
    }
}

#[server(prefix = "/api/sec")]
async fn get_file_tree(
    name: String,
    version: String,
    path: String,
) -> Result<FileTreeNode, ServerFnError> {
    use std::path::{Component, PathBuf};

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
            "Mod license does not allow exploring it via this tool".into(),
        ));
    }

    let requested_path = path.clone();
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

    let mut node_count = 0;
    match read_dir_to_nodes(path, &mut node_count).await {
        Err(e) => Err(ServerFnError::ServerError(format!(
            "Failed to read directory: {e}"
        ))),
        Ok(nodes) if nodes.is_empty() => Ok(FileTreeNode::Dir(requested_path, DirData::Empty)),
        Ok(nodes) => Ok(FileTreeNode::Dir(requested_path, DirData::Loaded(nodes))),
    }
}

#[cfg(feature = "ssr")]
async fn read_dir_to_nodes<P: AsRef<std::path::Path>>(
    path: P,
    node_count: &mut usize,
) -> std::io::Result<Vec<FileTreeNode>> {
    const NODE_LAZY_THRESHOLD: usize = 50;

    let mut nodes = Vec::new();
    let mut entries = tokio::fs::read_dir(path).await?;

    let mut files = Vec::new();
    let mut dirs = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let kind = entry.file_type().await?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if kind.is_file() {
            files.push(name);
        } else if kind.is_dir() {
            dirs.push((name, path));
        } else {
            continue;
        }

        *node_count += 1;
    }

    dirs.sort_by_key(|(n, _)| n.to_lowercase());

    for (name, path) in dirs {
        if *node_count >= NODE_LAZY_THRESHOLD || name.starts_with('.') {
            nodes.push(FileTreeNode::Dir(name, DirData::Lazy));
        } else {
            let children = Box::pin(read_dir_to_nodes(path, node_count)).await?;

            if children.is_empty() {
                nodes.push(FileTreeNode::Dir(name, DirData::Empty));
            } else {
                nodes.push(FileTreeNode::Dir(name, DirData::Loaded(children)));
            }
        }
    }

    files.sort_by_cached_key(|n| n.to_lowercase());
    for name in files {
        nodes.push(FileTreeNode::File(name));
    }

    Ok(nodes)
}
