use markov_strings::ImportExport;
use regex::Regex;
use serenity::{client::Context, model::channel::Message};

use crate::*;

pub const MARKOV_EXPORT_PATH: &str = "data/markov data/markov export.json";
pub const MARKOV_DATA_SET_PATH: &str = "data/markov data/markov data set.txt";
pub const BLACKLISTED_CHANNELS_PATH: &str = "data/markov data/blacklisted channels.json";
pub const BLACKLISTED_USERS_PATH: &str = "data/markov data/blacklisted users.json";

pub async fn should_add_message_to_markov_file(msg: &Message, ctx: &Context) {
    if let Some(_) = msg.channel_id.to_channel(&ctx.http).await.unwrap().guild() {
        {
            let markov_blacklisted_users_lock = get_markov_blacklisted_users_lock(ctx).await;
            let markov_blacklisted_channels_lock = get_markov_blacklisted_channels_lock(ctx).await;

            if !markov_blacklisted_channels_lock
                .read()
                .await
                .contains(&msg.channel_id.0)
                && !markov_blacklisted_users_lock
                    .read()
                    .await
                    .contains(&msg.author.id.0)
                && !msg.mentions_me(&ctx.http).await.unwrap()
                && msg.content.split(' ').collect::<Vec<&str>>().len() >= 5
            {
                let re = Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#).unwrap();
                let mut str = re.replace_all(&msg.content, "").into_owned();
                while str.ends_with(' ') {
                    str.pop();
                }
                let filtered_message = filter_message_for_markov_file(str, msg);
                //msg.reply(&ctx.http, &filtered_message).await.unwrap();
                append_to_markov_file(filtered_message);
            }
        }
    }
}
/// If the message filter changes it's helpful to call this function when the bot starts so the filtering is consistent across the file.
#[allow(dead_code)]
fn clean_markov_file(msg: Message) {
    let file = fs::read_to_string(MARKOV_DATA_SET_PATH).unwrap();
    let messages = file
        .split("\n\n")
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    fs::write(MARKOV_DATA_SET_PATH, "").unwrap();
    for message in messages {
        append_to_markov_file(filter_message_for_markov_file(message, &msg))
    }
}

pub async fn send_markov_text(ctx: &Context, msg: &Message) {
    let markov_lock = get_markov_chain_lock(ctx).await;

    let markov_chain = markov_lock.read().await;

    match markov_chain.generate() {
        Ok(markov_result) => {
            msg.channel_id
                .say(&ctx.http, &markov_result.text)
                .await
                .unwrap();
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Try again later.")
                .await
                .unwrap();
        }
    };
}

