use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::Arc,
};

use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use tokio::sync::RwLock;

mod attribution_cache;
use attribution_cache::AttributionCache;

type DedupString = Arc<str>;

pub struct ModsState {
    pub all_mods: RwLock<BTreeSet<DedupString>>,
    pub mod_versions: RwLock<HashMap<DedupString, Box<[DedupString]>>>,
    pub attribution_cache: AttributionCache,
}

impl ModsState {
    pub async fn get_version(&self, name: &str, version: &str) -> Option<DedupString> {
        let mod_versions = self.mod_versions.read().await;
        let mod_versions = mod_versions.get(name)?;

        match version {
            "latest" => mod_versions.first().cloned(),
            v => {
                let v = v.into();
                mod_versions.iter().find(|&ver| ver == &v).cloned()
            }
        }
    }
}

pub struct Highlighter {
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
}

#[derive(Clone)]
pub struct AppState {
    pub mods_folder: PathBuf,
    pub mods_state: Arc<ModsState>,
    pub highlighter: Arc<Highlighter>,
}

impl AppState {
    pub fn new(mods_folder: PathBuf) -> Self {
        use crate::components::DEFAULT_THEME;
        let mut theme_set = ThemeSet::load_defaults();
        if !theme_set.themes.contains_key(DEFAULT_THEME) {
            panic!("Default theme {DEFAULT_THEME} not found in theme set");
        }

        theme_set
            .themes
            .retain(|name, _| name == DEFAULT_THEME || name.to_lowercase().contains("dark"));

        Self {
            mods_folder,
            mods_state: Arc::new(ModsState {
                all_mods: RwLock::new(BTreeSet::new()),
                mod_versions: RwLock::new(HashMap::new()),
                attribution_cache: AttributionCache::default(),
            }),
            highlighter: Arc::new(Highlighter {
                syntax_set: SyntaxSet::load_defaults_newlines(),
                theme_set,
            }),
        }
    }
}
