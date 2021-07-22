use crate::*;
use druid::Target;
use markov_strings::Markov;
use regex::{Captures, Regex};
use serenity::{
    client::Context,
    model::{channel::Message, prelude::User},
};
use std::{fs, u64, usize};

pub async fn should_add_message_to_markov_file(msg: &Message, ctx: &Context) {
    if msg
        .channel_id
        .to_channel(&ctx.http)
        .await
        .unwrap()
        .guild()
        .is_some()
    {
        {
            let markov_blacklisted_users_lock = get_markov_blacklisted_users_lock(&ctx.data).await;
            let markov_blacklisted_channels_lock = get_markov_blacklisted_channels_lock(&ctx.data).await;

            if !markov_blacklisted_channels_lock
                .read()
                .await
                .contains(&msg.channel_id.0)
                && !markov_blacklisted_users_lock
                    .read()
                    .await
                    .contains(&msg.author.id.0)
                && !msg.mentions_me(&ctx.http).await.unwrap()
                && msg.content.split(' ').count() >= 5
            {
                let re = Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#).unwrap();
                let mut str = re.replace_all(&msg.content, "").into_owned();
                while str.ends_with(' ') {
                    str.pop();
                }
                let filtered_message = filter_message_for_markov_file(str, msg);
                //msg.reply(&ctx.http, &filtered_message).await.unwrap();
                append_to_markov_file(&filtered_message);
                let message_count_lock = get_message_count_lock(&ctx.data).await;
                let mut message_count = message_count_lock.write().await;
                *message_count = message_count.checked_add(1).unwrap();
                let front_channel_lock = get_front_channel_lock(&ctx.data).await;
                let front_channel = front_channel_lock.read().await;
                front_channel
                    .event_sink
                    .submit_command(
                        SET_MESSAGE_COUNT,
                        *message_count,
                        Target::Widget(ID_MESSAGE_COUNT),
                    )
                    .unwrap();
            }
        }
    }
}

pub async fn send_markov_text(ctx: &Context, msg: &Message) {
    let markov_lock = get_markov_chain_lock(&ctx.data).await;

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
        if r.text.split(' ').count() >= 5 && r.score >= 5 && r.refs.len() >= 2 {
            return true;
        }
        false
    });
    markov
}

pub fn init_markov() -> (Markov, usize) {
    let mut markov_chain = Markov::new();
    markov_chain.set_state_size(2).unwrap();
    markov_chain.set_max_tries(200);
    markov_chain.set_filter(|r| {
        if r.text.split(' ').count() >= 5 && r.score >= 5 && r.refs.len() >= 2 {
            return true;
        }
        false
    });
    let input_data = import_chain_from_file();
    let num_of_messages = input_data.len();
    markov_chain.add_to_corpus(input_data);
    (markov_chain, num_of_messages)
}

pub fn filter_message_for_markov_file(str: String, msg: &Message) -> String {
    let mut filtered_message = str;
    //THIS IS GONNA BE A PAIN IN THE ASS
    let user_regex = Regex::new(r"<@!?(\d+)>").unwrap();

    let regexes_to_replace_with_whitespace: Vec<Regex> = vec![
        Regex::new(r"<:?(\w+:)(\d+)>").unwrap(),  //emote regex
        Regex::new(r"<a:?(\w+:)(\d+)>").unwrap(), //animated emote regex
        Regex::new(r#"[,.!"\#$()=?*<>{}\[\]\\\|Łł@*;:+~ˇ^˘°˛`´˝]"#).unwrap(), //non alphanumeric regex
        Regex::new(r"^(\d{18})$").unwrap(), //remaining numbers from users regex
        Regex::new(r"\n").unwrap(),         //line feed regex
        Regex::new(r"[ ]{3}|[ ]{2}").unwrap(), //double and triple whitespace regex
        Regex::new(r"<@&(\d+)>").unwrap(),  // role regex
    ];

    let upper_case_regex = Regex::new(r"[A-Z][a-z0-9_-]{1,}").unwrap();

    loop {
        let mut number_of_matches: u16 = 0;

        while user_regex.is_match(&filtered_message) {
            number_of_matches += 1;

            filtered_message = user_regex
                .replace(&filtered_message, |caps: &Captures| {
                    let mut user_id = String::new();

                    for char in caps[0].chars() {
                        if char.is_digit(10) {
                            user_id += &char.to_string();
                        }
                    }
                    let user_id = user_id.parse::<u64>().unwrap();
                    let user = &msg
                        .mentions
                        .iter()
                        .find(|&user| user.id.0 == user_id)
                        .unwrap()
                        .name;
                    " ".to_owned() + user + " "
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
                .replace(&filtered_message, |caps: &Captures| caps[0].to_lowercase())
                .into_owned();
        }
        if number_of_matches == 0 {
            break;
        }
    }

    return filtered_message.trim().to_string();
}

pub async fn blacklist_user_command(msg: &Message, ctx: &Context) -> String {
    let user = match get_first_mentioned_user(msg) {
        Some(returned_user) => returned_user,
        None => {
            return "Please specify a user".to_string();
        }
    };
    add_or_remove_user_from_markov_blacklist(user, ctx).await
}

pub async fn blacklisted_command(ctx: &Context) -> String {
    let mut blacklisted_users = Vec::new();
    let blacklisted_users_lock = get_markov_blacklisted_users_lock(&ctx.data).await;

    for user_id in blacklisted_users_lock.read().await.iter() {
        blacklisted_users.push(ctx.http.get_user(*user_id).await.unwrap().name);
    }

    if blacklisted_users.is_empty() {
        return "Currently there are no blacklisted users".to_string();
    }

    let mut message = String::from("Blacklisted users: ");
    for user_name in blacklisted_users {
        message += &(user_name + ", ");
    }
    message.pop();
    message.pop();
    message
}

pub async fn add_or_remove_user_from_markov_blacklist(user: &User, ctx: &Context) -> String {
    let blacklisted_users_lock = get_markov_blacklisted_users_lock(&ctx.data).await;
    let mut blacklisted_users = blacklisted_users_lock.write().await;

    if blacklisted_users.contains(&user.id.0) {
        blacklisted_users.remove(&user.id.0);
        match save_markov_blacklisted_users(&*blacklisted_users) {
            Ok(_) => "Removed ".to_owned() + &user.name + " from the list of blacklisted users",
            Err(_) => "Couldn't remove the user from the file".to_string(),
        }
    } else {
        blacklisted_users.insert(user.id.0);
        match save_markov_blacklisted_users(&*blacklisted_users) {
            Ok(_) => "Added ".to_owned() + &user.name + " to the list of blacklisted users",
            Err(_) => "Couldn't add the user to the file".to_string(),
        }
    }
}
