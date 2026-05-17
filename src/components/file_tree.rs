use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::{components::A, hooks::use_params_map};
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
                    match tree.get() {
                        None => "No data".into_view().into_any(),
                        Some(Err(_)) => "Failed to load".into_view().into_any(),
                        Some(Ok(tree)) => tree.view(name_untracked(), version_untracked(), "".into()).into_any(),
                    }
                }}
            </Transition>
        </div>
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum FileTreeNode {
    File(String),
    Dir(String, Vec<Self>),
    EmptyDir(String),
    LazyDir(String),
}

impl FileTreeNode {
    fn view(&self, name: String, version: String, path: String) -> impl IntoView {
        fn join_path(base: &str, segment: &str) -> String {
            [base, segment]
                .join("/")
                .trim_start_matches('/')
                .to_string()
        }

        match self {
            Self::File(filename) => {
                let path = join_path(&path, filename);
                let filename = filename.clone();

                view! {
                    <A href={path}>{filename}</A>
                }
                .into_any()
            }
            Self::EmptyDir(dirname) => {
                let dirname = dirname.clone();

                view! {
                    <div class="dir empty">{dirname}</div>
                }
                .into_any()
            }
            Self::Dir(dirname, nodes) => {
                let path = join_path(&path, dirname);
                let dirname = dirname.clone();
                let nodes = nodes.clone();

                let (open, set_open) = RwSignal::new(dirname.is_empty()).split();
                let (loaded, set_loaded) = RwSignal::new(false).split();

                view! {
                    <a class="dir" on:click=move |_| set_open.update(|s| *s = !*s)>{dirname}</a>
                    <div class="dir_children" class:open=open>
                        {move || if open.get() && !loaded.get() {
                            set_loaded.set(true);
                        }}
                        {move || if loaded.get() {

                            nodes.iter()
                                    .map(|node| node.view(name.clone(), version.clone(), path.clone()))
                                    .collect_view()
                                    .into_any()
                        } else {
                            ().into_view().into_any()
                        }}
                    </div>
                }
                .into_any()
            }
            Self::LazyDir(dirname) => {
                let path = join_path(&path, dirname);
                let dirname = dirname.clone();

                let (open, set_open) = RwSignal::new(false).split();
                let (loaded, set_loaded) = RwSignal::new(false).split();

                let name = name.clone();
                let version = version.clone();

                view! {
                    <a class="dir" on:click=move |_| set_open.update(|s| *s = !*s)>{dirname.clone()}</a>
                    <div class="dir_children" class:open=open>
                        {move || if open.get() && !loaded.get() {
                            set_loaded.set(true);
                        }}
                        {move || if loaded.get() {
                            let name = name.clone();
                            let version = version.clone();
                            let path = path.clone();

                            view! {
                                <LazyDirInner name=name version=version path=path/>
                            }.into_any()
                        } else {
                            ().into_view().into_any()
                        }}
                    </div>
                }.into_any()
            }
        }
    }

    #[inline]
    const fn name(&self) -> &str {
        match self {
            Self::File(n) | Self::Dir(n, _) | Self::EmptyDir(n) | Self::LazyDir(n) => n.as_str(),
        }
    }
}

#[component]
fn LazyDirInner(name: String, version: String, path: String) -> impl IntoView {
    let n = name.clone();
    let v = version.clone();
    let p = path.clone();
    let tree = LocalResource::new(move || get_file_tree(n.clone(), v.clone(), p.clone()));

    Suspend::new(async move {
        match tree.await {
            Ok(FileTreeNode::Dir(_, nodes)) => nodes
                .iter()
                .map(|node| node.view(name.clone(), version.clone(), path.clone()))
                .collect_view()
                .into_any(),
            _ => "Failed to load".into_view().into_any(),
        }
    })
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
        Ok(nodes) => Ok(FileTreeNode::Dir(requested_path, nodes)),
    }
}

#[cfg(feature = "ssr")]
async fn read_dir_to_nodes<P: AsRef<std::path::Path>>(
    path: P,
    node_count: &mut usize,
) -> std::io::Result<Vec<FileTreeNode>> {
    const NODE_LAZY_THRESHOLD: usize = 25;

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
        if *node_count >= NODE_LAZY_THRESHOLD {
            nodes.push(FileTreeNode::LazyDir(name));
        } else {
            let mut children = Box::pin(read_dir_to_nodes(path, node_count)).await?;
            children.sort_by_key(|c| c.name().to_string());

            if children.is_empty() {
                nodes.push(FileTreeNode::EmptyDir(name));
            } else {
                nodes.push(FileTreeNode::Dir(name, children));
            }
        }
    }

    files.sort_by_cached_key(|n| n.to_lowercase());
    for name in files {
        nodes.push(FileTreeNode::File(name));
    }

    Ok(nodes)
}
