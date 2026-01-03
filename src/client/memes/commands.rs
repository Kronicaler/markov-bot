use serenity::all::{
    CommandOptionType, CommandType, CreateCommand, CreateCommandOption, InstallationContext,
    InteractionContext,
};

pub fn create_memes_commands() -> Vec<CreateCommand> {
    let upload_meme_command = CreateCommand::new("Upload meme")
        .add_integration_type(InstallationContext::User)
        .add_integration_type(InstallationContext::Guild)
        .add_context(InteractionContext::Guild)
        .add_context(InteractionContext::PrivateChannel)
        .kind(CommandType::Message);

    let post_meme_command = CreateCommand::new("meme").add_option(
        CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "post",
            "Post a meme from the desired tag",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "tag",
            "Select a tag",
        ))
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "ordered",
                "wether to send a random meme or send from oldest to newest",
            )
            .required(false),
        ),
    );

    vec![upload_meme_command, post_meme_command]
}
