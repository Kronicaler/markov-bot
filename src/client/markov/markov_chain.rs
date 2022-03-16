use regex::{Captures, Regex};
use serenity::model::channel::Message;
use std::u64;

const MIN_NUM_OF_WORDS: usize = 5;

/// Filters a message so it can be inserted into the Markov data set.
///
/// Removes links, User IDs, emotes, animated emotes, non alphanumeric characters, line feeds, extra whitespace, and role IDs.
///
/// Replaces uppercase letters with their lowercase variants.
pub fn filter_message_for_markov_file(msg: &Message) -> Option<String> {
    let re =
    Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
    .expect("Invalid regular expression");

    let mut str = re.replace_all(&msg.content, "").into_owned();
    while str.ends_with(' ') {
        str.pop();
    }

    let mut filtered_message = str;

    let user_regex = Regex::new(r"<@!?(\d+)>").expect("Invalid regular expression");

    let regexes_to_replace_with_whitespace: Vec<Regex> = vec![
        Regex::new(r"<:?(\w+:)(\d+)>").expect("Invalid regular expression"), //emote regex
        Regex::new(r"<a:?(\w+:)(\d+)>").expect("Invalid regular expression"), //animated emote regex
        Regex::new(r#"[,.!"\#$()=?*<>{}\[\]\\\|Łł@*;:+~ˇ^˘°˛`´˝]"#)
            .expect("Invalid regular expression"), //non alphanumeric regex
        Regex::new(r"^(\d{18})$").expect("Invalid regular expression"), //remaining numbers from users regex
        Regex::new(r"\n").expect("Invalid regular expression"),         //line feed regex
        Regex::new(r"[ ]{3}|[ ]{2}").expect("Invalid regular expression"), //double and triple whitespace regex
        Regex::new(r"<@&(\d+)>").expect("Invalid regular expression"),     // role regex
    ];

    let upper_case_regex = Regex::new(r"[A-Z][a-z0-9_-]{1,}").expect("Invalid regular expression");

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
                    let user_id = user_id.parse::<u64>().expect("Couldn't parse user id");
                    let user = &msg
                        .mentions
                        .iter()
                        .find(|&user| user.id.0 == user_id)
                        .expect("Couldn't find user mention in message")
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

    if filtered_message.trim().split(' ').count() < MIN_NUM_OF_WORDS {
        return None;
    }

    return Some(filtered_message.trim().to_owned());
}
/// Filters a string so it can be inserted into the Markov data set.
///
/// Removes links, User IDs, emotes, animated emotes, non alphanumeric characters, line feeds, extra whitespace, and role IDs.
///
/// Replaces uppercase letters with their lowercase variants.
pub fn filter_string_for_markov_file(msg: &str) -> String {
    let re =
    Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
    .expect("Invalid regular expression");

    let mut str = re.replace_all(msg, "").into_owned();
    while str.ends_with(' ') {
        str.pop();
    }

    let mut filtered_message = str;

    let regexes_to_replace_with_whitespace: Vec<Regex> = vec![
        Regex::new(r"<:?(\w+:)(\d+)>").expect("Invalid regular expression"), //emote regex
        Regex::new(r"<a:?(\w+:)(\d+)>").expect("Invalid regular expression"), //animated emote regex
        Regex::new(r#"[,.!"\#$()=?*<>{}\[\]\\\|Łł@*;:+~ˇ^˘°˛`´˝]"#)
            .expect("Invalid regular expression"), //non alphanumeric regex
        Regex::new(r"^(\d{18})$").expect("Invalid regular expression"), //remaining numbers from users regex
        Regex::new(r"\n").expect("Invalid regular expression"),         //line feed regex
        Regex::new(r"[ ]{3}|[ ]{2}").expect("Invalid regular expression"), //double and triple whitespace regex
        Regex::new(r"<@&(\d+)>").expect("Invalid regular expression"),     // role regex
    ];

    let upper_case_regex = Regex::new(r"[A-Z][a-z0-9_-]{1,}").expect("Invalid regular expression");

    loop {
        let mut number_of_matches: u16 = 0;

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

    return filtered_message.trim().to_owned();
}