pub fn append_to_markov_file(str: String) -> () {
    if !str.is_empty() && str.split(' ').collect::<Vec<&str>>().len() >= 5 {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(MARKOV_DATA_SET_PATH)
            .unwrap();

        if let Err(e) = writeln!(file, "{}\n", str) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
}
#[allow(dead_code)]
pub fn export_to_markov_file(export: ImportExport) {
    fs::write(MARKOV_EXPORT_PATH, serde_json::to_string(&export).unwrap()).unwrap();
}

fn import_chain_from_file() -> Vec<InputData> {
    let text_from_file =
        fs::read_to_string(create_file_if_missing(MARKOV_DATA_SET_PATH, "")).unwrap();
    let text_array = text_from_file.split("\n\n");
    let mut input_data: Vec<InputData> = Vec::new();
    for message in text_array {
        let input = InputData {
            text: message.to_string(),
            meta: None,
        };
        input_data.push(input);
    }
    return input_data;
}

pub fn init_markov_debug() -> Markov {
    let mut markov: Markov = serde_json::from_str(
        &fs::read_to_string(create_file_if_missing(
            MARKOV_EXPORT_PATH,
            &serde_json::to_string(&Markov::new().export()).unwrap(),
        ))
        .unwrap(),
    )
    .unwrap();
    markov.set_max_tries(200);
    markov.set_filter(|r| {
        if r.text.split(' ').collect::<Vec<&str>>().len() >= 5 && r.score >= 5 && r.refs.len() >= 2
        {
            return true;
        }
        return false;
    });
    return markov;
}

pub fn init_markov() -> Markov {
    let mut markov_chain = Markov::new();
    markov_chain.set_state_size(2).unwrap();
    markov_chain.set_max_tries(200);
    markov_chain.set_filter(|r| {
        if r.text.split(' ').collect::<Vec<&str>>().len() >= 5 && r.score >= 5 && r.refs.len() >= 2
        {
            return true;
        }
        return false;
    });
    markov_chain.add_to_corpus(import_chain_from_file());
    markov_chain
}

fn filter_message_for_markov_file(str: String, msg: &Message) -> String {
    let mut filtered_message = str.clone();
    //THIS IS GONNA BE A PAIN IN THE ASS
    let user_regex = Regex::new(r"<@!?(\d+)>").unwrap();

    let mut regexes_to_replace_with_whitespace: Vec<Regex> = Vec::new();
    regexes_to_replace_with_whitespace.push(Regex::new(r"<:?(\w+:)(\d+)>").unwrap()); //emote regex
    regexes_to_replace_with_whitespace.push(Regex::new(r"<a:?(\w+:)(\d+)>").unwrap()); //animated emote regex
    regexes_to_replace_with_whitespace
        .push(Regex::new(r#"[,.!"\#$()=?*<>{}\[\]\\\|Łł@*;:+~ˇ^˘°˛`´˝]"#).unwrap()); //non alphanumeric regex
    regexes_to_replace_with_whitespace.push(Regex::new(r"^(\d{18})$").unwrap()); //remaining numbers from users regex
    regexes_to_replace_with_whitespace.push(Regex::new(r"\n").unwrap()); //line feed regex
    regexes_to_replace_with_whitespace.push(Regex::new(r"[ ]{3}|[ ]{2}").unwrap()); //double and triple whitespace regex

    let upper_case_regex = Regex::new(r"[A-Z][a-z0-9_-]{1,}").unwrap();

    loop {
        let mut number_of_matches: u16 = 0;

        while user_regex.is_match(&filtered_message) {
            number_of_matches += 1;

            filtered_message = user_regex
                .replace(&filtered_message, |caps: &Captures| {
                    let user_id = &caps[0][2..20];
                    let user = &msg
                        .mentions
                        .iter()
                        .find(|&user| user.id.0.to_string() == user_id)
                        .unwrap()
                        .name;
                    return " ".to_owned() + user + " ";
                })
                .into_owned();
        }
        for regex in &regexes_to_replace_with_whitespace {
            while regex.is_match(&filtered_message) {
                number_of_matches += 1;
                filtered_message = regex.replace_all(&filtered_message, " ").into_owned();
            }
        }
        while upper_case_regex.is_match(&filtered_message) {
            number_of_matches += 1;
            filtered_message = upper_case_regex
                .replace(&filtered_message, |caps: &Captures| {
                    return caps[0].to_lowercase();
                })
                .into_owned();
        }
        if number_of_matches == 0 {
            break;
        }
    }

    return filtered_message.trim().to_string();
}

pub async fn add_or_remove_user_from_blacklist(user: &User, ctx: &Context) -> String {
    let blacklisted_users_lock = get_markov_blacklisted_users_lock(ctx).await;
    let mut blacklisted_users = blacklisted_users_lock.write().await;

    match !blacklisted_users.contains(&user.id.0) {
        true => {
            {
                blacklisted_users.insert(user.id.0);
            }
            match fs::write(
                BLACKLISTED_USERS_PATH,
                serde_json::to_string(&*blacklisted_users).unwrap(),
            ) {
                Ok(_) => {
                    let message =
                        "Added ".to_owned() + &user.name + " to the list of blacklisted users";
                    return message;
                }
                Err(_) => {
                    return "Couldn't add the user to the file".to_string();
                }
            };
        }
        false => {
            {
                blacklisted_users.remove(&user.id.0);
            }
            match fs::write(
                BLACKLISTED_USERS_PATH,
                serde_json::to_string(&*blacklisted_users).unwrap(),
            ) {
                Ok(_) => {
                    let message =
                        "Removed ".to_owned() + &user.name + " from the list of blacklisted users";
                    return message;
                }
                Err(_) => {
                    return "Couldn't remove the user from the file".to_string();
                }
            };
        }
    };
}

pub async fn blacklist_user_command(msg: &Message, ctx: &Context) -> String {
    let user = match get_first_mentioned_user(msg) {
        Some(returned_user) => returned_user,
        None => {
            return "Please specify a user".to_string();
        }
    };
    add_or_remove_user_from_blacklist(user, ctx).await
}

pub async fn blacklisted_command(ctx: &Context) -> String {
    let mut blacklisted_users = Vec::new();
    let blacklisted_users_lock = get_markov_blacklisted_users_lock(ctx).await;

    for user_id in blacklisted_users_lock.read().await.iter() {
        blacklisted_users.push(ctx.http.get_user(user_id.clone()).await.unwrap().name);
    }

    if blacklisted_users.len() == 0 {
        return "Currently there are no blacklisted users".to_string();
    }

    let mut message = String::from("Blacklisted users: ");
    for user_name in blacklisted_users {
        message += &(user_name + ", ");
    }
    message.pop();
    message.pop();
    return message;
}
