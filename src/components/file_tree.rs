use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::{
    components::{A, Outlet},
    hooks::use_params_map,
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
        <h2>"File tree"</h2>
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
        <Outlet/>
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
                    <a class="dir" on:click=move |_| set_open.update(|s| *s = !*s)>{dirname.clone()} " (lazy)"</a>
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

#[server]
async fn get_file_tree(
    name: String,
    version: String,
    path: String,
) -> Result<FileTreeNode, ServerFnError> {
    let path = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect::<Vec<_>>()
        .join("/");

    let test_tree = FileTreeNode::Dir(
        "".into(),
        vec![
            FileTreeNode::File("file1.txt".into()),
            FileTreeNode::Dir(
                "subdir".into(),
                vec![FileTreeNode::File("file2.txt".into())],
            ),
            FileTreeNode::EmptyDir("empty_subdir".into()),
            FileTreeNode::LazyDir("lazy_subdir".into()),
        ],
    );

    let lazy_subdir_contents = FileTreeNode::Dir(
        "lazy_subdir".into(),
        vec![
            FileTreeNode::File("i_got_lazy.txt".into()),
            FileTreeNode::File("me_too.txt".into()),
            FileTreeNode::Dir("nested".into(), vec![FileTreeNode::File("deep.txt".into())]),
        ],
    );

    match path.as_str() {
        "lazy_subdir" => Ok(lazy_subdir_contents),
        _ => Ok(test_tree),
    }
}
