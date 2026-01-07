use serenity::all::{CommandData, CommandDataOptionValue};

pub trait GetOptionFromCommand {
    fn get_string(&self, name: &str) -> String;
    fn get_optional_bool(&self, name: &str) -> Option<bool>;
}

impl GetOptionFromCommand for CommandData {
    fn get_string(&self, name: &str) -> String {
        match self.options.first().cloned().unwrap().value {
            CommandDataOptionValue::SubCommand(command_data_options) => command_data_options
                .iter()
                .find(|o| o.name == name)
                .unwrap()
                .value
                .as_str()
                .unwrap()
                .to_string(),
            _ => panic!("unknown option"),
        }
    }

    fn get_optional_bool(&self, name: &str) -> Option<bool> {
        match self.options.first().cloned().unwrap().value {
            CommandDataOptionValue::SubCommand(command_data_options) => Some(
                command_data_options
                    .iter()
                    .find(|o| o.name == name)?
                    .value
                    .as_bool()
                    .unwrap(),
            ),
            _ => panic!("unknown option"),
        }
    }
}
