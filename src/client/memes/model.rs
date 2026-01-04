use std::{collections::HashSet, sync::Arc, time::Duration};

use serenity::prelude::{TypeMap, TypeMapKey};
use tokio::{sync::RwLock, time::timeout};

use crate::client::memes::dal::get_meme_folders;

#[tracing::instrument(skip(data))]
pub fn init_memes_data(data: &mut tokio::sync::RwLockWriteGuard<serenity::prelude::TypeMap>) {
    let meme_folders = get_meme_folders()
        .into_iter()
        .map(|f| f.file_name().to_string_lossy().to_string())
        .collect::<HashSet<_>>();

    data.insert::<MemeFolders>(Arc::new(RwLock::new(MemeFolders {
        folders: meme_folders,
    })));
}

#[derive(Clone, Default, Debug)]
pub struct MemeFolders {
    pub folders: HashSet<String>,
}

impl TypeMapKey for MemeFolders {
    type Value = Arc<RwLock<MemeFolders>>;
}

pub async fn get_meme_folders_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<MemeFolders>> {
    timeout(Duration::from_secs(30), data.read())
        .await
        .unwrap()
        .get::<MemeFolders>()
        .expect("expected MemeFolders in TypeMap")
        .clone()
}
