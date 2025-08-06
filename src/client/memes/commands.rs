use itertools::Itertools;
use serenity::all::{CreateCommand, InstallationContext, InteractionContext};

use crate::client::memes::get_meme_folders;

pub fn create_memes_commands() -> Vec<CreateCommand> {
    let folders = get_meme_folders();

    folders.into_iter()
        .map(|f| {
            let folder_name = f.file_name().to_string_lossy().to_string();

            CreateCommand::new(folder_name.clone())
                .description(format!("send a {folder_name} meme"))
                .add_integration_type(InstallationContext::User)
                .add_integration_type(InstallationContext::Guild)
                .add_context(InteractionContext::Guild)
        })
        .collect_vec()
}