use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::Arc,
};

use tokio::sync::RwLock;

type DedupString = Arc<str>;

#[derive(Clone)]
pub struct ModsState {
    pub all_mods: Arc<RwLock<BTreeSet<DedupString>>>,
    pub mod_versions: Arc<RwLock<HashMap<DedupString, Box<[DedupString]>>>>,
}

#[derive(Clone)]
pub struct AppState {
    pub mods_folder: PathBuf,
    pub mods_state: ModsState,
}

impl AppState {
    pub fn new(mods_folder: PathBuf) -> Self {
        Self {
            mods_folder,
            mods_state: ModsState {
                all_mods: Arc::new(RwLock::new(BTreeSet::new())),
                mod_versions: Arc::new(RwLock::new(HashMap::new())),
            },
        }
    }
}
