use serenity::builder::CreateApplicationCommands;

use crate::client::slash_commands::UserCommand;

pub trait MarkovCommandBuilder {
    fn create_markov_commands(&mut self) -> &mut CreateApplicationCommands;
}

impl MarkovCommandBuilder for CreateApplicationCommands {
    fn create_markov_commands(&mut self) -> &mut CreateApplicationCommands {
        self
		.create_application_command(|command| {
			command.name(UserCommand::stopsavingmymessages).description(
				"Blacklist yourself if you don't want me to save and learn from your messages",
			)
		})
		.create_application_command(|command| {
			command
			.name(UserCommand::stopsavingmessagesserver)
			.description("Blacklist this server if you don't want me to save and learn from the messages sent in this server")
			.dm_permission(false)
			.default_member_permissions(serenity::model::Permissions::ADMINISTRATOR)
		})
		.create_application_command(|command| {
			command.name(UserCommand::continuesavingmymessages).description(
				"Remove yourself from the blacklist if you want me to save and learn from your messages",
			)
		})
    }
}
