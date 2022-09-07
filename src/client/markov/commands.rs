use serenity::{builder::CreateApplicationCommand, model::Permissions};

use crate::client::slash_commands::UserCommand;

pub fn create_markov_commands() -> Vec<CreateApplicationCommand> {
    return vec![
		CreateApplicationCommand::new(UserCommand::stopsavingmymessages.to_string())
		.description("Blacklist yourself if you don't want me to save and learn from your messages"),
		CreateApplicationCommand::new(UserCommand::stopsavingmessagesserver.to_string())
		.description("Blacklist this server if you don't want me to save and learn from the messages sent in this server")
		.dm_permission(false)
		.default_member_permissions(Permissions::ADMINISTRATOR),
		CreateApplicationCommand::new(UserCommand::stopsavingmessageschannel.to_string())
		.description("Blacklist this channel if you don't want me to save and learn from the messages sent in this channel")
		.dm_permission(false)
		.default_member_permissions(Permissions::ADMINISTRATOR),
		CreateApplicationCommand::new(UserCommand::continuesavingmymessages.to_string()).description(
			"Remove yourself from the blacklist if you want me to save and learn from your messages",
		)
	];
}
